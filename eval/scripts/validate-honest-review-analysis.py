#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


STATUS_RANK = {
    "contradicted": 0,
    "unknown": 1,
    "weak": 2,
    "partial": 3,
    "confirmed": 4,
}


def wrapper_by_id(bundle: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        wrapper["wrapper_id"]: wrapper
        for wrapper in bundle.get("wrappers", [])
        if "honest_review" in wrapper
    }


def validate(bundle: dict[str, Any], analysis: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    wrappers = wrapper_by_id(bundle)
    if analysis.get("version") != "honest-review-analysis.v1":
        errors.append("version must be honest-review-analysis.v1")

    for idx, item in enumerate(analysis.get("items", []), start=1):
        wrapper_id = item.get("wrapper_id")
        wrapper = wrappers.get(wrapper_id)
        if wrapper is None:
            errors.append(f"item {idx}: unknown wrapper_id {wrapper_id!r}")
            continue
        review = wrapper["honest_review"]
        if item.get("risk_item") != review.get("risk_item"):
            errors.append(f"item {idx}: risk_item does not match wrapper honest_review")
        validate_dimensions(idx, review, item, errors)
        validate_non_claims(idx, review, item, errors)
        validate_uncertainties(idx, review, item, errors)
        validate_next_check(idx, review, item, errors)

    return errors


def validate_dimensions(
    idx: int,
    review: dict[str, Any],
    item: dict[str, Any],
    errors: list[str],
) -> None:
    expected = review.get("dimensions", {})
    actual = item.get("dimension_judgements", {})
    for name in ["mechanism", "impact", "control"]:
        expected_dim = expected.get(name, {})
        actual_dim = actual.get(name, {})
        expected_status = expected_dim.get("status")
        actual_status = actual_dim.get("status")
        if STATUS_RANK.get(actual_status, 99) > STATUS_RANK.get(expected_status, -1):
            errors.append(
                f"item {idx}: {name} status {actual_status!r} exceeds honest_review status {expected_status!r}"
            )
        observed_and_inferred = set(expected_dim.get("observed", []) + expected_dim.get("inferred", []))
        for claim in actual_dim.get("supported_by", []):
            if claim not in observed_and_inferred:
                errors.append(f"item {idx}: {name} supported_by is not in observed/inferred: {claim!r}")
        expected_unknown = set(expected_dim.get("unknown", []))
        actual_limits = set(actual_dim.get("limits", []))
        missing_limits = expected_unknown - actual_limits
        if missing_limits and expected_status != "confirmed":
            errors.append(f"item {idx}: {name} limits missing honest_review unknowns: {sorted(missing_limits)!r}")


def validate_non_claims(
    idx: int,
    review: dict[str, Any],
    item: dict[str, Any],
    errors: list[str],
) -> None:
    expected = set(review.get("overall", {}).get("non_claims", []))
    actual = set(item.get("non_claims", []))
    missing = expected - actual
    if missing:
        errors.append(f"item {idx}: non_claims missing: {sorted(missing)!r}")


def validate_uncertainties(
    idx: int,
    review: dict[str, Any],
    item: dict[str, Any],
    errors: list[str],
) -> None:
    unknowns = {
        unknown
        for dimension in review.get("dimensions", {}).values()
        for unknown in dimension.get("unknown", [])
    }
    uncertainties = set(item.get("uncertainties", []))
    if unknowns and not (unknowns & uncertainties):
        errors.append(f"item {idx}: uncertainties must include at least one honest_review unknown")


def validate_next_check(
    idx: int,
    review: dict[str, Any],
    item: dict[str, Any],
    errors: list[str],
) -> None:
    unknowns = [
        unknown.lower()
        for dimension in review.get("dimensions", {}).values()
        for unknown in dimension.get("unknown", [])
    ]
    next_check = item.get("recommended_next_check", "").lower()
    if unknowns and not any(keyword_overlap(next_check, unknown) for unknown in unknowns):
        errors.append(f"item {idx}: recommended_next_check does not correspond to honest_review unknowns")


def keyword_overlap(left: str, right: str) -> bool:
    left_tokens = {token for token in left.replace("-", " ").split() if len(token) >= 4}
    right_tokens = {token for token in right.replace("-", " ").split() if len(token) >= 4}
    return bool(left_tokens & right_tokens)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", required=True)
    parser.add_argument("--analysis", required=True)
    args = parser.parse_args()

    bundle = json.loads(Path(args.bundle).read_text())
    analysis = json.loads(Path(args.analysis).read_text())
    errors = validate(bundle, analysis)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print("honest review analysis valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
