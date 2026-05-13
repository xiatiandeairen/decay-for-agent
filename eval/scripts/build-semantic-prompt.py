#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path


SECTION_HEADERS = (
    "Diff context:",
    "Top risk findings:",
    "Complexity impact:",
    "Stability impact:",
    "Quality impact:",
    "Semantic impact:",
    "History impact:",
)


def section(stdout: str, header: str) -> str:
    start = stdout.find(header)
    if start < 0:
        return ""
    rest = stdout[start + len(header) :]
    next_positions = [rest.find(other) for other in SECTION_HEADERS if other != header]
    next_positions = [pos for pos in next_positions if pos >= 0]
    end = min(next_positions) if next_positions else len(rest)
    return rest[:end].strip()


def evidence_ids(stdout: str) -> list[int]:
    return [int(match) for match in re.findall(r"^- id=(\d+)\b", stdout, re.MULTILINE)]


def build_prompt(stdout: str) -> str:
    ids = evidence_ids(stdout)
    lines = [
        "# Task",
        "",
        "You are the semantic selection layer for decay v0.2.0 M3.1.",
        "Use only the fact evidence below. Do not use external APIs, web search, or unstated repository knowledge.",
        "",
        "# Output Contract",
        "",
        "Return JSON only, matching semantic-findings.v1:",
        "",
        "```json",
        '{"version":"semantic-findings.v1","findings":[]}',
        "```",
        "",
        "Each finding must include claim, supporting_facts, source_refs, confidence, limitation, and review_action.",
        "Every source_refs item must be an evidence id from the allowed list.",
        "Do not create a finding if it cannot cite at least one allowed evidence id.",
        "Do not promote weak-only signals: mixed_concerns, blast_radius, history-only, or pure touched_existing_risky.",
        "If the fact evidence is empty or weak-only, return an empty findings array.",
        "",
        "# Allowed Evidence Ids",
        "",
        ", ".join(str(item) for item in ids) if ids else "none",
        "",
    ]

    for header in SECTION_HEADERS:
        body = section(stdout, header)
        if not body:
            continue
        lines.extend([f"# {header.rstrip(':')}", "", "```text", body, "```", ""])

    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: build-semantic-prompt.py <decay.stdout> <out.md>", file=sys.stderr)
        return 2

    stdout = Path(sys.argv[1]).read_text()
    out = Path(sys.argv[2])
    out.write_text(build_prompt(stdout))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
