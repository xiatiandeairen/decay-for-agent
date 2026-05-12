#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


RISK_MARKERS = (
    "fix",
    "bug",
    "panic",
    "security",
    "unsafe",
    "api",
    "breaking",
    "protocol",
    "config",
    "migration",
    "serde",
    "tls",
    "http",
)


def gh_json(args: list[str]) -> list[dict]:
    output = subprocess.check_output(args, text=True)
    return json.loads(output or "[]")


def score(pr: dict) -> tuple[int, list[str]]:
    title = (pr.get("title") or "").lower()
    labels = " ".join(label.get("name", "") for label in pr.get("labels", [])).lower()
    text = f"{title} {labels}"
    reasons = []
    total = 0
    for marker in RISK_MARKERS:
        if marker in text:
            total += 2
            reasons.append(f"title_or_label:{marker}")
    comments = pr.get("commentsCount") or 0
    if comments >= 5:
        total += 2
        reasons.append("review_discussion")
    if pr.get("closedAt"):
        total += 1
        reasons.append("merged")
    return total, reasons


def write_candidates(repo: str, prs: list[dict], limit: int) -> Path:
    owner, name = repo.split("/", 1)
    out = ROOT / "candidates" / f"{owner}-{name}.toml"
    out.parent.mkdir(parents=True, exist_ok=True)
    rows = []
    for pr in prs[:limit]:
        case_id = f"{name}-{pr['number']}"
        total, reasons = score(pr)
        rows.extend(
            [
                "[[candidates]]",
                f'id = "{case_id}"',
                f'repo = "{repo}"',
                f"pr = {pr['number']}",
                f'score = {total}',
                f'title = "{toml_quote(pr.get("title", ""))}"',
                f'url = "{pr.get("url", "")}"',
                f'reasons = [{", ".join(repr_toml(reason) for reason in reasons)}]',
                "",
            ]
        )
    out.write_text("\n".join(rows))
    return out


def toml_quote(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def repr_toml(value: str) -> str:
    return f'"{toml_quote(value)}"'


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("repo", help="owner/name")
    parser.add_argument("--limit", type=int, default=30)
    parser.add_argument("--state", default="merged")
    args = parser.parse_args()

    prs = gh_json(
        [
            "gh",
            "search",
            "prs",
            f"repo:{args.repo}",
            "is:pr",
            f"is:{args.state}",
            "--limit",
            str(args.limit),
            "--json",
            "number,title,url,labels,commentsCount,closedAt",
        ]
    )
    ranked = sorted(prs, key=lambda pr: score(pr)[0], reverse=True)
    out = write_candidates(args.repo, ranked, args.limit)
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
