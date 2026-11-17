#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "semantic-candidates" / "pool.jsonl"

DEFAULT_REPOS = [
    "hyperium/hyper",
    "BurntSushi/ripgrep",
    "tokio-rs/tokio",
    "rust-lang/cargo",
    "clap-rs/clap",
    "serde-rs/json",
]

KEYWORDS = [
    "bug",
    "fix",
    "regression",
    "panic",
    "compat",
    "api",
    "protocol",
    "breaking",
    "security",
    "default",
    "config",
    "feature",
]


def gh_search(repo: str, keyword: str, limit: int) -> list[dict]:
    cmd = [
        "gh",
        "search",
        "prs",
        keyword,
        "--repo",
        repo,
        "--merged",
        "--limit",
        str(limit),
        "--json",
        "number,title,url,repository,closedAt,labels,commentsCount,body",
    ]
    output = subprocess.check_output(cmd, text=True)
    return json.loads(output)


def normalized(row: dict, keyword: str) -> dict:
    repo = row.get("repository", {}).get("nameWithOwner", "")
    labels = [label.get("name", "") for label in row.get("labels", [])]
    return {
        "case_id": f"{repo.replace('/', '-')}-{row.get('number')}",
        "repo": repo,
        "pr": row.get("number"),
        "title": row.get("title", ""),
        "url": row.get("url", ""),
        "closed_at": row.get("closedAt", ""),
        "labels": labels,
        "comments_count": row.get("commentsCount", 0),
        "body": row.get("body", "") or "",
        "seed_keywords": [keyword],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", action="append", dest="repos", default=[])
    parser.add_argument("--limit-per-query", type=int, default=15)
    parser.add_argument("--out", type=Path, default=OUT)
    args = parser.parse_args()

    repos = args.repos or DEFAULT_REPOS
    by_key: dict[tuple[str, int], dict] = {}

    for repo in repos:
        for keyword in KEYWORDS:
            try:
                rows = gh_search(repo, keyword, args.limit_per_query)
            except subprocess.CalledProcessError as err:
                print(f"warning: gh search failed repo={repo} keyword={keyword}: {err}", file=sys.stderr)
                continue
            for row in rows:
                candidate = normalized(row, keyword)
                key = (candidate["repo"], int(candidate["pr"]))
                if key in by_key:
                    by_key[key]["seed_keywords"].append(keyword)
                else:
                    by_key[key] = candidate

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w") as handle:
        for candidate in sorted(by_key.values(), key=lambda item: (item["repo"], item["pr"])):
            candidate["seed_keywords"] = sorted(set(candidate["seed_keywords"]))
            handle.write(json.dumps(candidate, ensure_ascii=False, sort_keys=True) + "\n")

    print(f"wrote {args.out} candidates={len(by_key)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
