#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_HOLDOUT = ROOT / "risk-signals" / "holdout-v1.toml"

STATUS = {"pending", "complete"}
GATE_ROLES = {"high_value_positive", "negative_control", "boundary_needs_context"}
EXPECTED_BEHAVIOR = {"detect", "suppress_or_context_only", "low_confidence"}
DETECTION = {"hit", "miss", "partial", "silent", "not_applicable"}
VALUE_VERDICT = {"useful", "plausible", "noise", "silent", "unsupported", "not_applicable"}
SEMANTIC_NET_VALUE = {"better", "same", "worse", "unknown"}
MODE_NAMES = ["fact_only", "semantic_filtered"]


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_holdout_cases(path: Path) -> dict[str, dict[str, Any]]:
    data = load_toml(path)
    cases = data.get("cases", [])
    if not isinstance(cases, list):
        raise ValueError(f"{path}: cases must be a list")
    return {str(case["id"]): case for case in cases if isinstance(case, dict) and "id" in case}


def require_table(
    item: dict[str, Any],
    key: str,
    errors: list[str],
    label: str,
) -> dict[str, Any]:
    value = item.get(key)
    if not isinstance(value, dict):
        errors.append(f"{label}: {key} must be a table")
        return {}
    return value


def require_string(value: Any, label: str, errors: list[str], allow_empty: bool = False) -> None:
    if not isinstance(value, str):
        errors.append(f"{label} must be a string")
    elif not allow_empty and not value.strip():
        errors.append(f"{label} must be a non-empty string")


def require_bool(value: Any, label: str, errors: list[str]) -> None:
    if not isinstance(value, bool):
        errors.append(f"{label} must be a boolean")


def require_int(value: Any, label: str, errors: list[str]) -> None:
    if not isinstance(value, int) or value < 0:
        errors.append(f"{label} must be a non-negative integer")


def require_string_list(value: Any, label: str, errors: list[str], allow_empty: bool = False) -> None:
    if not isinstance(value, list):
        errors.append(f"{label} must be a list")
        return
    if not allow_empty and not value:
        errors.append(f"{label} must be a non-empty list")
        return
    for item in value:
        if not isinstance(item, str) or not item.strip():
            errors.append(f"{label} must contain non-empty strings")
            return


def validate_ground_truth(
    ground_truth: dict[str, Any],
    holdout_case: dict[str, Any] | None,
    strict: bool,
    errors: list[str],
) -> None:
    require_string(ground_truth.get("gate_role"), "ground_truth.gate_role", errors, allow_empty=not strict)
    require_bool(ground_truth.get("high_value"), "ground_truth.high_value", errors)
    require_string_list(
        ground_truth.get("primary_signal_ids"),
        "ground_truth.primary_signal_ids",
        errors,
        allow_empty=not strict,
    )
    require_string(
        ground_truth.get("expected_behavior"),
        "ground_truth.expected_behavior",
        errors,
        allow_empty=not strict,
    )
    require_string_list(
        ground_truth.get("forbidden_claims"),
        "ground_truth.forbidden_claims",
        errors,
        allow_empty=not strict,
    )
    require_string_list(
        ground_truth.get("required_context"),
        "ground_truth.required_context",
        errors,
        allow_empty=not strict,
    )
    require_string(ground_truth.get("notes"), "ground_truth.notes", errors, allow_empty=True)

    gate_role = ground_truth.get("gate_role", "")
    expected_behavior = ground_truth.get("expected_behavior", "")
    if gate_role and gate_role not in GATE_ROLES:
        errors.append(f"ground_truth.gate_role invalid: {gate_role!r}")
    if expected_behavior and expected_behavior not in EXPECTED_BEHAVIOR:
        errors.append(f"ground_truth.expected_behavior invalid: {expected_behavior!r}")

    if holdout_case is None:
        return

    if strict:
        if ground_truth.get("gate_role") != holdout_case.get("gate_role"):
            errors.append("ground_truth.gate_role does not match holdout contract")
        if ground_truth.get("high_value") != holdout_case.get("high_value"):
            errors.append("ground_truth.high_value does not match holdout contract")
        if set(ground_truth.get("primary_signal_ids", [])) != set(holdout_case.get("primary_signal_ids", [])):
            errors.append("ground_truth.primary_signal_ids does not match holdout contract")
        if ground_truth.get("expected_behavior") != holdout_case.get("expected_behavior"):
            errors.append("ground_truth.expected_behavior does not match holdout contract")
        if not set(holdout_case.get("forbidden_claims", [])).issubset(set(ground_truth.get("forbidden_claims", []))):
            errors.append("ground_truth.forbidden_claims missing holdout contract claims")
        if not set(holdout_case.get("required_context", [])).issubset(set(ground_truth.get("required_context", []))):
            errors.append("ground_truth.required_context missing holdout contract context")


def validate_mode(mode: dict[str, Any], mode_name: str, strict: bool, errors: list[str]) -> None:
    require_int(mode.get("top3_total"), f"{mode_name}.top3_total", errors)
    require_int(mode.get("top3_useful"), f"{mode_name}.top3_useful", errors)
    require_bool(mode.get("actionable"), f"{mode_name}.actionable", errors)
    require_int(mode.get("missed_high_risk_count"), f"{mode_name}.missed_high_risk_count", errors)
    require_int(mode.get("weak_only_top_risk_count"), f"{mode_name}.weak_only_top_risk_count", errors)
    require_string(mode.get("detection"), f"{mode_name}.detection", errors, allow_empty=not strict)
    require_string(mode.get("value_verdict"), f"{mode_name}.value_verdict", errors, allow_empty=not strict)
    require_bool(mode.get("evidence_valid"), f"{mode_name}.evidence_valid", errors)
    require_bool(mode.get("overclaim"), f"{mode_name}.overclaim", errors)
    require_bool(mode.get("forbidden_claim_breached"), f"{mode_name}.forbidden_claim_breached", errors)
    require_bool(mode.get("unsupported_scope_silence"), f"{mode_name}.unsupported_scope_silence", errors)
    require_bool(mode.get("next_check_useful"), f"{mode_name}.next_check_useful", errors)
    require_string(mode.get("notes"), f"{mode_name}.notes", errors, allow_empty=True)

    top3_total = mode.get("top3_total")
    top3_useful = mode.get("top3_useful")
    if isinstance(top3_total, int) and isinstance(top3_useful, int) and top3_useful > top3_total:
        errors.append(f"{mode_name}.top3_useful cannot exceed top3_total")

    detection = mode.get("detection", "")
    verdict = mode.get("value_verdict", "")
    if detection and detection not in DETECTION:
        errors.append(f"{mode_name}.detection invalid: {detection!r}")
    if verdict and verdict not in VALUE_VERDICT:
        errors.append(f"{mode_name}.value_verdict invalid: {verdict!r}")
    if strict and mode.get("overclaim") and not mode.get("forbidden_claim_breached"):
        errors.append(f"{mode_name}: overclaim requires forbidden_claim_breached = true")


def validate_comparison(comparison: dict[str, Any], strict: bool, errors: list[str]) -> None:
    require_bool(comparison.get("semantic_improved_ranking"), "comparison.semantic_improved_ranking", errors)
    require_bool(comparison.get("semantic_reduced_noise"), "comparison.semantic_reduced_noise", errors)
    require_bool(comparison.get("semantic_introduced_noise"), "comparison.semantic_introduced_noise", errors)
    require_string(
        comparison.get("semantic_net_value"),
        "comparison.semantic_net_value",
        errors,
        allow_empty=not strict,
    )
    require_string(comparison.get("notes"), "comparison.notes", errors, allow_empty=True)

    value = comparison.get("semantic_net_value", "")
    if value and value not in SEMANTIC_NET_VALUE:
        errors.append(f"comparison.semantic_net_value invalid: {value!r}")


def validate_gate(gate: dict[str, Any], strict: bool, errors: list[str], holdout_case: dict[str, Any] | None) -> None:
    require_bool(gate.get("ground_truth_complete"), "gate.ground_truth_complete", errors)
    require_bool(gate.get("forbidden_claims_checked"), "gate.forbidden_claims_checked", errors)
    require_bool(gate.get("holdout_eligible"), "gate.holdout_eligible", errors)
    require_string(gate.get("notes"), "gate.notes", errors, allow_empty=True)

    if strict:
        if gate.get("ground_truth_complete") is not True:
            errors.append("gate.ground_truth_complete must be true for complete judgements")
        if gate.get("forbidden_claims_checked") is not True:
            errors.append("gate.forbidden_claims_checked must be true for complete judgements")
        if holdout_case is not None and gate.get("holdout_eligible") is not True:
            errors.append("gate.holdout_eligible must be true for holdout judgements")


def validate_judgement(
    data: dict[str, Any],
    holdout_case: dict[str, Any] | None = None,
) -> list[str]:
    errors: list[str] = []

    require_string(data.get("version"), "version", errors)
    require_string(data.get("case_id"), "case_id", errors)
    require_string(data.get("repo"), "repo", errors)
    if not isinstance(data.get("pr"), int) or int(data["pr"]) <= 0:
        errors.append("pr must be a positive integer")
    status = data.get("status")
    if status not in STATUS:
        errors.append(f"status invalid: {status!r}")
        status = "pending"
    strict = status == "complete"
    require_string(data.get("judge"), "judge", errors, allow_empty=not strict)
    require_string(data.get("judged_at"), "judged_at", errors, allow_empty=not strict)
    require_string(data.get("ground_truth_summary"), "ground_truth_summary", errors, allow_empty=not strict)

    ground_truth = require_table(data, "ground_truth", errors, "judgement")
    validate_ground_truth(ground_truth, holdout_case, strict, errors)

    for mode_name in MODE_NAMES:
        mode = require_table(data, mode_name, errors, "judgement")
        validate_mode(mode, mode_name, strict, errors)

    comparison = require_table(data, "comparison", errors, "judgement")
    validate_comparison(comparison, strict, errors)
    gate = require_table(data, "gate", errors, "judgement")
    validate_gate(gate, strict, errors, holdout_case)
    return errors


def judgement_paths(inputs: list[Path]) -> list[Path]:
    paths: list[Path] = []
    for item in inputs:
        if item.is_dir():
            paths.extend(sorted(path for path in item.glob("*.toml") if path.is_file()))
        else:
            paths.append(item)
    return paths


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("judgements", nargs="+", type=Path)
    parser.add_argument("--holdout", type=Path, default=DEFAULT_HOLDOUT)
    args = parser.parse_args(argv)

    holdout_cases = load_holdout_cases(args.holdout)
    errors: list[str] = []
    for path in judgement_paths(args.judgements):
        data = load_toml(path)
        case_id = str(data.get("case_id", ""))
        path_errors = validate_judgement(data, holdout_cases.get(case_id))
        errors.extend(f"{path}: {error}" for error in path_errors)

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print("risk eval judgements valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
