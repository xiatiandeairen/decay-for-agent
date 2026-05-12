#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: write-context.py <case-dir> <case-json> <pr-json>", file=sys.stderr)
        return 2

    case_dir = Path(sys.argv[1])
    case = json.loads(sys.argv[2])
    pr = json.loads(Path(sys.argv[3]).read_text())
    files = pr.get("files") or []
    labels = pr.get("labels") or []

    lines = [
        f"# {case['id']}",
        "",
        f"- repo: `{case['repo']}`",
        f"- pr: `{case['pr']}`",
        f"- kind: `{case.get('kind', '')}`",
        f"- status: `{case.get('status', '')}`",
        f"- url: {pr.get('url', '')}",
        f"- title: {pr.get('title', '')}",
        f"- merged_at: {pr.get('mergedAt', '')}",
        f"- review_decision: {pr.get('reviewDecision', '')}",
        f"- labels: {', '.join(label.get('name', '') for label in labels)}",
        "",
        "## Selection Reason",
        "",
        case.get("reason", "").strip() or "TODO",
        "",
        "## Changed Files",
        "",
    ]
    if files:
        for file in files:
            path = file.get("path") or file.get("filename") or ""
            additions = file.get("additions", "")
            deletions = file.get("deletions", "")
            lines.append(f"- `{path}` (+{additions}/-{deletions})")
    else:
        lines.append("TODO")

    lines.extend(
        [
            "",
            "## High-Value Gate",
            "",
            "- [ ] linked issue / bug report",
            "- [ ] review risk discussion",
            "- [ ] CI/test failure",
            "- [ ] follow-up fix",
            "- [ ] public API / protocol / storage / security / config / dependency sensitive change",
            "- [ ] negative control",
            "",
            "## Notes",
            "",
            "TODO: fill before writing ground truth.",
        ]
    )

    (case_dir / "context.md").write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
