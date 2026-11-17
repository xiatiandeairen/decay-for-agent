#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
IN = ROOT / "semantic-candidates" / "scored.jsonl"
OUT = ROOT / "semantic-candidates" / "enriched.jsonl"


def load_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def pr_view(repo: str, pr: int) -> dict:
    cmd = [
        "gh",
        "pr",
        "view",
        str(pr),
        "--repo",
        repo,
        "--json",
        "number,title,url,state,baseRefName,baseRefOid,headRefName,headRefOid,labels,files,reviewDecision,comments,reviews,mergedAt,body",
    ]
    output = subprocess.check_output(cmd, text=True)
    return json.loads(output)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--in", dest="input", type=Path, default=IN)
    parser.add_argument("--out", type=Path, default=OUT)
    parser.add_argument("--limit", type=int, default=40)
    parser.add_argument("--min-score", type=int, default=8)
    args = parser.parse_args()

    rows = [row for row in load_jsonl(args.input) if int(row.get("risk_score", 0)) >= args.min_score]
    rows = rows[: args.limit]

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w") as handle:
        for row in rows:
            try:
                detail = pr_view(row["repo"], int(row["pr"]))
            except subprocess.CalledProcessError as err:
                print(f"warning: gh pr view failed {row['repo']}#{row['pr']}: {err}", file=sys.stderr)
                continue
            files = detail.get("files", [])
            comments = detail.get("comments", [])
            reviews = detail.get("reviews", [])
            enriched = {
                **row,
                "base_sha": detail.get("baseRefOid", ""),
                "head_sha": detail.get("headRefOid", ""),
                "review_decision": detail.get("reviewDecision", ""),
                "files_count": len(files),
                "files": [
                    {
                        "path": file.get("path", ""),
                        "additions": file.get("additions", 0),
                        "deletions": file.get("deletions", 0),
                    }
                    for file in files[:30]
                ],
                "review_count": len(reviews),
                "reviews": [
                    {
                        "author": review.get("author", {}).get("login", ""),
                        "state": review.get("state", ""),
                        "body": (review.get("body", "") or "")[:1200],
                    }
                    for review in reviews[:20]
                ],
                "comment_count_loaded": len(comments),
                "comments": [
                    {
                        "author": comment.get("author", {}).get("login", ""),
                        "body": (comment.get("body", "") or "")[:1200],
                    }
                    for comment in comments[:20]
                ],
            }
            handle.write(json.dumps(enriched, ensure_ascii=False, sort_keys=True) + "\n")

    print(f"wrote {args.out} candidates={len(rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
