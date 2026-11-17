#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CORPUS = ROOT / "corpus" / "risk-signal-frozen-v1.toml"
DEFAULT_OUT = ROOT / "benchmarks" / "frozen-v1"
RUN_PR_CASE = ROOT / "scripts" / "run-pr-case.sh"
RUN_SEMANTIC_CASE = ROOT / "scripts" / "run-semantic-case.sh"


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_cases(corpus: Path) -> list[dict[str, Any]]:
    data = load_toml(corpus)
    cases = data.get("cases", [])
    if not isinstance(cases, list) or not cases:
        raise ValueError(f"{corpus}: cases must be a non-empty list")
    return cases


def selected_cases(cases: list[dict[str, Any]], case_ids: list[str], limit: int | None) -> list[dict[str, Any]]:
    if case_ids:
        by_id = {case.get("id"): case for case in cases}
        missing = [case_id for case_id in case_ids if case_id not in by_id]
        if missing:
            raise ValueError(f"case not found in corpus: {', '.join(missing)}")
        cases = [by_id[case_id] for case_id in case_ids]
    if limit is not None:
        cases = cases[:limit]
    return cases


def toml_quote(value: str) -> str:
    return json.dumps(value)


def judgement_template(case: dict[str, Any]) -> str:
    case_id = str(case["id"])
    repo = str(case["repo"])
    pr = int(case["pr"])
    reason = str(case.get("reason", ""))
    lines = [
        "# Filled after reviewing fact-only decay output and semantic-filtered output.",
        "# Do not edit the frozen corpus to improve these scores.",
        "",
        'version = "risk-eval-judgement.v1"',
        f"case_id = {toml_quote(case_id)}",
        f"repo = {toml_quote(repo)}",
        f"pr = {pr}",
        'status = "pending"',
        'judge = ""',
        'judged_at = ""',
        f"ground_truth_summary = {toml_quote(reason)}",
        "",
        "[ground_truth]",
        'gate_role = ""',
        "high_value = false",
        "primary_signal_ids = []",
        'expected_behavior = ""',
        "forbidden_claims = []",
        "required_context = []",
        'notes = ""',
        "",
        "[fact_only]",
        "top3_total = 0",
        "top3_useful = 0",
        "actionable = false",
        "missed_high_risk_count = 0",
        "weak_only_top_risk_count = 0",
        'detection = ""',
        'value_verdict = ""',
        "evidence_valid = false",
        "overclaim = false",
        "forbidden_claim_breached = false",
        "unsupported_scope_silence = false",
        "next_check_useful = false",
        'notes = ""',
        "",
        "[semantic_filtered]",
        "top3_total = 0",
        "top3_useful = 0",
        "actionable = false",
        "missed_high_risk_count = 0",
        "weak_only_top_risk_count = 0",
        'detection = ""',
        'value_verdict = ""',
        "evidence_valid = false",
        "overclaim = false",
        "forbidden_claim_breached = false",
        "unsupported_scope_silence = false",
        "next_check_useful = false",
        'notes = ""',
        "",
        "[comparison]",
        "semantic_improved_ranking = false",
        "semantic_reduced_noise = false",
        "semantic_introduced_noise = false",
        'semantic_net_value = ""',
        'notes = ""',
        "",
        "[gate]",
        "ground_truth_complete = false",
        "forbidden_claims_checked = false",
        "holdout_eligible = false",
        'notes = ""',
        "",
    ]
    return "\n".join(lines)


def write_judgement_template(out_dir: Path, case: dict[str, Any], force: bool = False) -> Path:
    path = out_dir / "judgements" / f"{case['id']}.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    if force or not path.exists():
        path.write_text(judgement_template(case))
    return path


def command_for(mode: str, case_id: str) -> list[str]:
    if mode == "fact":
        return [str(RUN_PR_CASE), case_id]
    if mode == "semantic":
        return [str(RUN_SEMANTIC_CASE), case_id]
    raise ValueError(f"unsupported mode: {mode}")


def run_case(mode: str, case_id: str, corpus: Path, dry_run: bool) -> dict[str, Any]:
    command = command_for(mode, case_id)
    if dry_run:
        return {"mode": mode, "case_id": case_id, "command": command, "exit_code": None, "status": "dry_run"}

    env = os.environ.copy()
    env["DECAY_EVAL_CORPUS"] = str(corpus)
    completed = subprocess.run(command, cwd=ROOT.parent, env=env, text=True, capture_output=True)
    return {
        "mode": mode,
        "case_id": case_id,
        "command": command,
        "exit_code": completed.returncode,
        "status": "ok" if completed.returncode == 0 else "failed",
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def append_jsonl(path: Path, row: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as handle:
        handle.write(json.dumps(row, sort_keys=True) + "\n")


def write_manifest(out_dir: Path, corpus: Path, cases: list[dict[str, Any]], modes: list[str], dry_run: bool) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest = {
        "version": "frozen-benchmark-run.v1",
        "created_at": datetime.now(timezone.utc).isoformat(),
        "corpus": str(corpus),
        "case_count": len(cases),
        "modes": modes,
        "dry_run": dry_run,
        "case_ids": [case["id"] for case in cases],
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run the frozen-v1 benchmark corpus.")
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--case", action="append", default=[], dest="cases", help="case id to run; repeatable")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--mode", choices=["fact", "semantic", "both"], default="both")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--force-template", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    try:
        cases = selected_cases(load_cases(args.corpus), args.cases, args.limit)
    except ValueError as err:
        print(err, file=sys.stderr)
        return 2

    modes = ["fact", "semantic"] if args.mode == "both" else [args.mode]
    write_manifest(args.out, args.corpus, cases, modes, args.dry_run)
    run_log = args.out / "run-log.jsonl"
    if run_log.exists():
        run_log.unlink()

    for case in cases:
        write_judgement_template(args.out, case, force=args.force_template)

    for case in cases:
        case_id = str(case["id"])
        for mode in modes:
            result = run_case(mode, case_id, args.corpus, args.dry_run)
            append_jsonl(run_log, result)
            if result["status"] == "failed":
                print(f"{case_id} {mode} failed; see {run_log}", file=sys.stderr)
                return int(result["exit_code"] or 1)

    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
