#!/usr/bin/env python3
from __future__ import annotations

import re
from typing import Any


SECTION_HEADERS = (
    "Diff context:",
    "Top risk findings:",
    "Complexity impact:",
    "Coverage impact:",
    "Stability impact:",
    "Quality impact:",
    "Semantic impact:",
    "History impact:",
)

EVIDENCE_SECTION_CATEGORY = {
    "Complexity impact:": "complexity",
    "Coverage impact:": "coverage",
    "Stability impact:": "stability",
    "Quality impact:": "quality",
    "Semantic impact:": "semantic",
    "History impact:": "history",
}

ALLOWED_CONFIDENCE = {"high", "medium", "low"}
ALLOWED_VALUE = {"direct", "supporting", "context", "coverage_limit"}

EVIDENCE_RE = re.compile(
    r"^- id=(?P<id>\d+) category=(?P<category>\S+) kind=(?P<kind>\S+) "
    r"subject=(?P<subject>.*?) source=(?P<source>\S+) "
    r"confidence=(?P<confidence>\S+) value=(?P<value>\S+)$"
)

DETAIL_RE = re.compile(r"^  (?P<key>[a-z_]+)=(?P<value>.*)$")


class EvidenceContractError(ValueError):
    pass


def section(stdout: str, header: str) -> str:
    start = stdout.find(header)
    if start < 0:
        return ""
    rest = stdout[start + len(header) :]
    next_positions = [rest.find(other) for other in SECTION_HEADERS if other != header]
    next_positions = [pos for pos in next_positions if pos >= 0]
    end = min(next_positions) if next_positions else len(rest)
    return rest[:end].strip()


def parse_stdout(stdout: str) -> dict[str, Any]:
    report: dict[str, Any] = {
        "version": "decay-evidence.v1",
        "git_context": {},
        "sections": {
            header: {"summary": [], "evidence": []}
            for header in SECTION_HEADERS
        },
        "evidence": [],
        "diagnostics": [],
        "warnings": [],
        "_seen_headers": set(),
    }

    current_header: str | None = None
    current_item: dict[str, Any] | None = None
    seen_ids: set[int] = set()
    last_id = 0

    for line_no, raw_line in enumerate(stdout.splitlines(), start=1):
        line = raw_line.rstrip()
        if line in SECTION_HEADERS:
            current_header = line
            current_item = None
            report["_seen_headers"].add(line)
            continue
        if current_header is None:
            if line.strip():
                raise EvidenceContractError(
                    f"line {line_no}: content before first section header: {line}"
                )
            continue

        if line.startswith("diagnostic="):
            report["diagnostics"].append(line.split("=", 1)[1])
            continue
        if line.startswith("warning="):
            report["warnings"].append(line)
            continue
        if not line.strip():
            current_item = None
            continue

        if current_header == "Diff context:":
            match = DETAIL_RE.match(line)
            if not match:
                raise EvidenceContractError(
                    f"line {line_no}: invalid diff context entry: {line}"
                )
            key = match.group("key")
            value = match.group("value")
            if key == "changed_files":
                try:
                    report["git_context"][key] = int(value)
                except ValueError as err:
                    raise EvidenceContractError(
                        f"line {line_no}: changed_files must be an integer"
                    ) from err
            else:
                report["git_context"][key] = value
            continue

        if current_header == "Top risk findings:":
            report["sections"][current_header]["summary"].append(line)
            continue

        match = EVIDENCE_RE.match(line)
        if match:
            item = {
                "id": int(match.group("id")),
                "category": match.group("category"),
                "kind": match.group("kind"),
                "subject": match.group("subject"),
                "source": match.group("source"),
                "confidence": match.group("confidence"),
                "value": match.group("value"),
                "details": {},
            }
            expected_category = EVIDENCE_SECTION_CATEGORY[current_header]
            validate_evidence_item(item, expected_category, line_no, last_id, seen_ids)
            last_id = item["id"]
            seen_ids.add(item["id"])
            report["sections"][current_header]["evidence"].append(item)
            report["evidence"].append(item)
            current_item = item
            continue
        if line.startswith("- id="):
            raise EvidenceContractError(
                f"line {line_no}: invalid evidence line, confidence/value required"
            )

        detail = DETAIL_RE.match(line)
        if detail:
            key = detail.group("key")
            value = detail.group("value")
            if current_item is None:
                report["sections"][current_header]["summary"].append(line)
                continue
            current_item["details"][key] = value
            if key == "file":
                current_item["file"] = value
            elif key == "function":
                current_item["function"] = value
                if "file" not in current_item:
                    current_item["file"] = value.split(":", 1)[0]
            continue

        report["sections"][current_header]["summary"].append(line)

    ensure_required_headers(report)
    ensure_detail_contract(report["evidence"])
    report.pop("_seen_headers", None)
    return report


def evidence_ids(stdout: str) -> list[int]:
    return [item["id"] for item in parse_stdout(stdout)["evidence"]]


def validate_evidence_item(
    item: dict[str, Any],
    expected_category: str,
    line_no: int,
    last_id: int,
    seen_ids: set[int],
) -> None:
    if item["category"] != expected_category:
        raise EvidenceContractError(
            f"line {line_no}: evidence category {item['category']} does not match section {expected_category}"
        )
    if item["id"] in seen_ids:
        raise EvidenceContractError(f"line {line_no}: duplicate evidence id {item['id']}")
    if item["id"] != last_id + 1:
        raise EvidenceContractError(
            f"line {line_no}: evidence ids must be contiguous, expected {last_id + 1} got {item['id']}"
        )
    if item["confidence"] not in ALLOWED_CONFIDENCE:
        raise EvidenceContractError(
            f"line {line_no}: invalid confidence {item['confidence']}"
        )
    if item["value"] not in ALLOWED_VALUE:
        raise EvidenceContractError(f"line {line_no}: invalid value {item['value']}")
    validate_category_value(item, line_no)


def validate_category_value(item: dict[str, Any], line_no: int) -> None:
    category = item["category"]
    value = item["value"]
    confidence = item["confidence"]
    if category == "coverage":
        if confidence != "high" or value != "coverage_limit":
            raise EvidenceContractError(
                f"line {line_no}: coverage evidence must be confidence=high value=coverage_limit"
            )
        return
    if category == "history":
        if confidence != "low" or value != "context":
            raise EvidenceContractError(
                f"line {line_no}: history evidence must be confidence=low value=context"
            )
        return
    if category in {"stability", "semantic"} and value != "direct":
        raise EvidenceContractError(
            f"line {line_no}: {category} evidence must use value=direct"
        )
        return
    if category == "quality" and value not in {"supporting", "context"}:
        raise EvidenceContractError(
            f"line {line_no}: quality evidence must use supporting/context value"
        )
        return
    if category == "complexity" and value not in {"direct", "context"}:
        raise EvidenceContractError(
            f"line {line_no}: complexity evidence must use direct/context value"
        )


def ensure_required_headers(report: dict[str, Any]) -> None:
    seen_headers = report.get("_seen_headers", set())
    missing = [header for header in SECTION_HEADERS if header not in seen_headers]
    if missing:
        raise EvidenceContractError(f"missing section headers: {', '.join(missing)}")
    if report["git_context"].get("mode") != "git_ref":
        raise EvidenceContractError("Diff context.mode must be git_ref")


def ensure_detail_contract(evidence: list[dict[str, Any]]) -> None:
    for item in evidence:
        if item["category"] == "complexity":
            if "function" not in item:
                raise EvidenceContractError(
                    f"evidence id={item['id']} missing function detail"
                )
        elif "file" not in item:
            raise EvidenceContractError(f"evidence id={item['id']} missing file detail")
