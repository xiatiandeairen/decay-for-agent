#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "results"
JUDGEMENTS = ROOT / "judgements"
REPORT = ROOT / "reports" / "summary.md"


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def evidence_count(stdout: str) -> int:
    return len(re.findall(r"^- id=", stdout, flags=re.MULTILINE))


def top3_ids(stdout: str) -> set[int]:
    ids = []
    for match in re.finditer(r"^- id=(\d+)", stdout, flags=re.MULTILINE):
        ids.append(int(match.group(1)))
        if len(ids) == 3:
            break
    return set(ids)


def result_rows() -> list[dict]:
    rows = []
    if not RESULTS.exists():
        return rows
    for run_path in sorted(RESULTS.glob("*/run.toml")):
        run = load_toml(run_path)
        stdout_path = run_path.parent / "diff.stdout"
        stdout = stdout_path.read_text() if stdout_path.exists() else ""
        rows.append(
            {
                **run,
                "case_id": run_path.parent.name,
                "evidence_count": evidence_count(stdout),
                "top3": top3_ids(stdout),
            }
        )
    return rows


def judgement_rows() -> dict[str, dict]:
    rows = {}
    for path in sorted(JUDGEMENTS.glob("*.toml")):
        if path.name == "template.toml":
            continue
        data = load_toml(path)
        case_id = data.get("case_id")
        if case_id:
            rows[case_id] = data
    return rows


def main() -> int:
    results = result_rows()
    judgements = judgement_rows()

    total_cases = len(results)
    useful = plausible = noise = missed = 0
    top3_total = top3_useful = 0
    actionable_cases = 0
    negative_cases = negative_silent = 0
    missed_cases = 0

    lines = [
        "# External PR Validation Summary",
        "",
        f"- cases_run: {total_cases}",
        f"- cases_judged: {len(judgements)}",
    ]

    for row in results:
        case_id = row["case_id"]
        judgement = judgements.get(case_id)
        if row.get("kind") == "negative":
            negative_cases += 1
            if row["evidence_count"] == 0:
                negative_silent += 1
        if not judgement:
            continue

        case_useful = 0
        for item in judgement.get("evidence", []):
            verdict = item.get("judgement")
            if verdict == "useful":
                useful += 1
                case_useful += 1
            elif verdict == "plausible":
                plausible += 1
            elif verdict == "noise":
                noise += 1

            evidence_id = item.get("id")
            if evidence_id in row["top3"]:
                top3_total += 1
                if verdict == "useful":
                    top3_useful += 1

        case_missed = len(judgement.get("missed", []))
        missed += case_missed
        if case_missed:
            missed_cases += 1
        if case_useful:
            actionable_cases += 1

    judged_cases = max(len(judgements), 1)
    judged_evidence = useful + plausible + noise
    precision_top3 = top3_useful / top3_total if top3_total else 0.0
    actionable_pr_rate = actionable_cases / judged_cases
    negative_control_silence_rate = (
        negative_silent / negative_cases if negative_cases else 0.0
    )
    noise_rate = noise / judged_evidence if judged_evidence else 0.0
    missed_high_risk_rate = missed_cases / judged_cases

    lines.extend(
        [
            f"- useful: {useful}",
            f"- plausible: {plausible}",
            f"- noise: {noise}",
            f"- missed: {missed}",
            f"- precision_at_top3: {precision_top3:.2%}",
            f"- actionable_pr_rate: {actionable_pr_rate:.2%}",
            f"- negative_control_silence_rate: {negative_control_silence_rate:.2%}",
            f"- noise_rate: {noise_rate:.2%}",
            f"- missed_high_risk_rate: {missed_high_risk_rate:.2%}",
            "",
            "## Cases",
            "",
            "| Case | Kind | Evidence | Diff Exit | Judged |",
            "|---|---:|---:|---:|---|",
        ]
    )

    for row in results:
        case_id = row["case_id"]
        judged = "yes" if case_id in judgements else "no"
        lines.append(
            f"| {case_id} | {row.get('kind', '')} | {row['evidence_count']} | {row.get('diff_exit', '')} | {judged} |"
        )

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text("\n".join(lines) + "\n")
    print(f"wrote {REPORT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

