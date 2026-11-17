#!/usr/bin/env python3
"""Independently recompute `top3_useful` from a judge's ground_truth_draft.

The judge gives a subjective per-mode `top3_useful`. This script recomputes the
same number from the structured ground truth and the three pipelines' top-3
risk lists, applying a mechanical rule:

    top3_useful = count(top3 item where
        subject_file ∈ ground_truth.subject_files
        AND risk_signal ∈ ground_truth.primary_signal_ids
    )

The AI subjective value and the mechanical value are both kept. A `disagree`
flag is emitted whenever the two differ — those cases are what a human
reviewer should focus on, since either the ground truth is incomplete or the
AI judgement drifted from the rule.

Input shape per mode (whether fact-only / semantic-filtered / ai-baseline):
the pipeline's top-3 list, where each item has at least a `subject` (path
optionally followed by `:fn`) and a `risk_signal` enum value. AI baseline
normalized.json fits this exactly; fact / semantic decay outputs need to be
extracted upstream into the same shape — for round 1 we accept the same
schema as ai-baseline and treat fact / semantic outputs as `None` (skipped)
when not provided.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


@dataclass(frozen=True)
class TopRisk:
    subject: str
    risk_signal: str


@dataclass(frozen=True)
class RecomputeResult:
    ai_subjective: int
    mechanical: int

    @property
    def agree(self) -> bool:
        return self.ai_subjective == self.mechanical


def _subject_file(subject: str) -> str:
    return subject.split(":", 1)[0]


def mechanical_top3_useful(
    top_risks: Iterable[dict[str, Any]],
    subject_files: Iterable[str],
    primary_signal_ids: Iterable[str],
) -> int:
    """Count top-3 items that double-match the ground truth.

    `subject_files` and `primary_signal_ids` are sets-of-strings produced by
    the judge as part of `ground_truth_draft`. The rule is intentionally
    strict: both file and signal must match. Pure file matches without a
    matching signal do not count (those are weaker / supporting).
    """
    file_set = {f for f in subject_files if f}
    signal_set = {s for s in primary_signal_ids if s}
    if not file_set or not signal_set:
        return 0
    count = 0
    for item in top_risks:
        if not isinstance(item, dict):
            continue
        subj = item.get("subject", "")
        sig = item.get("risk_signal", "")
        if not isinstance(subj, str) or not isinstance(sig, str):
            continue
        if _subject_file(subj) in file_set and sig in signal_set:
            count += 1
    return count


def recompute(
    *,
    judge_output: dict[str, Any],
    top_risks_per_mode: dict[str, list[dict[str, Any]] | None],
) -> dict[str, Any]:
    """Cross-check the judge's subjective top3_useful against the mechanical rule.

    `top_risks_per_mode` maps mode name → top-3 list (or None when the pipeline
    output is not yet collected for that mode). Modes with None are emitted
    with `top3_useful_mechanical: null` and excluded from the agree check.
    """
    gt = judge_output["ground_truth_draft"]
    drafts = judge_output["judgement_drafts"]
    subject_files = gt.get("subject_files", []) or []
    primary_signals = gt.get("primary_signal_ids", []) or []

    per_mode: dict[str, dict[str, Any]] = {}
    flags: list[str] = []
    for mode in ("fact_only", "semantic_filtered", "ai_baseline"):
        ai_subj = int(drafts.get(mode, {}).get("top3_useful", 0))
        top3 = top_risks_per_mode.get(mode)
        if top3 is None:
            per_mode[mode] = {
                "ai_subjective": ai_subj,
                "mechanical": None,
                "agree": None,
                "reason": "top3 list not provided",
            }
            continue
        mech = mechanical_top3_useful(top3, subject_files, primary_signals)
        agree = mech == ai_subj
        per_mode[mode] = {
            "ai_subjective": ai_subj,
            "mechanical": mech,
            "agree": agree,
        }
        if not agree:
            flags.append(f"{mode}: ai={ai_subj}, mechanical={mech}")

    return {
        "case_id": judge_output["case_id"],
        "per_mode": per_mode,
        "flags": flags,
    }


def _load_ai_baseline_top_risks(normalized_path: Path) -> list[dict[str, Any]] | None:
    if not normalized_path.exists():
        return None
    data = json.loads(normalized_path.read_text())
    if data.get("status") == "ai-baseline-failed":
        return None
    return data.get("top_risks", [])


def _maybe_load(path: Path | None) -> list[dict[str, Any]] | None:
    if path is None or not path.exists():
        return None
    data = json.loads(path.read_text())
    # Accept either a wrapped {top_risks: [...]} or a bare list.
    if isinstance(data, list):
        return data
    return data.get("top_risks", [])


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--judge", type=Path, required=True, help="normalized judge output JSON")
    parser.add_argument("--ai-baseline", type=Path, help="ai-baseline normalized.json")
    parser.add_argument("--fact", type=Path, help="fact pipeline top-3 JSON (optional)")
    parser.add_argument("--semantic", type=Path, help="semantic pipeline top-3 JSON (optional)")
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    judge_output = json.loads(args.judge.read_text())
    top_risks = {
        "ai_baseline": _load_ai_baseline_top_risks(args.ai_baseline) if args.ai_baseline else None,
        "fact_only": _maybe_load(args.fact),
        "semantic_filtered": _maybe_load(args.semantic),
    }
    result = recompute(judge_output=judge_output, top_risks_per_mode=top_risks)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
