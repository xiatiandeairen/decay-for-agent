#!/usr/bin/env python3
from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None  # type: ignore[assignment]


ROOT = Path(__file__).resolve().parents[2]
BUILD_WRAPPER_SCRIPT = ROOT / "eval" / "scripts" / "build-risk-wrapper.py"
VALIDATOR_SCRIPT = ROOT / "eval" / "scripts" / "validate-honest-review-analysis.py"


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


build_risk_wrapper = load_module("build_risk_wrapper", BUILD_WRAPPER_SCRIPT)
validate_honest_review = load_module("validate_honest_review", VALIDATOR_SCRIPT)


def load_toml(path: Path) -> dict[str, Any]:
    if tomllib is None:
        raise RuntimeError("tomllib is required")
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_cases(corpus: Path) -> list[dict[str, Any]]:
    cases = load_toml(corpus).get("cases", [])
    if not isinstance(cases, list):
        raise ValueError(f"{corpus}: cases must be a list")
    return cases


def build_case_bundle(cases_root: Path, case_id: str) -> tuple[str, dict[str, Any] | None]:
    raw = cases_root / case_id / "raw"
    stdout = raw / "decay.stdout"
    diff = raw / "diff.patch"
    if not stdout.exists() or not diff.exists():
        return "missing_raw", None
    pr_json = json.loads((raw / "pr.json").read_text()) if (raw / "pr.json").exists() else None
    bundle = build_risk_wrapper.build_bundle(case_id, stdout.read_text(), diff.read_text(), pr_json)
    return "ok", bundle


def analyze_ai_output(raw_dir: Path, bundle: dict[str, Any]) -> tuple[str, list[str]]:
    path = raw_dir / "honest-review-analysis.json"
    if not path.exists():
        return "ai_missing", []
    analysis = json.loads(path.read_text())
    errors = validate_honest_review.validate(bundle, analysis)
    if errors:
        return "ai_invalid", errors
    return "ai_valid", []


def replay(corpus: Path, cases_root: Path) -> dict[str, Any]:
    cases = load_cases(corpus)
    rows: list[dict[str, Any]] = []
    risk_item_counts: Counter[str] = Counter()
    wrapper_type_counts: Counter[str] = Counter()
    dimension_counts: Counter[str] = Counter()
    ai_counts: Counter[str] = Counter()

    for case in cases:
        case_id = str(case["id"])
        status, bundle = build_case_bundle(cases_root, case_id)
        row: dict[str, Any] = {
            "case_id": case_id,
            "repo": case.get("repo", ""),
            "pr": case.get("pr", ""),
            "raw_status": status,
            "wrapper_count": 0,
            "honest_review_count": 0,
            "risk_items": [],
            "dimension_statuses": {},
            "ai_status": "not_applicable",
            "validator_errors": [],
        }
        if bundle is None:
            rows.append(row)
            continue

        wrappers = bundle.get("wrappers", [])
        honest_wrappers = [wrapper for wrapper in wrappers if "honest_review" in wrapper]
        row["wrapper_count"] = len(wrappers)
        row["honest_review_count"] = len(honest_wrappers)
        if honest_wrappers:
            ai_status, errors = analyze_ai_output(cases_root / case_id / "raw", bundle)
            row["ai_status"] = ai_status
            row["validator_errors"] = errors
            ai_counts[ai_status] += 1
        else:
            ai_counts["not_applicable"] += 1

        for wrapper in honest_wrappers:
            wrapper_type_counts[wrapper.get("wrapper_type", "unknown")] += 1
            review = wrapper["honest_review"]
            risk_item = review.get("risk_item", "unknown")
            risk_item_counts[risk_item] += 1
            row["risk_items"].append(risk_item)
            for name, dimension in review.get("dimensions", {}).items():
                key = f"{name}:{dimension.get('status', 'unknown')}"
                dimension_counts[key] += 1
                row["dimension_statuses"][key] = row["dimension_statuses"].get(key, 0) + 1
        rows.append(row)

    metrics = {
        "target_cases": len(cases),
        "cases_with_raw": sum(1 for row in rows if row["raw_status"] == "ok"),
        "cases_with_wrappers": sum(1 for row in rows if row["wrapper_count"] > 0),
        "cases_with_honest_review": sum(1 for row in rows if row["honest_review_count"] > 0),
        "total_wrappers": sum(row["wrapper_count"] for row in rows),
        "total_honest_reviews": sum(row["honest_review_count"] for row in rows),
        "risk_item_counts": dict(sorted(risk_item_counts.items())),
        "wrapper_type_counts": dict(sorted(wrapper_type_counts.items())),
        "dimension_counts": dict(sorted(dimension_counts.items())),
        "ai_counts": dict(sorted(ai_counts.items())),
    }
    return {
        "version": "honest-review-replay.v1",
        "corpus": str(corpus),
        "cases_root": str(cases_root),
        "metrics": metrics,
        "rows": rows,
    }


def write_markdown(result: dict[str, Any], out: Path) -> None:
    metrics = result["metrics"]
    lines = [
        "# Honest Review Frozen Replay",
        "",
        "This report replays deterministic risk wrappers and honest_review over the frozen corpus. It is not a product-effect benchmark by itself.",
        "",
        "## Metrics",
        "",
        f"- target_cases: {metrics['target_cases']}",
        f"- cases_with_raw: {metrics['cases_with_raw']}",
        f"- cases_with_wrappers: {metrics['cases_with_wrappers']}",
        f"- cases_with_honest_review: {metrics['cases_with_honest_review']}",
        f"- total_wrappers: {metrics['total_wrappers']}",
        f"- total_honest_reviews: {metrics['total_honest_reviews']}",
        f"- risk_item_counts: `{json.dumps(metrics['risk_item_counts'], sort_keys=True)}`",
        f"- wrapper_type_counts: `{json.dumps(metrics['wrapper_type_counts'], sort_keys=True)}`",
        f"- dimension_counts: `{json.dumps(metrics['dimension_counts'], sort_keys=True)}`",
        f"- ai_counts: `{json.dumps(metrics['ai_counts'], sort_keys=True)}`",
        "",
        "## Cases",
        "",
        "| Case | Wrappers | Honest Reviews | Risk Items | AI Status |",
        "|------|---------:|---------------:|------------|-----------|",
    ]
    for row in result["rows"]:
        lines.append(
            "| {case_id} | {wrapper_count} | {honest_review_count} | {risk_items} | {ai_status} |".format(
                case_id=row["case_id"],
                wrapper_count=row["wrapper_count"],
                honest_review_count=row["honest_review_count"],
                risk_items=", ".join(row["risk_items"]),
                ai_status=row["ai_status"],
            )
        )
    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "- `cases_with_honest_review` measures current deterministic coverage only.",
            "- `ai_valid` counts only cases that already have `honest-review-analysis.json` and pass validator.",
            "- Missing wrappers or missing AI output must not be interpreted as product success.",
        ]
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=ROOT / "eval" / "corpus" / "risk-signal-frozen-v1.toml")
    parser.add_argument("--cases-root", type=Path, default=ROOT / "eval" / "cases")
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--md-out", type=Path)
    args = parser.parse_args(argv or sys.argv[1:])

    result = replay(args.corpus, args.cases_root)
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    if args.md_out:
        write_markdown(result, args.md_out)
    if not args.json_out and not args.md_out:
        print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
