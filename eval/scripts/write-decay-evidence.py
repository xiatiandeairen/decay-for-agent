#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from decay_evidence import SECTION_HEADERS, evidence_ids, section


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: write-decay-evidence.py <decay.stdout> <out.md>", file=sys.stderr)
        return 2

    stdout = Path(sys.argv[1]).read_text()
    out = Path(sys.argv[2])
    evidence_count = len(evidence_ids(stdout))

    lines = [
        "# Decay Evidence",
        "",
        f"- evidence_count: {evidence_count}",
        "",
    ]
    for header in SECTION_HEADERS[1:]:
        body = section(stdout, header)
        lines.extend([f"## {header.rstrip(':')}", "", "```text", body, "```", ""])

    out.write_text("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
