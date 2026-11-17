#!/usr/bin/env python3
"""Convert a judge run directory into head-to-head ai_baseline judgement TOMLs.

The judge harness (`run-judge.py`) writes per-case `normalized.json` containing
`judgement_drafts.ai_baseline`. `summarize-head-to-head.py` reads the
`[ai_baseline]` section from `eval/head-to-head/judgements/<case-id>.toml`. This
script is the deterministic bridge: for every case in the judge run that produced
a valid normalized judgement, it emits the judgement TOML with status="complete".

A case is skipped (no TOML written) when:
  - its normalized.json is missing or marks status "judge-failed", or
  - the matching ai-baseline output is missing / "ai-baseline-failed" (no
    baseline to grade, so the AI baseline is not judged for that case).

The judge's subjective `judgement_drafts.ai_baseline` numbers are authoritative
(same grading basis as the frozen fact_only / semantic_filtered judgements). The
mechanical recompute flag, if any, is appended to notes for traceability.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


_REQUIRED_AB = (
    "top3_total",
    "top3_useful",
    "actionable",
    "missed_high_risk_count",
    "weak_only_top_risk_count",
)


def _parse_raw_stdout(stdout: str) -> dict | None:
    """Best-effort parse of a judge raw stdout into the judge-output dict.

    Used only as a fallback when the strict normalizer rejected the response
    for free-text length overflow (notes / expected_behavior exceed the frozen
    schema caps). The decision metrics live in small integer fields that are
    never affected by prose length, so recovering them from raw is faithful to
    the judge's actual grading. Returns None when the JSON is not parseable
    (e.g. a genuinely truncated response)."""
    s = stdout.strip()
    s = re.sub(r"^```(json)?\s*", "", s)
    s = re.sub(r"\s*```\s*$", "", s)
    i, j = s.find("{"), s.rfind("}")
    if i < 0 or j < 0:
        return None
    try:
        data = json.loads(s[i : j + 1])
    except Exception:
        return None
    drafts = data.get("judgement_drafts")
    if not isinstance(drafts, dict) or not isinstance(drafts.get("ai_baseline"), dict):
        return None
    if not all(k in drafts["ai_baseline"] for k in _REQUIRED_AB):
        return None
    return data


def _truncate(value: str, limit: int) -> str:
    return value if len(value) <= limit else value[: limit - 1].rstrip() + "…"


REPO_ROOT = Path(__file__).resolve().parents[2]
EVAL_ROOT = REPO_ROOT / "eval"
HEAD_TO_HEAD_DIR = EVAL_ROOT / "head-to-head"


def _toml_str(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ")
    return f'"{escaped}"'


def _baseline_is_valid(ai_norm_path: Path) -> bool:
    if not ai_norm_path.exists():
        return False
    data = json.loads(ai_norm_path.read_text())
    return data.get("status") != "ai-baseline-failed" and bool(data.get("top_risks"))


def render_toml(
    case_id: str,
    judge_norm: dict,
    mech: dict | None,
    judged_at: str,
    judge_label: str,
    recovered_from_raw: bool,
) -> str:
    ab = judge_norm["judgement_drafts"]["ai_baseline"]
    gt = judge_norm.get("ground_truth_draft", {})
    summary = _truncate(str(gt.get("expected_behavior", "")), 400)
    notes = _truncate(str(ab.get("notes", "")), 300)
    if recovered_from_raw:
        notes = (notes + " [recovered-from-raw: judge response exceeded schema "
                 "free-text caps; metrics unaffected]").strip()
    if mech:
        flags = [f for f in mech.get("flags", []) if f.startswith("ai_baseline")]
        if flags:
            notes = (notes + " [mechanical-flag: " + "; ".join(flags) + "]").strip()
    lines = [
        '# Generated from a judge run by judgements-from-judge-run.py.',
        '# ai_baseline section grades the frozen AI baseline output against ground truth.',
        "",
        'version = "ai-baseline-judgement.v1"',
        f"case_id = {_toml_str(case_id)}",
        'status = "complete"',
        f"judge = {_toml_str(judge_label)}",
        f"judged_at = {_toml_str(judged_at)}",
        f"ground_truth_summary = {_toml_str(summary)}",
        "",
        "[ai_baseline]",
        f"top3_total = {int(ab.get('top3_total', 0))}",
        f"top3_useful = {int(ab.get('top3_useful', 0))}",
        f"actionable = {'true' if ab.get('actionable') else 'false'}",
        f"missed_high_risk_count = {int(ab.get('missed_high_risk_count', 0))}",
        f"weak_only_top_risk_count = {int(ab.get('weak_only_top_risk_count', 0))}",
        f"notes = {_toml_str(notes)}",
        "",
    ]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--judge-run-dir", type=Path, required=True)
    parser.add_argument("--ai-baseline-run-dir", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, default=HEAD_TO_HEAD_DIR / "judgements")
    parser.add_argument("--judged-at", required=True, help="ISO date, e.g. 2026-06-08")
    parser.add_argument("--judge-label", default="claude-opus-4-7")
    args = parser.parse_args(argv or sys.argv[1:])

    args.out_dir.mkdir(parents=True, exist_ok=True)
    written, skipped = [], []
    for case_dir in sorted(p for p in args.judge_run_dir.iterdir() if p.is_dir()):
        case_id = case_dir.name
        norm_path = case_dir / "normalized.json"
        if not norm_path.exists():
            skipped.append((case_id, "no normalized.json"))
            continue
        norm = json.loads(norm_path.read_text())
        recovered_from_raw = False
        if norm.get("status") == "judge-failed" or "judgement_drafts" not in norm:
            raw_path = case_dir / "raw.json"
            recovered = None
            if raw_path.exists():
                raw = json.loads(raw_path.read_text())
                attempts = raw.get("attempts", [])
                if attempts:
                    recovered = _parse_raw_stdout(attempts[-1].get("stdout", ""))
            if recovered is None:
                skipped.append((case_id, "judge-failed (unrecoverable from raw)"))
                continue
            norm = recovered
            recovered_from_raw = True
        if not _baseline_is_valid(args.ai_baseline_run_dir / case_id / "normalized.json"):
            skipped.append((case_id, "ai-baseline invalid/missing"))
            continue
        mech_path = case_dir / "mechanical.json"
        mech = json.loads(mech_path.read_text()) if mech_path.exists() else None
        toml = render_toml(case_id, norm, mech, args.judged_at, args.judge_label, recovered_from_raw)
        (args.out_dir / f"{case_id}.toml").write_text(toml)
        written.append(case_id)

    print(f"wrote {len(written)} judgement TOMLs to {args.out_dir}")
    for c in written:
        print(f"  ok    {c}")
    for c, why in skipped:
        print(f"  skip  {c}: {why}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
