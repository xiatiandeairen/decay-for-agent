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
DEFAULT_TARGETS = ROOT / "risk-signals" / "follow-up-targets.toml"
DEFAULT_OUT = ROOT / "risk-signals" / "follow-up-raw"


def load_targets(path: Path) -> dict[str, list[dict[str, Any]]]:
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    return {case["id"]: case.get("refs", []) for case in data.get("cases", [])}


def gh_view(ref: dict[str, Any]) -> dict[str, Any]:
    kind = ref["type"]
    if kind == "issue":
        cmd = [
            "gh",
            "issue",
            "view",
            str(ref["number"]),
            "--repo",
            ref["repo"],
            "--json",
            "number,title,url,state,labels,body,comments,createdAt,closedAt",
        ]
    elif kind == "pr":
        cmd = [
            "gh",
            "pr",
            "view",
            str(ref["number"]),
            "--repo",
            ref["repo"],
            "--json",
            "number,title,url,state,baseRefName,baseRefOid,headRefName,headRefOid,labels,files,reviewDecision,comments,reviews,mergedAt,body",
        ]
    else:
        raise ValueError(f"unsupported ref type: {kind}")
    output = subprocess.check_output(cmd, text=True)
    return json.loads(output)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("case_ids", nargs="*")
    parser.add_argument("--targets", type=Path, default=DEFAULT_TARGETS)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()

    targets = load_targets(args.targets)
    case_ids = args.case_ids or sorted(targets)
    args.out.mkdir(parents=True, exist_ok=True)

    failed = 0
    for case_id in case_ids:
        refs = targets.get(case_id)
        if not refs:
            print(f"error: no follow-up refs for {case_id}", file=sys.stderr)
            failed += 1
            continue
        payload = {"case_id": case_id, "refs": []}
        for ref in refs:
            try:
                detail = gh_view(ref)
            except subprocess.CalledProcessError as err:
                print(f"error: gh view failed for {case_id} {ref}: {err}", file=sys.stderr)
                failed += 1
                continue
            payload["refs"].append({"target": ref, "detail": detail})
        if payload["refs"]:
            path = args.out / f"{case_id}.json"
            path.write_text(json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True))
            print(f"wrote {path}")

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
