#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path


SECTION_HEADERS = (
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


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: write-decay-evidence.py <decay.stdout> <out.md>", file=sys.stderr)
        return 2

    stdout = Path(sys.argv[1]).read_text()
    out = Path(sys.argv[2])
    evidence_count = len(re.findall(r"^- id=", stdout, flags=re.MULTILINE))

    lines = [
        "# Decay Evidence",
        "",
        f"- evidence_count: {evidence_count}",
        "",
    ]
    for header in SECTION_HEADERS:
        body = section(stdout, header)
        lines.extend([f"## {header.rstrip(':')}", "", "```text", body, "```", ""])

    out.write_text("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
