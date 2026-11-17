#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
import tomllib
from collections import defaultdict
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DIR = ROOT / "risk-signals"

SIGNALS_FILE = "signals.toml"
DEV_CASES_FILE = "dev-cases.toml"
EVAL_CANDIDATES_FILE = "eval-candidates.toml"
FROZEN_PILOT_FILE = "frozen-pilot-v1.toml"
FROZEN_V1_FILE = "frozen-v1.toml"
HOLDOUT_FILE = "holdout-v1.toml"

REQUIRED_SIGNAL_FIELDS = {
    "id",
    "name",
    "goal",
    "positive_indicators",
    "negative_boundaries",
    "expected_finding_shape",
}

REQUIRED_DEV_FIELDS = {
    "id",
    "kind",
    "title",
    "diff_summary",
    "expected_behavior",
    "ground_truth",
    "source_refs",
    "production_files",
    "test_files",
    "limitations",
}

REQUIRED_EVAL_FIELDS = {
    "id",
    "status",
    "repo",
    "url",
    "signal_ids",
    "risk_summary",
    "ground_truth_summary",
    "source_refs",
    "production_files",
    "test_files",
    "why_high_value",
    "why_not_noise",
    "limitations",
}

DEV_KINDS = {"positive", "negative", "boundary"}
DEV_EXPECTED = {"detect", "suppress", "low_confidence"}
EVAL_STATUS = {"candidate", "freeze_ready", "needs_more_context", "frozen", "rejected"}
HOLDOUT_GATE_ROLES = {"high_value_positive", "negative_control", "boundary_needs_context"}
HOLDOUT_EXPECTED = {"detect", "suppress_or_context_only", "low_confidence"}
HOLDOUT_MIN_COUNTS = {
    "high_value_positive": 6,
    "negative_control": 2,
    "boundary_needs_context": 2,
}
REQUIRED_HOLDOUT_FIELDS = {
    "id",
    "candidate_status",
    "gate_role",
    "high_value",
    "primary_signal_ids",
    "expected_behavior",
    "why_in_holdout",
    "forbidden_claims",
    "required_context",
}


class ValidationError(Exception):
    pass


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def require_fields(kind: str, item_id: str, item: dict[str, Any], fields: set[str], errors: list[str]) -> None:
    for field in sorted(fields):
        if field not in item:
            errors.append(f"{kind} {item_id}: missing field {field}")


def require_non_empty_list(kind: str, item_id: str, item: dict[str, Any], field: str, errors: list[str]) -> None:
    value = item.get(field)
    if not isinstance(value, list) or not value:
        errors.append(f"{kind} {item_id}: {field} must be a non-empty list")


def require_string(kind: str, item_id: str, item: dict[str, Any], field: str, errors: list[str]) -> None:
    value = item.get(field)
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{kind} {item_id}: {field} must be a non-empty string")


def require_bool(kind: str, item_id: str, item: dict[str, Any], field: str, errors: list[str]) -> None:
    if not isinstance(item.get(field), bool):
        errors.append(f"{kind} {item_id}: {field} must be a boolean")


def signal_ids(signals_data: dict[str, Any], errors: list[str]) -> set[str]:
    signals = signals_data.get("signals", [])
    if not isinstance(signals, list):
        errors.append("signals.toml: signals must be a list")
        return set()

    ids: set[str] = set()
    for index, signal in enumerate(signals):
        item_id = signal.get("id", f"<index:{index}>")
        require_fields("signal", item_id, signal, REQUIRED_SIGNAL_FIELDS, errors)
        for field in ["id", "name", "goal", "expected_finding_shape"]:
            require_string("signal", item_id, signal, field, errors)
        for field in ["positive_indicators", "negative_boundaries"]:
            require_non_empty_list("signal", item_id, signal, field, errors)
        if item_id in ids:
            errors.append(f"signal {item_id}: duplicate id")
        ids.add(item_id)

    if len(ids) != 8:
        errors.append(f"signals.toml: expected exactly 8 signals, got {len(ids)}")
    return ids


def validate_dev_cases(dev_data: dict[str, Any], known_signals: set[str], errors: list[str]) -> None:
    cases = dev_data.get("cases", [])
    if not isinstance(cases, list):
        errors.append("dev-cases.toml: cases must be a list")
        return
    if len(cases) < 32:
        errors.append(f"dev-cases.toml: expected at least 32 cases, got {len(cases)}")

    seen: set[str] = set()
    coverage: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for index, case in enumerate(cases):
        item_id = case.get("id", f"<index:{index}>")
        require_fields("dev case", item_id, case, REQUIRED_DEV_FIELDS, errors)
        if item_id in seen:
            errors.append(f"dev case {item_id}: duplicate id")
        seen.add(item_id)

        kind = case.get("kind")
        expected = case.get("expected_behavior")
        if kind not in DEV_KINDS:
            errors.append(f"dev case {item_id}: invalid kind {kind!r}")
        if expected not in DEV_EXPECTED:
            errors.append(f"dev case {item_id}: invalid expected_behavior {expected!r}")

        direct = case.get("signal_ids", [])
        near_miss = case.get("near_miss_signals", [])
        if kind == "negative":
            if direct:
                errors.append(f"dev case {item_id}: negative cases must use near_miss_signals, not signal_ids")
            signals = near_miss
        else:
            if near_miss:
                errors.append(f"dev case {item_id}: non-negative cases must use signal_ids, not near_miss_signals")
            signals = direct

        if not isinstance(signals, list) or not signals:
            errors.append(f"dev case {item_id}: must reference at least one signal")
            signals = []
        for signal in signals:
            if signal not in known_signals:
                errors.append(f"dev case {item_id}: unknown signal {signal}")
            else:
                coverage[signal][kind] += 1

        require_non_empty_list("dev case", item_id, case, "source_refs", errors)
        for field in ["id", "title", "diff_summary", "ground_truth", "limitations"]:
            require_string("dev case", item_id, case, field, errors)
        if not isinstance(case.get("production_files", []), list):
            errors.append(f"dev case {item_id}: production_files must be a list")
        if not isinstance(case.get("test_files", []), list):
            errors.append(f"dev case {item_id}: test_files must be a list")

    for signal in sorted(known_signals):
        total = sum(coverage[signal].values())
        if total < 4:
            errors.append(f"dev-cases.toml: signal {signal} needs at least 4 dev cases, got {total}")
        for kind in sorted(DEV_KINDS):
            if coverage[signal][kind] < 1:
                errors.append(f"dev-cases.toml: signal {signal} missing {kind} dev case")


def validate_eval_candidates(eval_data: dict[str, Any], known_signals: set[str], errors: list[str]) -> None:
    cases = eval_data.get("cases", [])
    if not isinstance(cases, list):
        errors.append("eval-candidates.toml: cases must be a list")
        return

    target_v1 = int(eval_data.get("target_frozen_eval_v1", 0) or 0)
    if target_v1 < 40:
        errors.append("eval-candidates.toml: target_frozen_eval_v1 must be at least 40")

    seen: set[str] = set()
    frozen_count = 0
    for index, case in enumerate(cases):
        item_id = case.get("id", f"<index:{index}>")
        require_fields("eval candidate", item_id, case, REQUIRED_EVAL_FIELDS, errors)
        if item_id in seen:
            errors.append(f"eval candidate {item_id}: duplicate id")
        seen.add(item_id)

        status = case.get("status")
        if status not in EVAL_STATUS:
            errors.append(f"eval candidate {item_id}: invalid status {status!r}")
        if status == "frozen":
            frozen_count += 1

        require_non_empty_list("eval candidate", item_id, case, "signal_ids", errors)
        require_non_empty_list("eval candidate", item_id, case, "source_refs", errors)
        require_non_empty_list("eval candidate", item_id, case, "production_files", errors)
        for field in [
            "id",
            "repo",
            "url",
            "risk_summary",
            "ground_truth_summary",
            "why_high_value",
            "why_not_noise",
            "limitations",
        ]:
            require_string("eval candidate", item_id, case, field, errors)
        for signal in case.get("signal_ids", []):
            if signal not in known_signals:
                errors.append(f"eval candidate {item_id}: unknown signal {signal}")
        if not isinstance(case.get("test_files", []), list):
            errors.append(f"eval candidate {item_id}: test_files must be a list")

    if frozen_count and frozen_count < target_v1:
        errors.append(
            f"eval-candidates.toml: frozen eval set has {frozen_count} cases, below target {target_v1}"
        )


def validate_frozen_pilot(pilot_data: dict[str, Any], eval_data: dict[str, Any], errors: list[str]) -> None:
    case_ids = pilot_data.get("case_ids", [])
    if not isinstance(case_ids, list) or not case_ids:
        errors.append("frozen-pilot-v1.toml: case_ids must be a non-empty list")
        return

    target = int(pilot_data.get("target_cases", 0) or 0)
    if target < 20:
        errors.append("frozen-pilot-v1.toml: target_cases must be at least 20")
    actual = int(pilot_data.get("actual_cases", 0) or 0)
    if actual != len(case_ids):
        errors.append(f"frozen-pilot-v1.toml: actual_cases {actual} does not match case_ids {len(case_ids)}")
    if len(case_ids) < target:
        errors.append(f"frozen-pilot-v1.toml: pilot set has {len(case_ids)} cases, below target {target}")
    if len(case_ids) != len(set(case_ids)):
        errors.append("frozen-pilot-v1.toml: duplicate case id")

    by_id = {case.get("id"): case for case in eval_data.get("cases", []) if isinstance(case, dict)}
    for case_id in case_ids:
        case = by_id.get(case_id)
        if not case:
            errors.append(f"frozen-pilot-v1.toml: unknown case {case_id}")
        elif case.get("status") != "freeze_ready":
            errors.append(f"frozen-pilot-v1.toml: case {case_id} status must be freeze_ready")


def validate_frozen_v1(v1_data: dict[str, Any], eval_data: dict[str, Any], errors: list[str]) -> None:
    case_ids = v1_data.get("case_ids", [])
    if not isinstance(case_ids, list) or not case_ids:
        errors.append("frozen-v1.toml: case_ids must be a non-empty list")
        return

    status = v1_data.get("status")
    if status not in {"building", "complete"}:
        errors.append("frozen-v1.toml: status must be building or complete")

    target = int(v1_data.get("target_cases", 0) or 0)
    if target != 40:
        errors.append("frozen-v1.toml: target_cases must be exactly 40")
    actual = int(v1_data.get("actual_cases", 0) or 0)
    if actual != len(case_ids):
        errors.append(f"frozen-v1.toml: actual_cases {actual} does not match case_ids {len(case_ids)}")
    if len(case_ids) != len(set(case_ids)):
        errors.append("frozen-v1.toml: duplicate case id")

    gap = v1_data.get("gap", {})
    if status == "complete":
        if len(case_ids) != target:
            errors.append(f"frozen-v1.toml: complete set must have {target} cases, got {len(case_ids)}")
        missing = int(gap.get("missing_cases", -1) if isinstance(gap, dict) else -1)
        if missing != 0:
            errors.append("frozen-v1.toml: complete set must have gap.missing_cases = 0")
    else:
        missing = int(gap.get("missing_cases", -1) if isinstance(gap, dict) else -1)
        if missing != target - len(case_ids):
            errors.append(
                f"frozen-v1.toml: gap.missing_cases {missing} does not match target-case delta {target - len(case_ids)}"
            )
        if len(case_ids) >= target:
            errors.append("frozen-v1.toml: status building cannot have target_cases or more case_ids")

    metrics = v1_data.get("metrics", {})
    if not isinstance(metrics, dict):
        errors.append("frozen-v1.toml: metrics must be a table")
    else:
        for field in [
            "precision_at_top3_target",
            "actionable_pr_rate_target",
            "negative_control_silence_rate_target",
            "missed_high_risk_count_max",
            "weak_only_top_risk_max",
        ]:
            if field not in metrics:
                errors.append(f"frozen-v1.toml: metrics missing {field}")

    by_id = {case.get("id"): case for case in eval_data.get("cases", []) if isinstance(case, dict)}
    for case_id in case_ids:
        case = by_id.get(case_id)
        if not case:
            errors.append(f"frozen-v1.toml: unknown case {case_id}")
        elif case.get("status") != "freeze_ready":
            errors.append(f"frozen-v1.toml: case {case_id} status must be freeze_ready")


def validate_holdout(
    holdout_data: dict[str, Any],
    eval_data: dict[str, Any],
    known_signals: set[str],
    errors: list[str],
) -> None:
    for field in ["version", "status", "source", "purpose"]:
        require_string("holdout", "<root>", holdout_data, field, errors)

    target = holdout_data.get("target_cases")
    actual = holdout_data.get("actual_cases")
    cases = holdout_data.get("cases", [])
    use_rules = holdout_data.get("use_rules", [])

    if not isinstance(target, int) or target < 10:
        errors.append("holdout-v1.toml: target_cases must be an integer >= 10")
    if not isinstance(actual, int):
        errors.append("holdout-v1.toml: actual_cases must be an integer")
    if not isinstance(cases, list) or not cases:
        errors.append("holdout-v1.toml: cases must be a non-empty list")
        return
    if isinstance(actual, int) and actual != len(cases):
        errors.append(f"holdout-v1.toml: actual_cases {actual} does not match cases {len(cases)}")
    if isinstance(target, int) and len(cases) != target:
        errors.append(f"holdout-v1.toml: target_cases {target} does not match cases {len(cases)}")
    if not isinstance(use_rules, list) or len(use_rules) < 3:
        errors.append("holdout-v1.toml: use_rules must contain at least 3 rules")
    elif not all(isinstance(rule, str) and rule.strip() for rule in use_rules):
        errors.append("holdout-v1.toml: use_rules must be non-empty strings")

    gate = holdout_data.get("gate", {})
    if not isinstance(gate, dict):
        errors.append("holdout-v1.toml: gate must be a table")
    else:
        for field in [
            "min_detection_precision",
            "min_value_precision",
            "min_evidence_valid_rate",
            "max_overclaim_rate",
            "min_unsupported_scope_silence_rate",
        ]:
            value = gate.get(field)
            if not isinstance(value, (int, float)) or value < 0 or value > 1:
                errors.append(f"holdout-v1.toml: gate.{field} must be between 0 and 1")

    by_id = {case.get("id"): case for case in eval_data.get("cases", []) if isinstance(case, dict)}
    seen: set[str] = set()
    role_counts: dict[str, int] = defaultdict(int)
    covered_signals: set[str] = set()
    for index, case in enumerate(cases):
        item_id = case.get("id", f"<index:{index}>")
        require_fields("holdout case", item_id, case, REQUIRED_HOLDOUT_FIELDS, errors)
        if item_id in seen:
            errors.append(f"holdout case {item_id}: duplicate id")
        seen.add(item_id)

        for field in ["id", "candidate_status", "gate_role", "expected_behavior", "why_in_holdout"]:
            require_string("holdout case", item_id, case, field, errors)
        require_bool("holdout case", item_id, case, "high_value", errors)
        require_non_empty_list("holdout case", item_id, case, "primary_signal_ids", errors)
        require_non_empty_list("holdout case", item_id, case, "forbidden_claims", errors)
        require_non_empty_list("holdout case", item_id, case, "required_context", errors)

        candidate_status = case.get("candidate_status")
        gate_role = case.get("gate_role")
        expected = case.get("expected_behavior")
        high_value = case.get("high_value")
        if candidate_status not in EVAL_STATUS:
            errors.append(f"holdout case {item_id}: invalid candidate_status {candidate_status!r}")
        if gate_role not in HOLDOUT_GATE_ROLES:
            errors.append(f"holdout case {item_id}: invalid gate_role {gate_role!r}")
        else:
            role_counts[gate_role] += 1
        if expected not in HOLDOUT_EXPECTED:
            errors.append(f"holdout case {item_id}: invalid expected_behavior {expected!r}")

        candidate = by_id.get(item_id)
        if not candidate:
            errors.append(f"holdout-v1.toml: unknown case {item_id}")
            candidate_signals: set[str] = set()
        else:
            candidate_signals = set(candidate.get("signal_ids", []))
            if candidate.get("status") != candidate_status:
                errors.append(
                    f"holdout case {item_id}: candidate_status {candidate_status!r} does not match eval-candidates status {candidate.get('status')!r}"
                )

        for signal in case.get("primary_signal_ids", []):
            if signal not in known_signals:
                errors.append(f"holdout case {item_id}: unknown signal {signal}")
            else:
                covered_signals.add(signal)
            if candidate and signal not in candidate_signals:
                errors.append(f"holdout case {item_id}: signal {signal} not present in eval-candidates signal_ids")

        if gate_role == "high_value_positive":
            if candidate_status != "freeze_ready":
                errors.append(f"holdout case {item_id}: high_value_positive must reference freeze_ready case")
            if high_value is not True:
                errors.append(f"holdout case {item_id}: high_value_positive must set high_value = true")
            if expected != "detect":
                errors.append(f"holdout case {item_id}: high_value_positive must use expected_behavior = detect")
        elif gate_role == "negative_control":
            if candidate_status != "rejected":
                errors.append(f"holdout case {item_id}: negative_control must reference rejected case")
            if high_value is not False:
                errors.append(f"holdout case {item_id}: negative_control must set high_value = false")
            if expected != "suppress_or_context_only":
                errors.append(
                    f"holdout case {item_id}: negative_control must use expected_behavior = suppress_or_context_only"
                )
        elif gate_role == "boundary_needs_context":
            if candidate_status != "needs_more_context":
                errors.append(f"holdout case {item_id}: boundary_needs_context must reference needs_more_context case")
            if expected != "low_confidence":
                errors.append(
                    f"holdout case {item_id}: boundary_needs_context must use expected_behavior = low_confidence"
                )

    for role, minimum in HOLDOUT_MIN_COUNTS.items():
        if role_counts[role] < minimum:
            errors.append(f"holdout-v1.toml: gate_role {role} needs at least {minimum} cases, got {role_counts[role]}")
    if len(covered_signals) < 6:
        errors.append(f"holdout-v1.toml: holdout must cover at least 6 distinct signals, got {len(covered_signals)}")


def validate_pack(root: Path) -> list[str]:
    errors: list[str] = []
    signals_data = load_toml(root / SIGNALS_FILE)
    known_signals = signal_ids(signals_data, errors)
    validate_dev_cases(load_toml(root / DEV_CASES_FILE), known_signals, errors)
    eval_data = load_toml(root / EVAL_CANDIDATES_FILE)
    validate_eval_candidates(eval_data, known_signals, errors)
    validate_frozen_pilot(load_toml(root / FROZEN_PILOT_FILE), eval_data, errors)
    validate_frozen_v1(load_toml(root / FROZEN_V1_FILE), eval_data, errors)
    validate_holdout(load_toml(root / HOLDOUT_FILE), eval_data, known_signals, errors)
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=DEFAULT_DIR)
    args = parser.parse_args()

    errors = validate_pack(args.root)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"risk signal pack valid: {args.root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
