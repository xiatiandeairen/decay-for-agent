#!/usr/bin/env python3
from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None  # type: ignore[assignment]


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "eval" / "scripts" / "build-risk-wrapper.py"
SPEC = importlib.util.spec_from_file_location("build_risk_wrapper", SCRIPT)
assert SPEC and SPEC.loader
build_risk_wrapper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_risk_wrapper)


def load_ground_truth(path: Path) -> dict[str, dict[str, Any]]:
    if not path.exists() or tomllib is None:
        return {}
    payload = tomllib.loads(path.read_text())
    out: dict[str, dict[str, Any]] = {}
    for case in payload.get("cases", []):
        case_id = case.get("id")
        if case_id:
            out[case_id] = case
    return out


def case_dirs(cases_root: Path) -> list[Path]:
    return sorted(path for path in cases_root.iterdir() if (path / "raw" / "decay.stdout").exists())


def load_case(case_dir: Path) -> dict[str, Any]:
    raw = case_dir / "raw"
    stdout = (raw / "decay.stdout").read_text()
    diff = (raw / "diff.patch").read_text() if (raw / "diff.patch").exists() else ""
    pr_json = json.loads((raw / "pr.json").read_text()) if (raw / "pr.json").exists() else None
    return build_risk_wrapper.build_bundle(case_dir.name, stdout, diff, pr_json)


def evaluate(cases_root: Path, ground_truth_path: Path) -> dict[str, Any]:
    ground_truth = load_ground_truth(ground_truth_path)
    rows = []
    gt_error_cases = []
    wrapper_cases = []
    review_worthy_cases = []
    overclaim_suspects = []

    for case_dir in case_dirs(cases_root):
        case_id = case_dir.name
        gt = ground_truth.get(case_id, {})
        gt_signals = set(gt.get("signal_ids", []))
        gt_error = "error_propagation" in gt_signals
        if gt_error:
            gt_error_cases.append(case_id)

        bundle = load_case(case_dir)
        wrappers = [
            wrapper
            for wrapper in bundle.get("wrappers", [])
            if wrapper.get("wrapper_type") == "ErrorPropagationWrapper"
        ]
        if wrappers:
            wrapper_cases.append(case_id)

        for wrapper in wrappers:
            review = wrapper.get("honest_review", {})
            dimensions = review.get("dimensions", {})
            overall = review.get("overall", {})
            row = {
                "case_id": case_id,
                "ground_truth_error_propagation": gt_error,
                "mechanism": dimensions.get("mechanism", {}).get("status", "missing"),
                "impact": dimensions.get("impact", {}).get("status", "missing"),
                "control": dimensions.get("control", {}).get("status", "missing"),
                "judgement": overall.get("judgement", "missing"),
                "non_claims": overall.get("non_claims", []),
                "ground_truth_summary": gt.get("ground_truth_summary", ""),
            }
            rows.append(row)
            if row["judgement"] == "review_worthy_behavior_change":
                review_worthy_cases.append(case_id)
            if row["mechanism"] == "confirmed" and not row["non_claims"]:
                overclaim_suspects.append(case_id)

    missed_gt = sorted(set(gt_error_cases) - set(wrapper_cases))
    return {
        "version": "error-propagation-honest-review-eval.v1",
        "cases_scanned": len(case_dirs(cases_root)),
        "ground_truth_error_propagation_cases": sorted(gt_error_cases),
        "wrapper_cases": sorted(wrapper_cases),
        "review_worthy_cases": sorted(set(review_worthy_cases)),
        "missed_ground_truth_cases": missed_gt,
        "overclaim_suspects": sorted(set(overclaim_suspects)),
        "rows": rows,
    }


def write_markdown(result: dict[str, Any], out: Path) -> None:
    lines = [
        "# Error Propagation Honest Review Eval",
        "",
        "## Summary",
        "",
        f"- cases_scanned: {result['cases_scanned']}",
        f"- ground_truth_error_propagation_cases: {len(result['ground_truth_error_propagation_cases'])}",
        f"- wrapper_cases: {len(result['wrapper_cases'])}",
        f"- review_worthy_cases: {len(result['review_worthy_cases'])}",
        f"- missed_ground_truth_cases: {len(result['missed_ground_truth_cases'])}",
        f"- overclaim_suspects: {len(result['overclaim_suspects'])}",
        "",
        "## Wrapper Rows",
        "",
        "| Case | GT Error Propagation | Mechanism | Impact | Control | Judgement |",
        "|------|----------------------|-----------|--------|---------|-----------|",
    ]
    for row in result["rows"]:
        lines.append(
            "| {case_id} | {gt} | {mechanism} | {impact} | {control} | {judgement} |".format(
                case_id=row["case_id"],
                gt=str(row["ground_truth_error_propagation"]).lower(),
                mechanism=row["mechanism"],
                impact=row["impact"],
                control=row["control"],
                judgement=row["judgement"],
            )
        )
    lines.extend(["", "## Missed Ground Truth Error Propagation Cases", ""])
    if result["missed_ground_truth_cases"]:
        for case_id in result["missed_ground_truth_cases"]:
            lines.append(f"- {case_id}")
    else:
        lines.append("- none")
    lines.extend(["", "## Interpretation", ""])
    lines.append(
        "This eval is a stability check for the honest review model, not proof of product value. "
        "A low wrapper count means the current detector has limited recall for error propagation ground truth."
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases-root", default=str(ROOT / "eval" / "cases"))
    parser.add_argument("--ground-truth", default=str(ROOT / "eval" / "risk-signals" / "eval-candidates.toml"))
    parser.add_argument("--json-out")
    parser.add_argument("--md-out")
    args = parser.parse_args()

    result = evaluate(Path(args.cases_root), Path(args.ground_truth))
    if args.json_out:
        Path(args.json_out).parent.mkdir(parents=True, exist_ok=True)
        Path(args.json_out).write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    if args.md_out:
        write_markdown(result, Path(args.md_out))
    if not args.json_out and not args.md_out:
        print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
