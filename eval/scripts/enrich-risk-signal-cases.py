#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CANDIDATES = ROOT / "risk-signals" / "eval-candidates.toml"
DEFAULT_OUT = ROOT / "risk-signals" / "raw"


def load_candidates(path: Path) -> dict[str, dict[str, Any]]:
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    return {case["id"]: case for case in data.get("cases", [])}


def gh_pr_view(repo: str, pr_number: str) -> dict[str, Any]:
    cmd = [
        "gh",
        "pr",
        "view",
        pr_number,
        "--repo",
        repo,
        "--json",
        "number,title,url,state,baseRefName,baseRefOid,headRefName,headRefOid,labels,files,reviewDecision,comments,reviews,mergedAt,body",
    ]
    output = subprocess.check_output(cmd, text=True)
    return json.loads(output)


def pr_number_from_url(url: str) -> str:
    return url.rstrip("/").split("/")[-1]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("case_ids", nargs="+")
    parser.add_argument("--candidates", type=Path, default=DEFAULT_CANDIDATES)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()

    candidates = load_candidates(args.candidates)
    args.out.mkdir(parents=True, exist_ok=True)

    failed = 0
    for case_id in args.case_ids:
        candidate = candidates.get(case_id)
        if not candidate:
            print(f"error: unknown case {case_id}", file=sys.stderr)
            failed += 1
            continue
        try:
            detail = gh_pr_view(candidate["repo"], pr_number_from_url(candidate["url"]))
        except subprocess.CalledProcessError as err:
            print(f"error: gh pr view failed for {case_id}: {err}", file=sys.stderr)
            failed += 1
            continue
        payload = {
            "candidate": candidate,
            "pr": detail,
        }
        (args.out / f"{case_id}.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True))
        print(f"wrote {args.out / f'{case_id}.json'}")

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
