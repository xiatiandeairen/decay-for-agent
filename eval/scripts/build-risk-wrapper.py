#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from decay_evidence import parse_stdout


WRAPPER_QUESTIONS = {
    "ConcurrencyOrderingWrapper": "Did this diff change concurrency ordering, wakeup, task handoff, or shared-state happens-before behavior?",
    "ErrorPropagationWrapper": "Did this diff change error propagation or terminal error handling?",
    "ExternalContractWrapper": "Did this diff change a public API, protocol-visible behavior, default contract, config contract, or external process/dependency contract?",
    "IntegerBoundaryWrapper": "Did this diff change platform-sized integer boundary behavior?",
    "LifecycleOrderingWrapper": "Did this diff change lifecycle ordering, shutdown, upgrade, cancel, or cleanup behavior?",
    "StateTransitionWrapper": "Did this diff change state transition or terminal-state preservation?",
}

ERROR_PROPAGATION_KINDS = {
    "panic_to_error_propagation_changed",
    "readiness_error_propagation_changed",
    "drop_error_propagation_changed",
    "error_propagation_changed",
}


def parse_decay_stdout(stdout: str) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    parsed = parse_stdout(stdout)
    return parsed["git_context"], parsed["evidence"]


def parse_diff_patch(diff: str) -> dict[str, list[dict[str, Any]]]:
    files: dict[str, list[dict[str, Any]]] = {}
    current_file: str | None = None
    current_hunk: dict[str, Any] | None = None

    for line in diff.splitlines():
        if line.startswith("diff --git "):
            current_file = None
            current_hunk = None
            continue
        if line.startswith("+++ b/"):
            current_file = line[len("+++ b/") :]
            files.setdefault(current_file, [])
            continue
        if line.startswith("@@ "):
            if current_file is None:
                continue
            current_hunk = {"header": line, "lines": []}
            files[current_file].append(current_hunk)
            continue
        if current_file is not None and current_hunk is not None:
            if line.startswith(("+", "-", " ")):
                current_hunk["lines"].append(line)

    return files


def wrapper_type_for(item: dict[str, Any], file_direct_kinds: set[str]) -> str | None:
    kind = item["kind"]
    subject = item["subject"].lower()
    if kind == "panic_to_error_propagation_changed":
        return "ErrorPropagationWrapper"
    if kind == "readiness_error_propagation_changed":
        return "ErrorPropagationWrapper"
    if kind == "drop_error_propagation_changed":
        return "ErrorPropagationWrapper"
    if kind == "error_propagation_changed":
        return "ErrorPropagationWrapper"
    if kind == "integer_platform_boundary_changed":
        return "IntegerBoundaryWrapper"
    if kind == "concurrency_ordering_changed":
        return "ConcurrencyOrderingWrapper"
    if kind in {
        "public_api_added",
        "public_api_function_added",
        "public_api_signature_changed",
        "protocol_shape_changed",
        "protocol_header_authority_changed",
        "default_value_changed",
        "config_value_changed",
        "env_default_changed",
        "security_sensitive_dependency_changed",
    }:
        return "ExternalContractWrapper"
    if kind == "lifecycle_ordering_changed":
        return "LifecycleOrderingWrapper"
    if (
        kind in {"boundary_expression_changed", "boundary_condition_changed"}
        and state_like_subject(subject)
        and not (file_direct_kinds & ERROR_PROPAGATION_KINDS)
    ):
        return "StateTransitionWrapper"
    return None


def build_bundle(
    case_id: str,
    decay_stdout: str,
    diff_patch: str,
    pr_json: dict[str, Any] | None,
) -> dict[str, Any]:
    git_context, evidence = parse_decay_stdout(decay_stdout)
    hunks_by_file = parse_diff_patch(diff_patch)
    wrappers: list[dict[str, Any]] = []

    grouped_primaries: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for item in [item for item in evidence if item["value"] == "direct"]:
        file = item.get("file", "")
        file_direct_kinds = {
            candidate["kind"]
            for candidate in evidence
            if candidate["value"] == "direct" and candidate.get("file") == file
        }
        wrapper_type = wrapper_type_for(item, file_direct_kinds)
        if wrapper_type is None:
            continue
        key = (wrapper_type, file)
        grouped_primaries.setdefault(key, []).append(item)

    for (wrapper_type, file), primary_items in grouped_primaries.items():
        primary = select_primary_evidence(primary_items)
        file_evidence = [item for item in evidence if item.get("file") == file]
        wrapper_hunks = relevant_hunks(hunks_by_file.get(file, []), primary_items)
        wrapper_context = wrapper_specific_context(
            wrapper_type,
            wrapper_hunks,
            primary,
            hunks_by_file,
            primary_items,
        )
        wrapper = {
            "wrapper_id": f"w{len(wrappers) + 1}",
            "wrapper_type": wrapper_type,
            "risk_question": WRAPPER_QUESTIONS[wrapper_type],
            "primary_evidence": primary_items,
            "supporting_evidence": [item for item in file_evidence if item["value"] == "supporting"],
            "context_evidence": [item for item in file_evidence if item["value"] == "context"],
            "coverage_limits": [item for item in file_evidence if item["value"] == "coverage_limit"],
            "diff_context": {"files": [{"path": file, "hunks": wrapper_hunks}]},
            "wrapper_context": wrapper_context,
            "analysis_contract": analysis_contract(wrapper_type),
        }
        honest_review = build_honest_review(wrapper_type, wrapper_context, primary, pr_json)
        if honest_review:
            wrapper["honest_review"] = honest_review
        wrappers.append(wrapper)

    return {
        "version": "risk-context-wrapper.v1",
        "case_id": case_id,
        "git_context": git_context,
        "pr_context": pr_context(pr_json),
        "wrappers": wrappers,
    }


def select_primary_evidence(items: list[dict[str, Any]]) -> dict[str, Any]:
    return sorted(items, key=primary_sort_key)[0]


def primary_sort_key(item: dict[str, Any]) -> tuple[int, int, int]:
    kind_rank = {
        "panic_to_error_propagation_changed": 0,
        "readiness_error_propagation_changed": 1,
        "drop_error_propagation_changed": 2,
        "error_propagation_changed": 3,
        "protocol_header_authority_changed": 0,
        "public_api_function_added": 1,
        "public_api_added": 2,
        "public_api_signature_changed": 3,
        "protocol_shape_changed": 4,
        "security_sensitive_dependency_changed": 5,
        "default_value_changed": 6,
        "config_value_changed": 6,
        "env_default_changed": 6,
        "integer_platform_boundary_changed": 0,
        "boundary_expression_changed": 0,
    }
    return (
        kind_rank.get(item["kind"], 9),
        -len(item["subject"]),
        item["id"],
    )


def relevant_hunks(hunks: list[dict[str, Any]], primary_items: list[dict[str, Any]]) -> list[dict[str, Any]]:
    tokens = []
    for item in primary_items:
        for token in subject_tokens(item["subject"]):
            if token not in tokens:
                tokens.append(token)
    selected = []
    seen = set()
    for hunk in hunks:
        text = "\n".join(hunk["lines"])
        if not tokens or any(token in text for token in tokens):
            marker = (hunk["header"], tuple(hunk["lines"][:240]))
            if marker in seen:
                continue
            seen.add(marker)
            selected.append(
                {
                    "header": hunk["header"],
                    "reason": "linked_to_primary_evidence",
                    "lines": hunk["lines"][:240],
                }
            )
    return selected[:5]


def subject_tokens(subject: str) -> list[str]:
    excluded = {
        "let",
        "mut",
        "return",
        "self",
        "Ok",
        "Err",
        "Some",
        "None",
        "Poll::Ready",
    }
    return [
        token
        for token in re.findall(r"[A-Za-z_][A-Za-z0-9_:]*", subject)
        if len(token) >= 3 and token not in excluded
    ]


def state_like_subject(subject: str) -> bool:
    markers = [
        "closed",
        "close",
        "state",
        "cancel",
        "shutdown",
        "upgrade",
        "pending",
        "ready",
        "reset",
    ]
    return any(marker in subject for marker in markers)


def wrapper_specific_context(
    wrapper_type: str,
    hunks: list[dict[str, Any]],
    primary: dict[str, Any],
    hunks_by_file: dict[str, list[dict[str, Any]]] | None = None,
    primary_items: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    lines = [line for hunk in hunks for line in hunk["lines"]]
    primary_items = primary_items or [primary]
    if wrapper_type == "ErrorPropagationWrapper":
        error_tokens: list[str] = []
        for item in primary_items:
            for token in subject_tokens(item["subject"]):
                if token not in error_tokens:
                    error_tokens.append(token)
        return {
            "before_panic_lines": find_lines(lines, "-", ["unwrap", "expect", "panic", "from_utf8", "to_str"]),
            "after_error_lines": find_lines(lines, "+", ["Err", "error", "return"]),
            "before_error_flow_lines": find_lines(lines, "-", ["Err", "error", "Reason", "NO_ERROR", "CANCEL", "send_error", "recv_msg", "RST_STREAM", "Poll::Ready"]),
            "after_error_flow_lines": find_lines(lines, "+", ["Err", "error", "Reason", "NO_ERROR", "CANCEL", "send_error", "recv_msg", "RST_STREAM", "Poll::Ready"]),
            "before_readiness_lines": find_lines(lines, "-", [".async_io(", ".try_io(", "Interest::READABLE", "Interest::WRITABLE"]),
            "after_readiness_lines": find_lines(lines, "+", [".async_io(", ".try_io(", "Interest::ERROR"]),
            "before_drop_lines": find_lines(lines, "-", ["body_tx: Option", "body_tx: None"]),
            "after_drop_lines": find_lines(lines, "+", ["SenderDropGuard", "impl Drop for", "send_error", "new_incomplete"]),
            "error_tokens": error_tokens,
            "shared_input_tokens": shared_error_input_tokens(lines),
            "related_test_lines": related_test_lines(hunks_by_file or {}),
            "primary_kind": primary["kind"],
            "primary_kinds": [item["kind"] for item in primary_items],
        }
    if wrapper_type == "IntegerBoundaryWrapper":
        before_conversion_lines = find_lines(lines, "-", [" as usize", " as isize", "usize::MAX", "isize::MAX"])
        after_conversion_lines = find_lines(lines, "+", ["try_from", "try_into", "unwrap_or", "usize::MAX", "isize::MAX"])
        conversion_lines = before_conversion_lines + after_conversion_lines
        return {
            "before_conversion_lines": before_conversion_lines,
            "after_conversion_lines": after_conversion_lines,
            "platform_hints": ["usize/isize width differs on 32-bit and 64-bit targets"],
            "shared_integer_tokens": shared_integer_tokens(lines),
            "boundary_role_tokens": boundary_role_tokens(conversion_lines),
            "related_boundary_test_lines": related_boundary_test_lines(hunks_by_file or {}),
        }
    if wrapper_type == "ExternalContractWrapper":
        return {
            "contract_tokens": external_contract_tokens(primary["subject"]),
            "before_contract_lines": find_lines(
                lines,
                "-",
                [
                    "pub fn ",
                    "pub struct ",
                    "HeaderCaseMap",
                    "http10_disable_keep_alive",
                    "keep_alive",
                    "HOST",
                    "authority",
                    "ARGS_",
                    "\"--\"",
                    "dangerous_configuration",
                    "config",
                    "timeout",
                    "protocol",
                    "HeaderName",
                ],
            ),
            "after_contract_lines": find_lines(
                lines,
                "+",
                [
                    "pub fn ",
                    "pub struct ",
                    "HeaderCaseMap",
                    "http10_disable_keep_alive",
                    "keep_alive",
                    "HOST",
                    "authority",
                    "ARGS_",
                    "\"--\"",
                    "dangerous_configuration",
                    "config",
                    "timeout",
                    "protocol",
                    "HeaderName",
                ],
            ),
            "context_contract_lines": find_lines(
                lines,
                " ",
                [
                    "pub fn ",
                    "pub struct ",
                    "HeaderCaseMap",
                    "http10_disable_keep_alive",
                    "keep_alive",
                    "HOST",
                    "authority",
                    "ARGS_",
                    "\"--\"",
                    "dangerous_configuration",
                    "config",
                    "timeout",
                    "protocol",
                    "HeaderName",
                ],
            ),
            "before_related_lines": related_external_contract_lines(lines, "-", primary["subject"]),
            "after_related_lines": related_external_contract_lines(lines, "+", primary["subject"]),
            "shared_contract_tokens": shared_external_contract_tokens(lines),
            "related_contract_test_lines": related_external_contract_test_lines(hunks_by_file or {}),
            "primary_kind": primary["kind"],
            "primary_kinds": [item["kind"] for item in primary_items],
        }
    if wrapper_type == "ConcurrencyOrderingWrapper":
        return {
            "concurrency_tokens": concurrency_tokens(primary["subject"]),
            "before_concurrency_lines": find_lines(lines, "-", ["poll_flush", "wants_read_again", "Lifecycle::Completed", "Cancelled", "num_idle_threads", "notify_one", "wake", "waker", "shutdown", "abort"]),
            "after_concurrency_lines": find_lines(lines, "+", ["wants_write_again", "conn_ready", "Lifecycle::Completed", "CancelData::Open", "OwnedFd::from_raw_fd", "is_counted_idle", "num_idle_threads", "notify_one", "wake", "waker", "shutdown", "abort"]),
            "context_concurrency_lines": find_lines(lines, " ", ["poll_flush", "wants_read_again", "Lifecycle::Completed", "Cancelled", "num_idle_threads", "notify_one", "wake", "waker", "shutdown", "abort"]),
            "before_related_lines": related_concurrency_lines(lines, "-", primary["subject"]),
            "after_related_lines": related_concurrency_lines(lines, "+", primary["subject"]),
            "shared_concurrency_tokens": shared_concurrency_tokens(lines),
            "related_concurrency_test_lines": related_concurrency_test_lines(hunks_by_file or {}),
            "primary_kind": primary["kind"],
            "primary_kinds": [item["kind"] for item in primary_items],
        }
    if wrapper_type == "LifecycleOrderingWrapper":
        return {
            "lifecycle_tokens": lifecycle_tokens(primary["subject"]),
            "before_lifecycle_lines": find_lines(lines, "-", ["graceful_shutdown", "unwrap", "shutdown", "close()", "send_reset", "cancel", "drop(", "take()", "upgrade"]),
            "after_lifecycle_lines": find_lines(lines, "+", ["graceful_shutdown", "if let Some(", "shutdown", "close()", "send_reset", "cancel", "drop(", "take()", "upgrade", "has_initial_read_write_state", "poll_canceled"]),
            "context_lifecycle_lines": find_lines(lines, " ", ["graceful_shutdown", "shutdown", "close()", "send_reset", "cancel", "drop(", "take()", "upgrade"]),
            "before_related_lines": related_lifecycle_lines(lines, "-", primary["subject"]),
            "after_related_lines": related_lifecycle_lines(lines, "+", primary["subject"]),
            "shared_lifecycle_tokens": shared_lifecycle_tokens(lines),
            "related_lifecycle_test_lines": related_lifecycle_test_lines(hunks_by_file or {}),
            "primary_kind": primary["kind"],
            "primary_kinds": [item["kind"] for item in primary_items],
        }
    if wrapper_type == "StateTransitionWrapper":
        return {
            "state_tokens": state_tokens(primary["subject"]),
            "before_state_lines": find_lines(lines, "-", ["CLOSED", "closed", "state", "cancel", "shutdown", "upgrade", "Pending", "Ready", "reserve_capacity"]),
            "after_state_lines": find_lines(lines, "+", ["CLOSED", "closed", "state", "cancel", "shutdown", "upgrade", "Pending", "Ready", "reserve_capacity"]),
            "before_related_lines": related_transition_lines(lines, "-", primary["subject"]),
            "after_related_lines": related_transition_lines(lines, "+", primary["subject"]),
            "shared_transition_tokens": shared_transition_tokens(lines),
            "related_state_test_lines": related_state_test_lines(hunks_by_file or {}),
            "primary_kind": primary["kind"],
            "primary_kinds": [item["kind"] for item in primary_items],
        }
    return {}


def build_honest_review(
    wrapper_type: str,
    context: dict[str, Any],
    primary: dict[str, Any],
    pr_json: dict[str, Any] | None,
) -> dict[str, Any] | None:
    if wrapper_type == "IntegerBoundaryWrapper":
        mechanism = review_integer_mechanism(context)
        impact = review_integer_impact(context, primary, pr_json)
        control = review_integer_control(context)
        return {
            "version": "honest-review.v1",
            "risk_item": "integer_boundary_change",
            "dimensions": {
                "mechanism": mechanism,
                "impact": impact,
                "control": control,
            },
            "overall": overall_integer_review(mechanism, impact, control),
        }
    if wrapper_type == "ExternalContractWrapper":
        mechanism = review_external_contract_mechanism(context)
        impact = review_external_contract_impact(context, primary, pr_json)
        control = review_external_contract_control(context)
        return {
            "version": "honest-review.v1",
            "risk_item": "external_contract_change",
            "dimensions": {
                "mechanism": mechanism,
                "impact": impact,
                "control": control,
            },
            "overall": overall_external_contract_review(mechanism, impact, control),
        }
    if wrapper_type == "ConcurrencyOrderingWrapper":
        mechanism = review_concurrency_mechanism(context)
        impact = review_concurrency_impact(context, primary, pr_json)
        control = review_concurrency_control(context)
        return {
            "version": "honest-review.v1",
            "risk_item": "concurrency_ordering_change",
            "dimensions": {
                "mechanism": mechanism,
                "impact": impact,
                "control": control,
            },
            "overall": overall_concurrency_review(context, mechanism, impact, control),
        }
    if wrapper_type == "LifecycleOrderingWrapper":
        mechanism = review_lifecycle_mechanism(context)
        impact = review_lifecycle_impact(context, primary, pr_json)
        control = review_lifecycle_control(context)
        return {
            "version": "honest-review.v1",
            "risk_item": "lifecycle_ordering_change",
            "dimensions": {
                "mechanism": mechanism,
                "impact": impact,
                "control": control,
            },
            "overall": overall_lifecycle_review(context, mechanism, impact, control),
        }
    if wrapper_type == "StateTransitionWrapper":
        mechanism = review_state_mechanism(context)
        impact = review_state_impact(context, primary, pr_json)
        control = review_state_control(context)
        return {
            "version": "honest-review.v1",
            "risk_item": "state_transition_change",
            "dimensions": {
                "mechanism": mechanism,
                "impact": impact,
                "control": control,
            },
            "overall": overall_state_review(mechanism, impact, control),
        }
    if wrapper_type != "ErrorPropagationWrapper":
        return None

    mechanism = review_error_mechanism(context)
    impact = review_error_impact(context, primary, pr_json)
    control = review_error_control(context)
    return {
        "version": "honest-review.v1",
        "risk_item": "error_propagation_change",
        "dimensions": {
            "mechanism": mechanism,
            "impact": impact,
            "control": control,
        },
        "overall": overall_error_review(mechanism, impact, control),
    }


def review_error_mechanism(context: dict[str, Any]) -> dict[str, Any]:
    before = context.get("before_panic_lines", [])
    after = context.get("after_error_lines", [])
    before_flow = context.get("before_error_flow_lines", [])
    after_flow = context.get("after_error_flow_lines", [])
    before_readiness = context.get("before_readiness_lines", [])
    after_readiness = context.get("after_readiness_lines", [])
    before_drop = context.get("before_drop_lines", [])
    after_drop = context.get("after_drop_lines", [])
    shared = context.get("shared_input_tokens", [])
    primary_kind = context.get("primary_kind")
    observed = []
    if before:
        observed.append(f"removed panic-like error handling: {stable_line_evidence(before[0])}")
    if after:
        observed.append(f"added explicit error return: {stable_line_evidence(after[0])}")
    if before_flow:
        observed.append(f"removed error-flow behavior: {stable_line_evidence(before_flow[0])}")
    if after_flow:
        observed.append(f"added error-flow behavior: {stable_line_evidence(after_flow[0])}")
    if before_readiness:
        observed.append(f"removed previous readiness registration: {stable_line_evidence(before_readiness[0])}")
    if after_readiness:
        observed.append(f"added readiness registration with error wakeups: {stable_line_evidence(after_readiness[0])}")
    if before_drop:
        observed.append(f"removed previous drop-owner shape: {stable_line_evidence(before_drop[0])}")
    if after_drop:
        observed.append(f"added drop-path error surfacing: {stable_line_evidence(after_drop[-1])}")
    if shared:
        observed.append(f"same input token appears on both sides: {', '.join(shared)}")

    if before and after and shared:
        status = "confirmed"
        inferred = ["failure mode changed from panic-like handling to structured error return"]
        unknown = []
    elif primary_kind == "readiness_error_propagation_changed" and before_readiness and after_readiness:
        status = "partial"
        inferred = ["readiness registration now includes explicit error wakeups for the same receive path"]
        unknown = [
            "whether every affected recv-like entry point was updated is not proven by the wrapper alone",
            "whether this changes caller-visible behavior on every platform is not proven",
        ]
    elif primary_kind == "drop_error_propagation_changed" and after_drop:
        status = "partial"
        inferred = ["drop path now sends an explicit incomplete-message error"]
        unknown = [
            "whether every dispatcher-drop path reaches the same sender state is not proven by the wrapper alone",
            "whether this always changes caller-visible body behavior is not proven",
        ]
    elif primary_kind == "error_propagation_changed" and before_flow and after_flow:
        status = "partial"
        inferred = ["error propagation behavior changed in the same diff hunk"]
        unknown = [
            "whether this changes caller-visible behavior is not proven by the wrapper alone",
            "whether this is a fix, compatibility change, or risk introduction is not proven",
        ]
    elif before and after:
        status = "partial"
        inferred = ["panic-like and error-return changes appear nearby"]
        unknown = ["whether both changes handle the same input/error path"]
    elif before or after:
        status = "weak"
        inferred = []
        unknown = ["missing one side of the before/after error propagation change"]
    else:
        status = "unknown"
        inferred = []
        unknown = ["no panic-like before line or error-return after line was extracted"]

    return dimension(status, observed, inferred, unknown)


def review_error_impact(
    context: dict[str, Any],
    primary: dict[str, Any],
    pr_json: dict[str, Any] | None,
) -> dict[str, Any]:
    observed = [f"changed production file: {primary.get('file', '')}"]
    inferred: list[str] = []
    unknown = [
        "full public API contract is not available in this wrapper",
        "call graph from changed function to every caller is not available in this wrapper",
    ]
    tokens = {token.lower() for token in context.get("error_tokens", [])}
    token_text = " ".join(
        context.get("error_tokens", [])
        + context.get("before_error_flow_lines", [])
        + context.get("after_error_flow_lines", [])
        + context.get("before_readiness_lines", [])
        + context.get("after_readiness_lines", [])
        + context.get("before_drop_lines", [])
        + context.get("after_drop_lines", [])
    )

    if "invalidunicodecodepoint".lower() in tokens or any("InvalidUnicodeCodePoint" in line for line in context.get("after_error_lines", [])):
        observed.append("new error code is InvalidUnicodeCodePoint")
        inferred.append("caller-visible error behavior likely changed for malformed unicode input")
    if pr_json:
        title = pr_json.get("title", "")
        body = pr_json.get("body", "")
        if title:
            observed.append(f"PR title: {title}")
        if body:
            observed.append(f"PR body: {body}")
        metadata = f"{title} {body}".lower()
        if any(token in token_text for token in ["NO_ERROR", "CANCEL", "RST_STREAM"]):
            inferred.append("PR metadata is consistent with protocol error or terminal-stream behavior change")
        elif "Interest::ERROR" in token_text or "so_error" in metadata:
            inferred.append("PR metadata is consistent with socket error surfacing on a readiness path")
        elif "new_incomplete" in token_text or any(word in metadata for word in ["dropped", "drop", "mid-body", "incomplete"]):
            inferred.append("PR metadata is consistent with incomplete-body error surfacing on drop")
        elif any(word in metadata for word in ["fix", "panic", "invalid", "utf"]):
            inferred.append("PR metadata is consistent with a malformed-input behavior fix")
    if context.get("related_test_lines"):
        observed.append("related test diff calls a public entry point or asserts an error")
        if any(token in token_text for token in ["NO_ERROR", "CANCEL", "RST_STREAM"]):
            inferred.append("impact likely reaches caller-visible body or stream behavior")
        elif "Interest::ERROR" in token_text:
            inferred.append("impact likely reaches caller-visible socket error behavior")
        elif "new_incomplete" in token_text:
            inferred.append("impact likely reaches caller-visible incomplete-body behavior")
        else:
            inferred.append("impact likely reaches a caller-visible deserialization path")
        unknown.append("whether every affected entry point is covered is not proven")

    status = "partial" if inferred else "weak"
    return dimension(status, observed, inferred, unknown)


def review_error_control(context: dict[str, Any]) -> dict[str, Any]:
    tests = context.get("related_test_lines", [])
    observed = tests[:8]
    inferred: list[str] = []
    unknown: list[str] = []
    if tests and any("unwrap_err" in line or "assert" in line for line in tests):
        status = "partial"
        inferred.append("test evidence covers a related error path and asserts caller-visible failure")
        unknown.append("coverage of all malformed-input variants and all entry points is not proven")
    elif tests:
        status = "weak"
        inferred.append("test diff is related to error handling but assertion strength is unclear")
        unknown.append("whether the test exercises the changed path is not proven")
    else:
        status = "unknown"
        unknown.append("no related test hunk was included in the wrapper")
    return dimension(status, observed, inferred, unknown)


def overall_error_review(
    mechanism: dict[str, Any],
    impact: dict[str, Any],
    control: dict[str, Any],
) -> dict[str, Any]:
    mechanism_status = mechanism["status"]
    impact_status = impact["status"]
    control_status = control["status"]
    if mechanism_status == "confirmed" and impact_status in {"partial", "confirmed"}:
        judgement = "review_worthy_behavior_change"
    elif mechanism_status in {"partial", "weak"}:
        judgement = "needs_more_context_before_review_claim"
    else:
        judgement = "not_enough_evidence"

    return {
        "judgement": judgement,
        "mechanism_status": mechanism_status,
        "impact_status": impact_status,
        "control_status": control_status,
        "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete test coverage",
            "not a merge recommendation",
        ],
    }


def review_integer_mechanism(context: dict[str, Any]) -> dict[str, Any]:
    before = context.get("before_conversion_lines", [])
    after = context.get("after_conversion_lines", [])
    shared = context.get("shared_integer_tokens", [])
    observed = []
    if before:
        observed.append(f"removed integer conversion behavior: {before[0]}")
    if after:
        observed.append(f"added integer boundary conversion behavior: {after[0]}")
    if shared:
        observed.append(f"same numeric token appears on both sides: {', '.join(shared)}")

    if before and after and shared:
        status = "confirmed"
        inferred = ["integer boundary strategy changed from unchecked cast or previous behavior to explicit checked/fallback conversion"]
        unknown = [
            "exact runtime behavior on every target architecture is not proven by the wrapper alone",
        ]
    elif before and after:
        status = "partial"
        inferred = ["integer conversion behavior changed nearby"]
        unknown = ["whether both conversions operate on the same numeric value is not proven"]
    elif before or after:
        status = "weak"
        inferred = []
        unknown = ["missing one side of the before/after integer boundary change"]
    else:
        status = "unknown"
        inferred = []
        unknown = ["no before/after integer conversion lines were extracted"]
    return dimension(status, observed, inferred, unknown)


def review_integer_impact(
    context: dict[str, Any],
    primary: dict[str, Any],
    pr_json: dict[str, Any] | None,
) -> dict[str, Any]:
    observed = [f"changed production file: {primary.get('file', '')}"]
    inferred: list[str] = []
    unknown = [
        "full dataflow from converted value to allocation/read/loop behavior is not available in this wrapper",
        "actual 32-bit runtime behavior is not executed by this wrapper",
    ]
    roles = context.get("boundary_role_tokens", [])
    if roles:
        observed.append(f"boundary role tokens: {', '.join(roles)}")
        inferred.append("converted value likely participates in size/length/read-boundary behavior")
    if any("usize::MAX" in line or "isize::MAX" in line for line in context.get("after_conversion_lines", [])):
        observed.append("fallback uses pointer-sized MAX value")
        inferred.append("large input behavior may saturate rather than truncate or error")
    if pr_json:
        title = pr_json.get("title", "")
        body = pr_json.get("body", "")
        if title:
            observed.append(f"PR title: {title}")
        if any(word in f"{title} {body}".lower() for word in ["32-bit", "overflow", "large", "usize", "content-length"]):
            inferred.append("PR metadata is consistent with a platform or large-input boundary change")

    status = "partial" if inferred else "weak"
    return dimension(status, observed, inferred, unknown)


def review_integer_control(context: dict[str, Any]) -> dict[str, Any]:
    tests = context.get("related_boundary_test_lines", [])
    observed = tests[:8]
    inferred: list[str] = []
    unknown: list[str] = []
    if tests and any(keyword in " ".join(tests).lower() for keyword in ["32", "max", "overflow", "large", "usize"]):
        status = "partial"
        inferred.append("test evidence references a related platform or boundary value")
        unknown.append("test execution on relevant target architectures is not proven")
    elif tests:
        status = "weak"
        inferred.append("test diff is near boundary behavior but exact large/platform coverage is unclear")
        unknown.append("whether the test exercises the changed integer boundary is not proven")
    else:
        status = "unknown"
        unknown.append("no related integer-boundary test hunk was included in the wrapper")
    return dimension(status, observed, inferred, unknown)


def overall_integer_review(
    mechanism: dict[str, Any],
    impact: dict[str, Any],
    control: dict[str, Any],
) -> dict[str, Any]:
    mechanism_status = mechanism["status"]
    impact_status = impact["status"]
    control_status = control["status"]
    if mechanism_status == "confirmed" and impact_status in {"partial", "confirmed"}:
        judgement = "review_worthy_boundary_change"
    elif mechanism_status in {"partial", "weak"}:
        judgement = "needs_more_context_before_review_claim"
    else:
        judgement = "not_enough_evidence"

    return {
        "judgement": judgement,
        "mechanism_status": mechanism_status,
        "impact_status": impact_status,
        "control_status": control_status,
        "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of 32-bit behavior without target-specific execution",
            "not proof of complete test coverage",
            "not a merge recommendation",
        ],
    }


def review_external_contract_mechanism(context: dict[str, Any]) -> dict[str, Any]:
    contract_tokens_list = context.get("contract_tokens", [])
    before = context.get("before_contract_lines", [])
    after = context.get("after_contract_lines", [])
    context_lines = context.get("context_contract_lines", [])
    before_related = context.get("before_related_lines", [])
    after_related = context.get("after_related_lines", [])
    shared = context.get("shared_contract_tokens", [])
    primary_kind = context.get("primary_kind")
    observed = []
    if contract_tokens_list:
        observed.append(
            f"contract-like tokens in primary evidence: {', '.join(contract_tokens_list)}"
        )
    if before:
        observed.append(f"removed prior contract behavior: {stable_line_evidence(before[0])}")
    if after:
        observed.append(f"added contract behavior: {stable_line_evidence(after[0])}")
    if context_lines:
        observed.append(f"nearby contract context: {stable_line_evidence(context_lines[0])}")
    if before_related:
        observed.append(
            f"removed related contract code: {stable_line_evidence(before_related[0])}"
        )
    if after_related:
        observed.append(
            f"added related contract code: {stable_line_evidence(after_related[0])}"
        )
    if shared:
        observed.append(f"same contract token appears on both sides: {', '.join(shared)}")

    token_text = " ".join(
        contract_tokens_list + before + after + context_lines + before_related + after_related
    ).lower()
    has_host_header_contract = primary_kind == "protocol_header_authority_changed" and (
        "host" in token_text
        and "authority" in token_text
        and any("HOST" in line for line in after + after_related)
    )
    has_process_arg_separator = primary_kind == "default_value_changed" and (
        any("\"--\"" in line for line in after + after_related)
        and any("args_" in line.lower() for line in before + after + before_related + after_related)
    )
    has_http10_keep_alive_api = primary_kind == "public_api_function_added" and (
        "http10_disable_keep_alive" in token_text
        and any(marker in token_text for marker in ["keep_alive", "http1", "http/1.0", "disable"])
    )

    if has_host_header_contract:
        status = "confirmed"
        inferred = ["Host header construction changed from authority-derived behavior to host/port-specific behavior"]
        unknown = ["whether every caller or example path now uses the same Host construction is not proven"]
    elif has_process_arg_separator:
        status = "confirmed"
        inferred = ["external decompression command defaults now separate flags from path arguments with `--`"]
        unknown = ["whether every external tool invocation in the project follows the same separator rule is not proven"]
    elif has_http10_keep_alive_api:
        status = "confirmed"
        inferred = ["a new public HTTP/1.0 keep-alive configuration contract was added to the builder surface"]
        unknown = ["whether every client/server builder path applies the same new keep-alive contract is not proven"]
    elif primary_kind in {
        "public_api_added",
        "public_api_function_added",
        "public_api_signature_changed",
        "protocol_shape_changed",
        "security_sensitive_dependency_changed",
        "default_value_changed",
        "config_value_changed",
        "env_default_changed",
    } and (after or after_related):
        status = "partial"
        inferred = ["external-facing contract code changed in the same diff hunk"]
        unknown = [
            "the full caller-visible compatibility surface is not reconstructed by the wrapper alone",
            "whether this is a compatible extension or a breaking semantic shift is not proven",
        ]
    elif before or after or before_related or after_related:
        status = "weak"
        inferred = []
        unknown = ["contract-like code changed, but the external compatibility mechanism is not proven by the wrapper alone"]
    else:
        status = "unknown"
        inferred = []
        unknown = ["no external-contract before/after lines were extracted"]
    return dimension(status, observed, inferred, unknown)


def review_external_contract_impact(
    context: dict[str, Any],
    primary: dict[str, Any],
    pr_json: dict[str, Any] | None,
) -> dict[str, Any]:
    observed = [f"changed file: {primary.get('file', '')}"]
    inferred: list[str] = []
    unknown = [
        "full downstream caller and compatibility surface is not available in this wrapper",
        "cross-version or cross-tool compatibility execution is not available in this wrapper",
    ]
    token_text = " ".join(
        context.get("contract_tokens", [])
        + context.get("before_contract_lines", [])
        + context.get("after_contract_lines", [])
        + context.get("context_contract_lines", [])
        + context.get("before_related_lines", [])
        + context.get("after_related_lines", [])
    ).lower()

    if any(word in token_text for word in ["host", "authority", "port", "header"]):
        inferred.append("protocol-visible header behavior likely changed for callers or servers")
    if any(word in token_text for word in ["args_", "\"--\"", "decompress", "gzip", "bzip", "xz", "zstd", "lz4", "lzma"]):
        inferred.append("external process invocation behavior likely changed for path/flag handling")
    if any(word in token_text for word in ["http10_disable_keep_alive", "keep_alive", "http1"]):
        inferred.append("public builder/config behavior likely changed for HTTP/1 keep-alive semantics")
    if any(word in token_text for word in ["dangerous_configuration", "openssl", "rustls", "crypto", "ffi"]):
        inferred.append("dependency feature behavior or security contract likely changed")
    if any(word in token_text for word in ["pub fn", "pub struct", "protocol", "headercasemap"]):
        inferred.append("public API or protocol-shape compatibility likely changed")
    if pr_json:
        title = pr_json.get("title", "")
        body = pr_json.get("body", "")
        if title:
            observed.append(f"PR title: {title}")
        if body:
            observed.append(f"PR body: {body}")
        metadata = f"{title} {body}".lower()
        if any(word in metadata for word in ["rfc", "host header", "authority", "userinfo", "routing", "protocol"]):
            inferred.append("PR metadata is consistent with a protocol-visible contract change")
        if any(word in metadata for word in ["keep_alive", "keep alive", "http/1.0", "http1.0", "security advisory", "ghsa"]):
            inferred.append("PR metadata is consistent with a public keep-alive or security contract change")
        if any(word in metadata for word in ["filename", "option", "decompression", "tool", "overwrite", "remove data", "side effect"]):
            inferred.append("PR metadata is consistent with an external process invocation contract change")
        if any(word in metadata for word in ["public api", "compat", "connector", "verifier"]):
            inferred.append("PR metadata is consistent with a public compatibility surface change")
    if context.get("related_contract_test_lines"):
        observed.append("related test diff asserts a protocol, config, API, or external-tool behavior")
        inferred.append("impact likely reaches a caller-visible compatibility or behavior surface")
        unknown.append("whether every supported consumer path is covered is not proven")

    status = "partial" if inferred else "weak"
    return dimension(status, observed, inferred, unknown)


def review_external_contract_control(context: dict[str, Any]) -> dict[str, Any]:
    tests = context.get("related_contract_test_lines", [])
    observed = tests[:8]
    inferred: list[str] = []
    unknown: list[str] = []
    text = " ".join(tests).lower()
    if tests and any(
        keyword in text
        for keyword in [
            "assert",
            "host",
            "authority",
            "header",
            "keep_alive",
            "http10",
            "protocol",
            "compat",
            "filename",
            "option",
            "decompress",
            "connector",
            "verifier",
        ]
    ):
        status = "partial"
        inferred.append("test evidence covers a related compatibility, protocol, or external-tool behavior")
        unknown.append("coverage of every consumer-facing contract path is not proven")
    elif tests:
        status = "weak"
        inferred.append("test diff is related to an external contract change but assertion strength is unclear")
        unknown.append("whether the test exercises the changed compatibility path is not proven")
    else:
        status = "unknown"
        unknown.append("no related external-contract test hunk was included in the wrapper")
    return dimension(status, observed, inferred, unknown)


def overall_external_contract_review(
    mechanism: dict[str, Any],
    impact: dict[str, Any],
    control: dict[str, Any],
) -> dict[str, Any]:
    mechanism_status = mechanism["status"]
    impact_status = impact["status"]
    control_status = control["status"]
    if mechanism_status == "confirmed" and impact_status in {"partial", "confirmed"}:
        judgement = "review_worthy_external_contract_change"
    elif mechanism_status in {"partial", "weak"} or (
        mechanism_status == "confirmed" and impact_status == "weak"
    ):
        judgement = "needs_more_context_before_review_claim"
    else:
        judgement = "not_enough_evidence"

    return {
        "judgement": judgement,
        "mechanism_status": mechanism_status,
        "impact_status": impact_status,
        "control_status": control_status,
        "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete compatibility coverage",
            "not proof that every downstream consumer is safe",
            "not a merge recommendation",
        ],
    }


def review_concurrency_mechanism(context: dict[str, Any]) -> dict[str, Any]:
    concurrency_tokens_list = context.get("concurrency_tokens", [])
    before = context.get("before_concurrency_lines", [])
    after = context.get("after_concurrency_lines", [])
    context_lines = context.get("context_concurrency_lines", [])
    before_related = context.get("before_related_lines", [])
    after_related = context.get("after_related_lines", [])
    shared = context.get("shared_concurrency_tokens", [])
    observed = []
    if concurrency_tokens_list:
        observed.append(f"concurrency-like tokens in primary evidence: {', '.join(concurrency_tokens_list)}")
    if before:
        observed.append(f"removed prior concurrency behavior: {stable_line_evidence(before[0])}")
    if after:
        observed.append(f"added concurrency behavior: {stable_line_evidence(after[0])}")
    if context_lines:
        observed.append(f"nearby concurrency context: {stable_line_evidence(context_lines[0])}")
    if before_related:
        observed.append(f"removed related concurrency code: {stable_line_evidence(before_related[0])}")
    if after_related:
        observed.append(f"added related concurrency code: {stable_line_evidence(after_related[0])}")
    if shared:
        observed.append(f"same concurrency token appears on both sides: {', '.join(shared)}")

    token_text = " ".join(
        concurrency_tokens_list + before + after + context_lines + before_related + after_related
    ).lower()
    has_idle_race_fix = any("is_counted_idle" in line for line in after) and (
        "num_idle_threads" in token_text or "notify_one" in token_text or "shutdown" in token_text
    )
    has_wakeup_repoll_fix = any("wants_write_again" in line for line in after) and any(
        "wants_read_again" in line for line in after + before + context_lines
    )
    has_completion_cleanup_fix = any("Lifecycle::Completed(cqe)" in line for line in after) and any(
        marker in token_text
        for marker in ["canceldata::open", "ownedfd::from_raw_fd", "abort", "waker", "wake_by_ref"]
    )

    if has_idle_race_fix:
        status = "confirmed"
        inferred = ["idle-thread accounting now tracks whether a legitimate wakeup already consumed the idle slot"]
        unknown = ["whether every blocking-pool wakeup and shutdown interleaving is covered is not proven"]
    elif has_wakeup_repoll_fix:
        status = "confirmed"
        inferred = ["write-ready and read-ready wakeup ordering is now re-polled instead of yielding without a guaranteed wakeup"]
        unknown = ["whether every custom future and transport implementation hits the same wakeup edge is not proven"]
    elif has_completion_cleanup_fix:
        status = "partial"
        inferred = ["completion-versus-cancel cleanup ordering changed for a completed operation that may be dropped before re-poll"]
        unknown = [
            "whether every completion/cancel interleaving reaches the same cleanup branch is not proven",
            "whether other fd-owning or completion-owning operations share the same ordering fix is not proven",
        ]
    elif concurrency_tokens_list and after and shared:
        status = "partial"
        inferred = ["concurrency or wakeup behavior changed within the same path"]
        unknown = ["full happens-before relationship is not reconstructed by the wrapper alone"]
    elif before or after or before_related or after_related:
        status = "weak"
        inferred = []
        unknown = ["concurrency-like code changed, but the affected ordering invariant is not proven by the wrapper alone"]
    else:
        status = "unknown"
        inferred = []
        unknown = ["no concurrency-ordering before/after lines were extracted"]
    return dimension(status, observed, inferred, unknown)


def review_concurrency_impact(
    context: dict[str, Any],
    primary: dict[str, Any],
    pr_json: dict[str, Any] | None,
) -> dict[str, Any]:
    observed = [f"changed production file: {primary.get('file', '')}"]
    inferred: list[str] = []
    unknown = [
        "full shared-state graph and scheduler interleavings are not available in this wrapper",
        "runtime execution across all task, wakeup, and shutdown paths is not available in this wrapper",
    ]
    token_text = " ".join(
        context.get("concurrency_tokens", [])
        + context.get("before_concurrency_lines", [])
        + context.get("after_concurrency_lines", [])
        + context.get("context_concurrency_lines", [])
        + context.get("before_related_lines", [])
        + context.get("after_related_lines", [])
    ).lower()

    if any(word in token_text for word in ["is_counted_idle", "notify_one", "shutdown", "num_idle_threads"]):
        inferred.append("blocking-pool wakeup versus shutdown ordering likely changed")
    if any(word in token_text for word in ["wants_write_again", "wants_read_again", "conn_ready", "poll_flush"]):
        inferred.append("wakeup and re-poll behavior likely changed for a caller-visible readiness path")
    if any(word in token_text for word in ["Lifecycle::Completed".lower(), "canceldata::open", "ownedfd::from_raw_fd", "abort"]):
        inferred.append("completion versus cancellation cleanup ordering likely changed for an async operation")
    if pr_json:
        title = pr_json.get("title", "")
        body = pr_json.get("body", "")
        if title:
            observed.append(f"PR title: {title}")
        if body:
            observed.append(f"PR body: {body}")
        metadata = f"{title} {body}".lower()
        if any(word in metadata for word in ["race", "deadlock", "wakeup", "lockup", "shutdown", "abort", "drop", "completed", "loom"]):
            inferred.append("PR metadata is consistent with a real concurrency-ordering issue")
    if context.get("related_concurrency_test_lines"):
        observed.append("related test diff asserts a loom race, wakeup, abort, timeout, or shutdown outcome")
        inferred.append("impact likely reaches a real task-scheduling or shared-state outcome")
        unknown.append("whether every affected interleaving is covered is not proven")

    status = "partial" if inferred else "weak"
    return dimension(status, observed, inferred, unknown)


def review_concurrency_control(context: dict[str, Any]) -> dict[str, Any]:
    tests = context.get("related_concurrency_test_lines", [])
    observed = tests[:8]
    inferred: list[str] = []
    unknown: list[str] = []
    text = " ".join(tests).lower()
    if tests and any(keyword in text for keyword in ["loom", "timeout", "deadlock", "abort", "shutdown", "wakeup", "pending", "ready", "race"]):
        status = "partial"
        inferred.append("test evidence covers a related interleaving, wakeup, or shutdown outcome")
        unknown.append("coverage of all scheduler and shared-state interleavings is not proven")
    elif tests:
        status = "weak"
        inferred.append("test diff is related to concurrency behavior but assertion strength is unclear")
        unknown.append("whether the test exercises the changed ordering path is not proven")
    else:
        status = "unknown"
        unknown.append("no related concurrency-ordering test hunk was included in the wrapper")
    return dimension(status, observed, inferred, unknown)


def overall_concurrency_review(
    context: dict[str, Any],
    mechanism: dict[str, Any],
    impact: dict[str, Any],
    control: dict[str, Any],
) -> dict[str, Any]:
    mechanism_status = mechanism["status"]
    impact_status = impact["status"]
    control_status = control["status"]
    concurrency_text = " ".join(
        context.get("concurrency_tokens", [])
        + context.get("after_concurrency_lines", [])
        + context.get("after_related_lines", [])
    ).lower()
    completed_boundary = any(word in concurrency_text for word in ["lifecycle::completed", "ownedfd::from_raw_fd", "abort"])
    if mechanism_status == "confirmed" and impact_status in {"partial", "confirmed"} and not completed_boundary:
        judgement = "review_worthy_concurrency_ordering_change"
    elif mechanism_status in {"confirmed", "partial", "weak"}:
        judgement = "needs_more_context_before_review_claim"
    else:
        judgement = "not_enough_evidence"

    return {
        "judgement": judgement,
        "mechanism_status": mechanism_status,
        "impact_status": impact_status,
        "control_status": control_status,
        "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete concurrency coverage",
            "not proof of all scheduler interleavings",
            "not a merge recommendation",
        ],
    }


def review_lifecycle_mechanism(context: dict[str, Any]) -> dict[str, Any]:
    lifecycle_tokens_list = context.get("lifecycle_tokens", [])
    before = context.get("before_lifecycle_lines", [])
    after = context.get("after_lifecycle_lines", [])
    context_lines = context.get("context_lifecycle_lines", [])
    before_related = context.get("before_related_lines", [])
    after_related = context.get("after_related_lines", [])
    shared = context.get("shared_lifecycle_tokens", [])
    observed = []
    if lifecycle_tokens_list:
        observed.append(f"lifecycle-like tokens in primary evidence: {', '.join(lifecycle_tokens_list)}")
    if before:
        observed.append(f"removed prior lifecycle behavior: {stable_line_evidence(before[0])}")
    if after:
        observed.append(f"added lifecycle behavior: {stable_line_evidence(after[0])}")
    if context_lines:
        observed.append(f"nearby lifecycle context: {stable_line_evidence(context_lines[0])}")
    if before_related:
        observed.append(f"removed related lifecycle code: {stable_line_evidence(before_related[0])}")
    if after_related:
        observed.append(f"added related lifecycle code: {stable_line_evidence(after_related[0])}")
    if shared:
        observed.append(f"same lifecycle token appears on both sides: {', '.join(shared)}")

    token_text = " ".join(lifecycle_tokens_list + before + after + context_lines + before_related + after_related).lower()
    has_shutdown_guard = any("graceful_shutdown" in line for line in after) and any(
        "unwrap" in line and "graceful_shutdown" in line for line in before
    )
    has_initial_shutdown = any("has_initial_read_write_state" in line for line in after) and any(
        "self.close()" in line for line in after + context_lines + after_related
    )
    has_cancel_reset_cleanup = any("send_reset" in line for line in after) and any(
        marker in token_text for marker in ["cancel_result", "cancel_rx", "cancel_tx", "poll_canceled", "drop(", "take()"]
    )

    if has_shutdown_guard:
        status = "confirmed"
        inferred = ["graceful shutdown is now guarded against a missing upgraded inner connection"]
        unknown = ["whether every post-upgrade shutdown caller reaches the same guarded path is not proven"]
    elif has_initial_shutdown:
        status = "confirmed"
        inferred = ["shutdown path now closes immediately when graceful shutdown is requested before first I/O"]
        unknown = ["whether every server lifecycle edge shares the same initial-state guard is not proven"]
    elif has_cancel_reset_cleanup:
        status = "partial"
        inferred = ["cancellation path now resets the stream and drops cleanup handles explicitly"]
        unknown = [
            "whether every async cancellation interleaving reaches the same cleanup ordering is not proven",
            "whether ownership and wakeup ordering stay correct under all concurrent paths is not proven",
        ]
    elif lifecycle_tokens_list and after and shared:
        status = "partial"
        inferred = ["lifecycle or cleanup behavior changed within the same path"]
        unknown = ["full before/after lifecycle ordering is not reconstructed by the wrapper alone"]
    elif after or after_related or before or before_related:
        status = "weak"
        inferred = []
        unknown = ["lifecycle-like code changed, but the affected ordering invariant is not proven by the wrapper alone"]
    else:
        status = "unknown"
        inferred = []
        unknown = ["no lifecycle before/after lines were extracted"]
    return dimension(status, observed, inferred, unknown)


def review_lifecycle_impact(
    context: dict[str, Any],
    primary: dict[str, Any],
    pr_json: dict[str, Any] | None,
) -> dict[str, Any]:
    observed = [f"changed production file: {primary.get('file', '')}"]
    inferred: list[str] = []
    unknown = [
        "full ownership and lifecycle graph is not available in this wrapper",
        "full caller-path and async cleanup impact is not available in this wrapper",
    ]
    token_text = " ".join(
        context.get("lifecycle_tokens", [])
        + context.get("before_lifecycle_lines", [])
        + context.get("after_lifecycle_lines", [])
        + context.get("context_lifecycle_lines", [])
        + context.get("before_related_lines", [])
        + context.get("after_related_lines", [])
    ).lower()

    if any(word in token_text for word in ["graceful_shutdown", "upgrade", "initial", "close"]):
        inferred.append("shutdown or upgrade lifecycle behavior likely changed for a caller-visible connection path")
    if any(word in token_text for word in ["send_reset", "cancel", "rst_stream", "drop("]):
        inferred.append("cancellation or cleanup ordering likely changed for a caller-visible stream path")
    if pr_json:
        title = pr_json.get("title", "")
        body = pr_json.get("body", "")
        if title:
            observed.append(f"PR title: {title}")
        if body:
            observed.append(f"PR body: {body}")
        metadata = f"{title} {body}".lower()
        if any(word in metadata for word in ["shutdown", "upgrade", "hang", "cancel", "drop", "reset", "flow-control", "cleanup"]):
            inferred.append("PR metadata is consistent with a real lifecycle or cleanup ordering change")
    if context.get("related_lifecycle_test_lines"):
        observed.append("related test diff asserts a shutdown, upgrade, cancel, drop, or reset outcome")
        inferred.append("impact likely reaches a caller-visible lifecycle or completion behavior")
        unknown.append("whether every affected lifecycle path is covered is not proven")

    status = "partial" if inferred else "weak"
    return dimension(status, observed, inferred, unknown)


def review_lifecycle_control(context: dict[str, Any]) -> dict[str, Any]:
    tests = context.get("related_lifecycle_test_lines", [])
    observed = tests[:8]
    inferred: list[str] = []
    unknown: list[str] = []
    text = " ".join(tests).lower()
    if tests and any(keyword in text for keyword in ["assert", "shutdown", "upgrade", "cancel", "drop", "reset", "timeout", "hang", "rst_stream"]):
        status = "partial"
        inferred.append("test evidence covers a related lifecycle or cleanup outcome")
        unknown.append("coverage of all lifecycle interleavings and cleanup edges is not proven")
    elif tests:
        status = "weak"
        inferred.append("test diff is related to lifecycle behavior but assertion strength is unclear")
        unknown.append("whether the test exercises the changed lifecycle ordering is not proven")
    else:
        status = "unknown"
        unknown.append("no related lifecycle-ordering test hunk was included in the wrapper")
    return dimension(status, observed, inferred, unknown)


def overall_lifecycle_review(
    context: dict[str, Any],
    mechanism: dict[str, Any],
    impact: dict[str, Any],
    control: dict[str, Any],
) -> dict[str, Any]:
    mechanism_status = mechanism["status"]
    impact_status = impact["status"]
    control_status = control["status"]
    lifecycle_text = " ".join(
        context.get("lifecycle_tokens", [])
        + context.get("after_lifecycle_lines", [])
        + context.get("context_lifecycle_lines", [])
        + context.get("after_related_lines", [])
    ).lower()
    cancel_heavy = any(word in lifecycle_text for word in ["send_reset", "cancel", "rst_stream"])
    if mechanism_status == "confirmed" and impact_status in {"partial", "confirmed"} and not cancel_heavy:
        judgement = "review_worthy_lifecycle_ordering_change"
    elif mechanism_status in {"confirmed", "partial", "weak"}:
        judgement = "needs_more_context_before_review_claim"
    else:
        judgement = "not_enough_evidence"

    return {
        "judgement": judgement,
        "mechanism_status": mechanism_status,
        "impact_status": impact_status,
        "control_status": control_status,
        "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete lifecycle coverage",
            "not proof of all async cleanup interleavings",
            "not a merge recommendation",
        ],
    }


def review_state_mechanism(context: dict[str, Any]) -> dict[str, Any]:
    state_tokens_list = context.get("state_tokens", [])
    before = context.get("before_state_lines", [])
    after = context.get("after_state_lines", [])
    before_related = context.get("before_related_lines", [])
    after_related = context.get("after_related_lines", [])
    shared = context.get("shared_transition_tokens", [])
    observed = []
    if state_tokens_list:
        observed.append(f"state-like tokens in primary evidence: {', '.join(state_tokens_list)}")
    if before:
        observed.append(f"removed prior state-transition behavior: {stable_line_evidence(before[0])}")
    if after:
        observed.append(f"added state-transition behavior: {stable_line_evidence(after[0])}")
    if before_related:
        observed.append(f"removed related transition code: {stable_line_evidence(before_related[0])}")
    if after_related:
        observed.append(f"added related transition code: {stable_line_evidence(after_related[0])}")
    if shared:
        observed.append(f"same transition token appears on both sides: {', '.join(shared)}")

    if state_tokens_list and after and before_related and shared:
        status = "confirmed"
        inferred = ["terminal or cancellation state handling changed within the same transition path"]
        unknown = ["whether every transition edge and concurrent interleaving is covered is not proven"]
    elif state_tokens_list and (after or after_related):
        status = "partial"
        inferred = ["state or terminal-path transition behavior changed nearby"]
        unknown = [
            "whether the full before/after transition graph is reconstructed is not proven",
            "whether this changes every caller-visible state outcome is not proven",
        ]
    elif before or after or before_related or after_related:
        status = "weak"
        inferred = []
        unknown = ["state-like code changed, but the affected invariant is not proven by the wrapper alone"]
    else:
        status = "unknown"
        inferred = []
        unknown = ["no state-transition before/after lines were extracted"]
    return dimension(status, observed, inferred, unknown)


def review_state_impact(
    context: dict[str, Any],
    primary: dict[str, Any],
    pr_json: dict[str, Any] | None,
) -> dict[str, Any]:
    observed = [f"changed production file: {primary.get('file', '')}"]
    inferred: list[str] = []
    unknown = [
        "full state-machine graph is not available in this wrapper",
        "full caller-path and concurrency-path impact is not available in this wrapper",
    ]
    token_text = " ".join(
        context.get("state_tokens", [])
        + context.get("before_state_lines", [])
        + context.get("after_state_lines", [])
        + context.get("before_related_lines", [])
        + context.get("after_related_lines", [])
    ).lower()

    if any("closed" in token.lower() for token in context.get("state_tokens", [])):
        inferred.append("terminal-state preservation likely changed for a closed path")
    if any(word in token_text for word in ["cancel", "pending", "ready", "reset"]):
        inferred.append("cancellation or wakeup transition behavior likely changed")
    if any(word in token_text for word in ["shutdown", "upgrade"]):
        inferred.append("lifecycle transition behavior likely changed around shutdown or upgrade")
    if pr_json:
        title = pr_json.get("title", "")
        body = pr_json.get("body", "")
        if title:
            observed.append(f"PR title: {title}")
        if body:
            observed.append(f"PR body: {body}")
        metadata = f"{title} {body}".lower()
        if any(word in metadata for word in ["closed", "forget", "cancel", "shutdown", "drop", "hang", "upgrade", "deadlock"]):
            inferred.append("PR metadata is consistent with a real state-transition or terminal-path behavior change")
    if context.get("related_state_test_lines"):
        observed.append("related test diff asserts a state, shutdown, cancel, or transition outcome")
        inferred.append("impact likely reaches a caller-visible state or completion behavior")
        unknown.append("whether every affected transition path is covered is not proven")

    status = "partial" if inferred else "weak"
    return dimension(status, observed, inferred, unknown)


def review_state_control(context: dict[str, Any]) -> dict[str, Any]:
    tests = context.get("related_state_test_lines", [])
    observed = tests[:8]
    inferred: list[str] = []
    unknown: list[str] = []
    if tests and any(keyword in " ".join(tests).lower() for keyword in ["assert", "closed", "cancel", "shutdown", "pending", "ready", "hang", "timeout", "deadlock"]):
        status = "partial"
        inferred.append("test evidence covers a related state or transition outcome")
        unknown.append("coverage of all transition edges and interleavings is not proven")
    elif tests:
        status = "weak"
        inferred.append("test diff is related to transition behavior but assertion strength is unclear")
        unknown.append("whether the test exercises the changed transition path is not proven")
    else:
        status = "unknown"
        unknown.append("no related state-transition test hunk was included in the wrapper")
    return dimension(status, observed, inferred, unknown)


def overall_state_review(
    mechanism: dict[str, Any],
    impact: dict[str, Any],
    control: dict[str, Any],
) -> dict[str, Any]:
    mechanism_status = mechanism["status"]
    impact_status = impact["status"]
    control_status = control["status"]
    if mechanism_status == "confirmed" and impact_status in {"partial", "confirmed"}:
        judgement = "review_worthy_state_transition_change"
    elif mechanism_status in {"partial", "weak"}:
        judgement = "needs_more_context_before_review_claim"
    else:
        judgement = "not_enough_evidence"

    return {
        "judgement": judgement,
        "mechanism_status": mechanism_status,
        "impact_status": impact_status,
        "control_status": control_status,
        "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete transition coverage",
            "not proof of all concurrent interleavings",
            "not a merge recommendation",
        ],
    }


def dimension(
    status: str,
    observed: list[str],
    inferred: list[str],
    unknown: list[str],
) -> dict[str, Any]:
    return {
        "status": status,
        "observed": observed,
        "inferred": inferred,
        "unknown": unknown,
    }


def stable_line_evidence(line: str) -> str:
    text = line[1:].strip() if line.startswith(("+", "-", " ")) else line.strip()
    text = re.sub(r"[;,]+$", "", text).strip()
    text = re.sub(r"\s+", " ", text)
    return f"code contains `{text}`"


def shared_error_input_tokens(lines: list[str]) -> list[str]:
    removed = "\n".join(line[1:] for line in lines if line.startswith("-"))
    added = "\n".join(line[1:] for line in lines if line.startswith("+"))
    removed_tokens = error_input_tokens(removed)
    added_tokens = error_input_tokens(added)
    return sorted(removed_tokens & added_tokens)


def error_input_tokens(text: str) -> set[str]:
    tokens: set[str] = set()
    for match in re.finditer(r"(?:from_utf8|to_str|parse|decode)\((?P<arg>[A-Za-z_][A-Za-z0-9_]*)\)", text):
        tokens.add(match.group("arg"))
    for token in re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", text):
        if token in {"raw", "bytes", "input", "slice", "buf"}:
            tokens.add(token)
    return tokens


def shared_integer_tokens(lines: list[str]) -> list[str]:
    removed = "\n".join(line[1:] for line in lines if line.startswith("-"))
    added = "\n".join(line[1:] for line in lines if line.startswith("+"))
    return sorted(integer_tokens(removed) & integer_tokens(added))


def shared_transition_tokens(lines: list[str]) -> list[str]:
    removed = "\n".join(line[1:] for line in lines if line.startswith("-"))
    added = "\n".join(line[1:] for line in lines if line.startswith("+"))
    return sorted(transition_tokens(removed) & transition_tokens(added))


def shared_concurrency_tokens(lines: list[str]) -> list[str]:
    removed = "\n".join(line[1:] for line in lines if line.startswith("-"))
    added = "\n".join(line[1:] for line in lines if line.startswith("+"))
    return sorted(concurrency_text_tokens(removed) & concurrency_text_tokens(added))


def shared_lifecycle_tokens(lines: list[str]) -> list[str]:
    removed = "\n".join(line[1:] for line in lines if line.startswith("-"))
    added = "\n".join(line[1:] for line in lines if line.startswith("+"))
    return sorted(lifecycle_text_tokens(removed) & lifecycle_text_tokens(added))


def shared_external_contract_tokens(lines: list[str]) -> list[str]:
    removed = "\n".join(line[1:] for line in lines if line.startswith("-"))
    added = "\n".join(line[1:] for line in lines if line.startswith("+"))
    return sorted(external_contract_text_tokens(removed) & external_contract_text_tokens(added))


def integer_tokens(text: str) -> set[str]:
    text = re.sub(r"\blet\s+[A-Za-z_][A-Za-z0-9_]*\s*=", "=", text)
    excluded = {
        "let",
        "mut",
        "return",
        "usize",
        "isize",
        "u64",
        "u32",
        "i64",
        "i32",
        "MAX",
        "MIN",
        "try_from",
        "try_into",
        "unwrap_or",
        "unwrap_or_else",
    }
    tokens = set()
    for token in re.findall(r"\*?[A-Za-z_][A-Za-z0-9_]*", text):
        clean = token.lstrip("*")
        if clean not in excluded and len(clean) >= 2:
            tokens.add(clean)
    return tokens


def transition_tokens(text: str) -> set[str]:
    excluded = {
        "let",
        "mut",
        "return",
        "self",
        "fn",
        "pub",
        "crate",
        "Pin",
        "Poll",
        "Some",
        "None",
        "Ok",
        "Err",
    }
    tokens = set()
    for token in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", text):
        if token not in excluded and len(token) >= 3:
            tokens.add(token)
    return tokens


def lifecycle_text_tokens(text: str) -> set[str]:
    markers = ["shutdown", "upgrade", "cancel", "reset", "drop", "close", "take", "initial", "cleanup"]
    excluded = {
        "let",
        "mut",
        "return",
        "self",
        "fn",
        "pub",
        "crate",
        "Pin",
        "Poll",
        "Some",
        "None",
        "Ok",
        "Err",
    }
    tokens = set()
    for token in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", text):
        lower = token.lower()
        if token in excluded:
            continue
        if any(marker in lower for marker in markers):
            tokens.add(token)
    return tokens


def external_contract_text_tokens(text: str) -> set[str]:
    markers = [
        "host",
        "authority",
        "header",
        "protocol",
        "keep_alive",
        "http10",
        "http1",
        "config",
        "default",
        "timeout",
        "retry",
        "args_",
        "gzip",
        "bzip",
        "xz",
        "zstd",
        "lz4",
        "lzma",
        "connector",
        "verifier",
        "feature",
        "openssl",
        "rustls",
        "ffi",
        "headercasemap",
    ]
    excluded = {
        "let",
        "mut",
        "return",
        "self",
        "fn",
        "pub",
        "crate",
        "Some",
        "None",
        "Ok",
        "Err",
    }
    tokens = set()
    for token in re.findall(r"[A-Za-z_][A-Za-z0-9_:]*", text):
        lower = token.lower()
        if token in excluded:
            continue
        if any(marker in lower for marker in markers):
            tokens.add(token)
    return tokens


def concurrency_text_tokens(text: str) -> set[str]:
    markers = [
        "wake",
        "waker",
        "notify",
        "idle",
        "shutdown",
        "abort",
        "completed",
        "cancel",
        "pending",
        "ready",
        "poll",
        "lock",
        "condvar",
        "spawn",
        "join",
        "thread",
    ]
    excluded = {
        "let",
        "mut",
        "return",
        "self",
        "fn",
        "pub",
        "crate",
        "Pin",
        "Poll",
        "Some",
        "None",
        "Ok",
        "Err",
    }
    tokens = set()
    for token in re.findall(r"[A-Za-z_][A-Za-z0-9_:]*", text):
        lower = token.lower()
        if token in excluded:
            continue
        if any(marker in lower for marker in markers):
            tokens.add(token)
    return tokens


def boundary_role_tokens(lines: list[str]) -> list[str]:
    text = "\n".join(lines).lower()
    roles = []
    for token in ["remaining", "rem", "len", "length", "capacity", "offset", "read", "size", "body"]:
        if token in text:
            roles.append(token)
    return roles


def related_test_lines(hunks_by_file: dict[str, list[dict[str, Any]]]) -> list[str]:
    out: list[str] = []
    keywords = [
        "invalid",
        "malformed",
        "utf",
        "unicode",
        "error",
        "unwrap_err",
        "raw",
        "\\x",
        "b'",
        "incomplete",
        "timed out",
        "connectionrefused",
        "so_error",
    ]
    for path, hunks in hunks_by_file.items():
        lower_path = path.lower()
        if not is_test_path(lower_path):
            continue
        for hunk in hunks:
            for line in hunk["lines"]:
                if line.startswith("+") and any(keyword in line.lower() for keyword in keywords):
                    out.append(f"{path}: {line}")
    return out[:40]


def related_boundary_test_lines(hunks_by_file: dict[str, list[dict[str, Any]]]) -> list[str]:
    out: list[str] = []
    keywords = ["32", "max", "overflow", "large", "usize", "isize", "content-length", "length", "remaining"]
    for path, hunks in hunks_by_file.items():
        lower_path = path.lower()
        if not is_test_path(lower_path):
            continue
        for hunk in hunks:
            for line in hunk["lines"]:
                if line.startswith("+") and any(keyword in line.lower() for keyword in keywords):
                    out.append(f"{path}: {line}")
    return out[:40]


def related_state_test_lines(hunks_by_file: dict[str, list[dict[str, Any]]]) -> list[str]:
    out: list[str] = []
    keywords = [
        "closed",
        "cancel",
        "shutdown",
        "upgrade",
        "pending",
        "ready",
        "hang",
        "timeout",
        "deadlock",
        "rst_stream",
        "permit",
        "semaphore",
    ]
    for path, hunks in hunks_by_file.items():
        lower_path = path.lower()
        if not is_test_path(lower_path):
            continue
        for hunk in hunks:
            for line in hunk["lines"]:
                if line.startswith("+") and any(keyword in line.lower() for keyword in keywords):
                    out.append(f"{path}: {line}")
    return out[:40]


def related_concurrency_test_lines(hunks_by_file: dict[str, list[dict[str, Any]]]) -> list[str]:
    out: list[str] = []
    keywords = [
        "loom",
        "timeout",
        "deadlock",
        "abort",
        "shutdown",
        "wake",
        "waker",
        "pending",
        "ready",
        "race",
        "spawn_blocking",
        "leaked",
        "fd",
    ]
    for path, hunks in hunks_by_file.items():
        lower_path = path.lower()
        if not is_test_path(lower_path):
            continue
        for hunk in hunks:
            for line in hunk["lines"]:
                if line.startswith("+") and any(keyword in line.lower() for keyword in keywords):
                    out.append(f"{path}: {line}")
    return out[:40]


def related_lifecycle_test_lines(hunks_by_file: dict[str, list[dict[str, Any]]]) -> list[str]:
    out: list[str] = []
    keywords = [
        "shutdown",
        "upgrade",
        "cancel",
        "drop",
        "reset",
        "rst_stream",
        "timeout",
        "hang",
        "close",
    ]
    for path, hunks in hunks_by_file.items():
        lower_path = path.lower()
        if not is_test_path(lower_path):
            continue
        for hunk in hunks:
            for line in hunk["lines"]:
                if line.startswith("+") and any(keyword in line.lower() for keyword in keywords):
                    out.append(f"{path}: {line}")
    return out[:40]


def related_external_contract_test_lines(hunks_by_file: dict[str, list[dict[str, Any]]]) -> list[str]:
    out: list[str] = []
    keywords = [
        "host",
        "authority",
        "header",
        "keep_alive",
        "http10",
        "protocol",
        "compat",
        "filename",
        "option",
        "decompress",
        "connector",
        "verifier",
        "routing",
        "userinfo",
    ]
    for path, hunks in hunks_by_file.items():
        lower_path = path.lower()
        if not is_test_path(lower_path):
            continue
        for hunk in hunks:
            for line in hunk["lines"]:
                if line.startswith("+") and any(keyword in line.lower() for keyword in keywords):
                    out.append(f"{path}: {line}")
    return out[:40]


def is_test_path(path: str) -> bool:
    return (
        path.startswith("tests/")
        or "/tests/" in path
        or path.endswith("_test.rs")
        or path.endswith("test.rs")
    )


def find_lines(lines: list[str], prefix: str, needles: list[str]) -> list[str]:
    return [
        line
        for line in lines
        if line.startswith(prefix) and any(needle in line for needle in needles)
    ][:20]


def related_transition_lines(lines: list[str], prefix: str, subject: str) -> list[str]:
    tokens = subject_tokens(subject)
    return [
        line
        for line in lines
        if line.startswith(prefix) and any(token in line for token in tokens)
    ][:20]


def related_concurrency_lines(lines: list[str], prefix: str, subject: str) -> list[str]:
    tokens = set(subject_tokens(subject)) | set(concurrency_tokens(subject))
    return [
        line
        for line in lines
        if line.startswith(prefix)
        and (
            any(token in line for token in tokens)
            or any(
                marker in line.lower()
                for marker in [
                    "wake",
                    "waker",
                    "notify",
                    "idle",
                    "shutdown",
                    "abort",
                    "completed",
                    "cancel",
                    "pending",
                    "ready",
                    "poll",
                    "lock",
                    "condvar",
                ]
            )
        )
    ][:20]


def related_lifecycle_lines(lines: list[str], prefix: str, subject: str) -> list[str]:
    tokens = set(subject_tokens(subject)) | set(lifecycle_tokens(subject))
    return [
        line
        for line in lines
        if line.startswith(prefix)
        and (
            any(token in line for token in tokens)
            or any(
                marker in line.lower()
                for marker in ["shutdown", "upgrade", "cancel", "reset", "drop(", "take()", "close()", "initial"]
            )
        )
    ][:20]


def related_external_contract_lines(lines: list[str], prefix: str, subject: str) -> list[str]:
    tokens = set(subject_tokens(subject)) | set(external_contract_tokens(subject))
    return [
        line
        for line in lines
        if line.startswith(prefix)
        and (
            any(token in line for token in tokens)
            or any(
                marker in line.lower()
                for marker in [
                    "host",
                    "authority",
                    "header",
                    "protocol",
                    "keep_alive",
                    "http10",
                    "http1",
                    "config",
                    "default",
                    "timeout",
                    "retry",
                    "args_",
                    "\"--\"",
                    "gzip",
                    "bzip",
                    "xz",
                    "zstd",
                    "lz4",
                    "lzma",
                    "connector",
                    "verifier",
                    "dangerous_configuration",
                    "headercasemap",
                ]
            )
        )
    ][:20]


def lifecycle_tokens(subject: str) -> list[str]:
    out: list[str] = []
    markers = ["shutdown", "upgrade", "cancel", "reset", "drop", "close", "take", "initial", "cleanup"]
    for token in subject_tokens(subject):
        parts = token.split("::")
        for part in parts:
            lower = part.lower()
            if any(marker in lower for marker in markers) and part not in out:
                out.append(part)
    return out


def external_contract_tokens(subject: str) -> list[str]:
    out: list[str] = []
    markers = [
        "host",
        "authority",
        "header",
        "protocol",
        "keep_alive",
        "http10",
        "http1",
        "config",
        "default",
        "timeout",
        "retry",
        "args_",
        "gzip",
        "bzip",
        "xz",
        "zstd",
        "lz4",
        "lzma",
        "connector",
        "verifier",
        "feature",
        "openssl",
        "rustls",
        "ffi",
        "headercasemap",
    ]
    for token in subject_tokens(subject):
        parts = token.split("::")
        for part in parts:
            lower = part.lower()
            if any(marker in lower for marker in markers) and part not in out:
                out.append(part)
    return out


def concurrency_tokens(subject: str) -> list[str]:
    out: list[str] = []
    markers = [
        "wake",
        "waker",
        "notify",
        "idle",
        "shutdown",
        "abort",
        "completed",
        "cancel",
        "pending",
        "ready",
        "poll",
        "lock",
        "condvar",
        "spawn",
        "join",
        "thread",
    ]
    for token in subject_tokens(subject):
        parts = token.split("::")
        for part in parts:
            lower = part.lower()
            if any(marker in lower for marker in markers) and part not in out:
                out.append(part)
    return out


def state_tokens(subject: str) -> list[str]:
    out: list[str] = []
    for token in subject_tokens(subject):
        parts = token.split("::")
        for part in parts:
            lower = part.lower()
            is_state_const = part.isupper() and any(
                marker in lower for marker in ["closed", "close", "state", "open"]
            )
            if (
                lower in {"closed", "close", "state", "open", "cancel", "shutdown", "upgrade", "pending", "ready", "reset"}
                or is_state_const
                or any(marker in lower for marker in ["closed", "close", "state", "cancel", "shutdown", "upgrade", "pending", "ready", "reset"])
            ):
                if part not in out:
                    out.append(part)
    return out


def analysis_contract(wrapper_type: str) -> dict[str, Any]:
    return {
        "consumer": "codex-cli",
        "task": "Produce semantic confidence evidence from this wrapper.",
        "wrapper_type": wrapper_type,
        "rules": [
            "Use only evidence and context in this wrapper.",
            "Treat primary_evidence as direct evidence.",
            "Treat supporting_evidence as supporting only.",
            "Treat context_evidence as background only.",
            "Do not claim a bug unless the wrapper contains explicit external confirmation.",
            "Mention uncertainty when diff, source, test, or history context is missing.",
        ],
        "expected_output": "semantic-confidence-evidence.v1",
    }


def pr_context(payload: dict[str, Any] | None) -> dict[str, Any]:
    if not payload:
        return {}
    return {
        "number": payload.get("number"),
        "title": payload.get("title", ""),
        "body": payload.get("body", ""),
        "url": payload.get("url", ""),
        "files": payload.get("files", []),
        "comments": payload.get("comments", []),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case-id", required=True)
    parser.add_argument("--decay-stdout", required=True)
    parser.add_argument("--diff-patch", required=True)
    parser.add_argument("--pr-json")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    pr_payload = None
    if args.pr_json and Path(args.pr_json).exists():
        pr_payload = json.loads(Path(args.pr_json).read_text())

    bundle = build_bundle(
        args.case_id,
        Path(args.decay_stdout).read_text(),
        Path(args.diff_patch).read_text(),
        pr_payload,
    )
    Path(args.out).write_text(json.dumps(bundle, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
