#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: toml_case.py <corpus.toml> <case-id>", file=sys.stderr)
        return 2

    corpus = Path(sys.argv[1])
    case_id = sys.argv[2]
    with corpus.open("rb") as handle:
        data = tomllib.load(handle)

    for case in data.get("cases", []):
        if case.get("id") == case_id:
            print(json.dumps(case))
            return 0

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
