#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
IN = ROOT / "semantic-candidates" / "pool.jsonl"
OUT = ROOT / "semantic-candidates" / "scored.jsonl"

HIGH_SIGNAL = {
    "regression": 4,
    "panic": 4,
    "security": 4,
    "compat": 3,
    "breaking": 3,
    "protocol": 3,
    "api": 3,
    "default": 2,
    "config": 2,
    "feature": 1,
    "bug": 2,
    "fix": 2,
}

CONTEXT_PATTERNS = {
    "review_required": re.compile(r"\b(review|concern|should|must|unsafe|compat|regression|panic|security)\b", re.I),
    "tests": re.compile(r"\b(test|coverage|regression test)\b", re.I),
    "behavior": re.compile(r"\b(behavior|default|config|protocol|api|public|breaking|compat)\b", re.I),
    "failure_mode": re.compile(r"\b(panic|hang|deadlock|overflow|invalid|incorrect|wrong|missing|fail|error|leak|race)\b", re.I),
    "issue_linked": re.compile(r"\b(fixes|closes|resolves)\s+#\d+\b|github\.com/.+/(issues|pull)/\d+", re.I),
}

LOW_VALUE = re.compile(r"\b(doc|docs|typo|readme|format|fmt|style|lint)\b", re.I)
AUTOMATION_DEP = re.compile(
    r"\b(dependabot|renovate|build\(deps\)|chore\(deps\)|bump .+ from .+ to|dependencies|github_actions)\b",
    re.I,
)
EXPLORATORY = re.compile(r"\b(wip|draft|experiment|prototype|exploratory)\b", re.I)
HIGH_VALUE_TITLE = re.compile(
    r"\b(fix|revert|regression|panic|deadlock|soundness|security|compat|breaking|incorrect|invalid|leak|race)\b",
    re.I,
)


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    if not path.exists():
        return rows
    for line in path.read_text().splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def score(row: dict) -> tuple[int, list[str]]:
    text = " ".join(
        [
            row.get("title", ""),
            row.get("body", ""),
            " ".join(row.get("labels", [])),
            " ".join(row.get("seed_keywords", [])),
        ]
    )
    reasons: list[str] = []
    total = 0

    for keyword, value in HIGH_SIGNAL.items():
        if re.search(rf"\b{re.escape(keyword)}\b", text, re.I):
            total += value
            reasons.append(f"+{value} keyword:{keyword}")

    comments = int(row.get("comments_count") or 0)
    if comments >= 8:
        total += 3
        reasons.append("+3 many_comments")
    elif comments >= 3:
        total += 1
        reasons.append("+1 some_comments")

    for name, pattern in CONTEXT_PATTERNS.items():
        if pattern.search(text):
            total += 2
            reasons.append(f"+2 context:{name}")

    title = row.get("title", "")
    if HIGH_VALUE_TITLE.search(title):
        total += 3
        reasons.append("+3 high_value_title")

    if AUTOMATION_DEP.search(text):
        total -= 12
        reasons.append("-12 automation_dependency")

    if EXPLORATORY.search(title):
        total -= 6
        reasons.append("-6 exploratory_title")

    if LOW_VALUE.search(text):
        total -= 6
        reasons.append("-6 low_value_keyword")

    return total, reasons


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--in", dest="input", type=Path, default=IN)
    parser.add_argument("--out", type=Path, default=OUT)
    args = parser.parse_args()

    rows = load_jsonl(args.input)
    scored = []
    for row in rows:
        total, reasons = score(row)
        scored.append({**row, "risk_score": total, "score_reasons": reasons})

    scored.sort(key=lambda item: (-item["risk_score"], item["repo"], item["pr"]))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w") as handle:
        for row in scored:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")

    print(f"wrote {args.out} candidates={len(scored)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
