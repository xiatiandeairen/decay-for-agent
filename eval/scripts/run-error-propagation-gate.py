#!/usr/bin/env python3
from __future__ import annotations

import argparse
import importlib.util
import json
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


JUDGEMENT_RANK = {
    "none": 0,
    "needs_more_context_before_review_claim": 1,
    "review_worthy_behavior_change": 2,
}


def load_pack(path: Path) -> dict[str, Any]:
    if tomllib is None:
        raise RuntimeError("tomllib is required")
    return tomllib.loads(path.read_text())


def load_bundle(case_id: str, cases_root: Path) -> tuple[str, dict[str, Any] | None, str | None]:
    raw = cases_root / case_id / "raw"
    required = [raw / "decay.stdout", raw / "diff.patch"]
    if any(not path.exists() for path in required):
        return "missing_raw", None, None
    try:
        pr_json = json.loads((raw / "pr.json").read_text()) if (raw / "pr.json").exists() else None
        bundle = build_risk_wrapper.build_bundle(
            case_id,
            (raw / "decay.stdout").read_text(),
            (raw / "diff.patch").read_text(),
            pr_json,
        )
        return "ok", bundle, None
    except Exception as err:  # pragma: no cover - exercised via tests
        return "invalid_raw", None, f"{type(err).__name__}: {err}"


def evaluate(pack_path: Path, cases_root: Path) -> dict[str, Any]:
    pack = load_pack(pack_path)
    rows: list[dict[str, Any]] = []

    tp = fp = fn = tn = 0
    review_tp = review_fp = review_fn = 0
    boundary_expected = boundary_hits = 0
    duplicate_wrapper_case_count = 0
    raw_valid_cases = 0
    missed_positive_cases: list[str] = []
    duplicate_cases: list[str] = []
    overclaim_cases: list[str] = []
    invalid_raw_cases: list[str] = []

    for case in pack["cases"]:
        case_id = case["id"]
        raw_status, bundle, raw_error = load_bundle(case_id, cases_root)
        row = {
            "case_id": case_id,
            "role": case["role"],
            "expected_detection": case["expected_detection"],
            "expected_judgement": case["expected_judgement"],
            "max_wrapper_count": case["max_wrapper_count"],
            "raw_status": raw_status,
            "raw_error": raw_error,
            "wrapper_count": 0,
            "actual_detection": "suppress",
            "actual_judgement": "none",
            "duplicate_wrapper": False,
            "judgement_match": False,
            "best_wrapper_id": None,
        }

        if raw_status != "ok" or bundle is None:
            if raw_status != "ok":
                invalid_raw_cases.append(case_id)
            rows.append(row)
            if case["expected_detection"] == "detect":
                fn += 1
                missed_positive_cases.append(case_id)
                if case["expected_judgement"] == "review_worthy_behavior_change":
                    review_fn += 1
                if case["expected_judgement"] == "needs_more_context_before_review_claim":
                    boundary_expected += 1
            else:
                tn += 1
            continue

        raw_valid_cases += 1
        wrappers = [
            wrapper
            for wrapper in bundle.get("wrappers", [])
            if wrapper.get("wrapper_type") == "ErrorPropagationWrapper"
        ]
        best_wrapper = max(wrappers, key=wrapper_rank, default=None)
        actual_judgement = "none"
        if best_wrapper is not None:
            actual_judgement = (
                best_wrapper.get("honest_review", {})
                .get("overall", {})
                .get("judgement", "none")
            )

        row["wrapper_count"] = len(wrappers)
        row["actual_detection"] = "detect" if wrappers else "suppress"
        row["actual_judgement"] = actual_judgement
        row["best_wrapper_id"] = best_wrapper.get("wrapper_id") if best_wrapper else None
        row["duplicate_wrapper"] = len(wrappers) > case["max_wrapper_count"]
        row["judgement_match"] = actual_judgement == case["expected_judgement"]

        if row["duplicate_wrapper"]:
            duplicate_wrapper_case_count += 1
            duplicate_cases.append(case_id)

        expected_detect = case["expected_detection"] == "detect"
        actual_detect = bool(wrappers)
        if expected_detect and actual_detect:
            tp += 1
        elif expected_detect and not actual_detect:
            fn += 1
            missed_positive_cases.append(case_id)
        elif not expected_detect and actual_detect:
            fp += 1
        else:
            tn += 1

        expected_review = case["expected_judgement"] == "review_worthy_behavior_change"
        actual_review = actual_judgement == "review_worthy_behavior_change"
        if expected_review and actual_review:
            review_tp += 1
        elif expected_review and not actual_review:
            review_fn += 1
        elif not expected_review and actual_review:
            review_fp += 1
            overclaim_cases.append(case_id)

        if case["expected_judgement"] == "needs_more_context_before_review_claim":
            boundary_expected += 1
            if actual_judgement == "needs_more_context_before_review_claim":
                boundary_hits += 1

        rows.append(row)

    metrics = {
        "total_cases": len(pack["cases"]),
        "raw_valid_cases": raw_valid_cases,
        "raw_invalid_cases": len(invalid_raw_cases),
        "positives": sum(1 for case in pack["cases"] if case["expected_detection"] == "detect"),
        "negatives": sum(1 for case in pack["cases"] if case["expected_detection"] == "suppress"),
        "tp": tp,
        "fp": fp,
        "fn": fn,
        "tn": tn,
        "detection_precision": ratio(tp, tp + fp),
        "detection_recall": ratio(tp, tp + fn),
        "review_worthy_tp": review_tp,
        "review_worthy_fp": review_fp,
        "review_worthy_fn": review_fn,
        "review_worthy_precision": ratio(review_tp, review_tp + review_fp),
        "boundary_expected": boundary_expected,
        "boundary_hits": boundary_hits,
        "boundary_honesty_rate": ratio(boundary_hits, boundary_expected),
        "duplicate_wrapper_case_count": duplicate_wrapper_case_count,
        "raw_contract_valid_rate": ratio(raw_valid_cases, len(pack["cases"])),
    }
    gates = gate_result(metrics, pack.get("gates", {}))
    return {
        "version": "error-propagation-gate-result.v1",
        "pack": str(pack_path),
        "risk_item": pack.get("risk_item", "error_propagation_change"),
        "metrics": metrics,
        "gates": gates,
        "missed_positive_cases": sorted(set(missed_positive_cases)),
        "duplicate_cases": sorted(set(duplicate_cases)),
        "overclaim_cases": sorted(set(overclaim_cases)),
        "invalid_raw_cases": sorted(set(invalid_raw_cases)),
        "rows": rows,
    }


def wrapper_rank(wrapper: dict[str, Any]) -> tuple[int, int, int]:
    review = wrapper.get("honest_review", {})
    judgement = review.get("overall", {}).get("judgement", "none")
    mechanism = review.get("dimensions", {}).get("mechanism", {}).get("status", "unknown")
    mechanism_rank = {
        "contradicted": 0,
        "unknown": 1,
        "weak": 2,
        "partial": 3,
        "confirmed": 4,
    }.get(mechanism, 0)
    return (
        JUDGEMENT_RANK.get(judgement, 0),
        mechanism_rank,
        -len(wrapper.get("primary_evidence", [])),
    )


def ratio(numerator: int, denominator: int) -> float | None:
    if denominator == 0:
        return None
    return round(numerator / denominator, 4)


def gate_result(metrics: dict[str, Any], thresholds: dict[str, Any]) -> dict[str, Any]:
    checks = {
        "raw_contract_valid_rate": compare_min(
            metrics["raw_contract_valid_rate"], thresholds.get("raw_contract_valid_rate_min")
        ),
        "detection_precision": compare_min(
            metrics["detection_precision"], thresholds.get("detection_precision_min")
        ),
        "detection_recall": compare_min(
            metrics["detection_recall"], thresholds.get("detection_recall_min")
        ),
        "review_worthy_precision": compare_min(
            metrics["review_worthy_precision"], thresholds.get("review_worthy_precision_min")
        ),
        "boundary_honesty_rate": compare_min(
            metrics["boundary_honesty_rate"], thresholds.get("boundary_honesty_rate_min")
        ),
        "duplicate_wrapper_case_count": compare_max(
            metrics["duplicate_wrapper_case_count"],
            thresholds.get("duplicate_wrapper_case_count_max"),
        ),
    }
    return {
        "pass": all(check["pass"] for check in checks.values()),
        "checks": checks,
    }


def compare_min(actual: float | None, minimum: float | None) -> dict[str, Any]:
    if minimum is None:
        return {"pass": True, "actual": actual, "expected": None}
    if actual is None:
        return {"pass": False, "actual": actual, "expected": minimum}
    return {"pass": actual >= minimum, "actual": actual, "expected": minimum}


def compare_max(actual: int, maximum: int | None) -> dict[str, Any]:
    if maximum is None:
        return {"pass": True, "actual": actual, "expected": None}
    return {"pass": actual <= maximum, "actual": actual, "expected": maximum}


def write_markdown(result: dict[str, Any], out: Path) -> None:
    metrics = result["metrics"]
    lines = [
        "# Error Propagation Gate",
        "",
        "## Summary",
        "",
        f"- total_cases: {metrics['total_cases']}",
        f"- raw_contract_valid_rate: {metrics['raw_contract_valid_rate']}",
        f"- detection_precision: {metrics['detection_precision']}",
        f"- detection_recall: {metrics['detection_recall']}",
        f"- review_worthy_precision: {metrics['review_worthy_precision']}",
        f"- boundary_honesty_rate: {metrics['boundary_honesty_rate']}",
        f"- duplicate_wrapper_case_count: {metrics['duplicate_wrapper_case_count']}",
        f"- gate_pass: {str(result['gates']['pass']).lower()}",
        "",
        "## Gate Checks",
        "",
        "| Check | Actual | Expected | Pass |",
        "|-------|--------|----------|------|",
    ]
    for name, check in result["gates"]["checks"].items():
        lines.append(
            f"| {name} | {check['actual']} | {check['expected']} | {str(check['pass']).lower()} |"
        )
    lines.extend(
        [
            "",
            "## Case Rows",
            "",
            "| Case | Role | Raw | Wrapper Count | Actual Detection | Actual Judgement | Duplicate |",
            "|------|------|-----|---------------|------------------|------------------|-----------|",
        ]
    )
    for row in result["rows"]:
        lines.append(
            "| {case_id} | {role} | {raw_status} | {wrapper_count} | {actual_detection} | {actual_judgement} | {duplicate} |".format(
                case_id=row["case_id"],
                role=row["role"],
                raw_status=row["raw_status"],
                wrapper_count=row["wrapper_count"],
                actual_detection=row["actual_detection"],
                actual_judgement=row["actual_judgement"],
                duplicate=str(row["duplicate_wrapper"]).lower(),
            )
        )
    lines.extend(["", "## Missed Positive Cases", ""])
    lines.extend([f"- {case_id}" for case_id in result["missed_positive_cases"]] or ["- none"])
    lines.extend(["", "## Overclaim Cases", ""])
    lines.extend([f"- {case_id}" for case_id in result["overclaim_cases"]] or ["- none"])
    lines.extend(["", "## Invalid Raw Cases", ""])
    lines.extend([f"- {case_id}" for case_id in result["invalid_raw_cases"]] or ["- none"])
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--pack",
        default=str(ROOT / "eval" / "risk-signals" / "error-propagation-gate-v1.toml"),
    )
    parser.add_argument("--cases-root", default=str(ROOT / "eval" / "cases"))
    parser.add_argument("--json-out")
    parser.add_argument("--md-out")
    args = parser.parse_args()

    result = evaluate(Path(args.pack), Path(args.cases_root))
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
