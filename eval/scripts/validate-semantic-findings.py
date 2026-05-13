#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from pathlib import Path


REQUIRED_FINDING_KEYS = {
    "claim",
    "supporting_facts",
    "source_refs",
    "confidence",
    "limitation",
    "review_action",
}
ALLOWED_CONFIDENCE = {"high", "medium", "low"}


def evidence_ids(stdout: str) -> set[int]:
    return {int(match) for match in re.findall(r"^- id=(\d+)\b", stdout, re.MULTILINE)}


def load_json(path: Path) -> dict:
    text = path.read_text().strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError as err:
        raise ValueError(f"invalid json: {err}") from err


def require_non_empty_string(value: object, path: str) -> None:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{path} must be a non-empty string")


def validate(stdout: str, payload: dict) -> None:
    allowed_ids = evidence_ids(stdout)

    if payload.get("version") != "semantic-findings.v1":
        raise ValueError("version must be semantic-findings.v1")
    findings = payload.get("findings")
    if not isinstance(findings, list):
        raise ValueError("findings must be a list")

    for idx, finding in enumerate(findings):
        path = f"findings[{idx}]"
        if not isinstance(finding, dict):
            raise ValueError(f"{path} must be an object")
        extra = set(finding) - REQUIRED_FINDING_KEYS
        missing = REQUIRED_FINDING_KEYS - set(finding)
        if extra:
            raise ValueError(f"{path} has unknown keys: {sorted(extra)}")
        if missing:
            raise ValueError(f"{path} missing keys: {sorted(missing)}")

        require_non_empty_string(finding["claim"], f"{path}.claim")
        require_non_empty_string(finding["limitation"], f"{path}.limitation")
        require_non_empty_string(finding["review_action"], f"{path}.review_action")
        if finding["confidence"] not in ALLOWED_CONFIDENCE:
            raise ValueError(f"{path}.confidence must be high, medium, or low")

        facts = finding["supporting_facts"]
        if not isinstance(facts, list) or not facts:
            raise ValueError(f"{path}.supporting_facts must be a non-empty list")
        for fact_idx, fact in enumerate(facts):
            require_non_empty_string(fact, f"{path}.supporting_facts[{fact_idx}]")

        refs = finding["source_refs"]
        if not isinstance(refs, list) or not refs:
            raise ValueError(f"{path}.source_refs must be a non-empty list")
        for ref_idx, ref in enumerate(refs):
            ref_path = f"{path}.source_refs[{ref_idx}]"
            if not isinstance(ref, dict):
                raise ValueError(f"{ref_path} must be an object")
            if set(ref) != {"type", "id"}:
                raise ValueError(f"{ref_path} must contain exactly type and id")
            if ref["type"] != "evidence":
                raise ValueError(f"{ref_path}.type must be evidence")
            if not isinstance(ref["id"], int):
                raise ValueError(f"{ref_path}.id must be an integer")
            if ref["id"] not in allowed_ids:
                raise ValueError(f"{ref_path}.id {ref['id']} is not in fact evidence")


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: validate-semantic-findings.py <decay.stdout> <semantic-findings.json>", file=sys.stderr)
        return 2

    try:
        stdout = Path(sys.argv[1]).read_text()
        payload = load_json(Path(sys.argv[2]))
        validate(stdout, payload)
    except ValueError as err:
        print(f"semantic findings invalid: {err}", file=sys.stderr)
        return 1

    print("semantic findings valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
