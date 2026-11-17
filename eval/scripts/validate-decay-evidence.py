#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from decay_evidence import EvidenceContractError, parse_stdout


def main() -> int:
    if len(sys.argv) not in {2, 3}:
        print(
            "usage: validate-decay-evidence.py <decay.stdout> [normalized.json]",
            file=sys.stderr,
        )
        return 2

    stdout_path = Path(sys.argv[1])
    try:
        report = parse_stdout(stdout_path.read_text())
    except EvidenceContractError as err:
        print(f"decay evidence invalid: {err}", file=sys.stderr)
        return 1

    if len(sys.argv) == 3:
        Path(sys.argv[2]).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")

    print("decay evidence valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
