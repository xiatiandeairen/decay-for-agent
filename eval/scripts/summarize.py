#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "cases"
RESULTS = ROOT / "results"
JUDGEMENTS = ROOT / "judgements"
REPORT = ROOT / "reports" / "50-case-summary.md"


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def evidence_count(stdout: str) -> int:
    return len(re.findall(r"^- id=", stdout, flags=re.MULTILINE))


def top_risk_count(stdout: str) -> int:
    section = stdout.split("Top risk findings:", 1)[-1].split("Complexity impact:", 1)[0]
    return len(re.findall(r"^\d+\. \[", section, flags=re.MULTILINE))


def parse_metrics(path: Path) -> dict:
    if not path.exists():
        return {}
    text = path.read_text()
    match = re.search(r"```toml\n(.*?)```", text, flags=re.DOTALL)
    if not match:
        return {}
    return tomllib.loads(match.group(1))


def new_case_rows() -> list[dict]:
    rows = []
    if not CASES.exists():
        return rows
    for metadata_path in sorted(CASES.glob("*/metadata.toml")):
        case_dir = metadata_path.parent
        metadata = load_toml(metadata_path)
        stdout_path = case_dir / "raw" / "decay.stdout"
        stdout = stdout_path.read_text() if stdout_path.exists() else ""
        comparison = parse_metrics(case_dir / "comparison.md")
        rows.append(
            {
                **metadata,
                "case_id": metadata.get("case_id", case_dir.name),
                "evidence_count": evidence_count(stdout),
                "top_risk_count": top_risk_count(stdout),
                "comparison": comparison,
                "source": "cases",
            }
        )
    return rows


def legacy_result_rows() -> list[dict]:
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
                "top_risk_count": top_risk_count(stdout),
                "comparison": {},
                "source": "legacy-results",
            }
        )
    return rows


def legacy_judgements() -> dict[str, dict]:
    rows = {}
    for path in sorted(JUDGEMENTS.glob("*.toml")):
        if path.name == "template.toml":
            continue
        data = load_toml(path)
        case_id = data.get("case_id")
        if case_id:
            rows[case_id] = data
    return rows


def apply_legacy_metrics(row: dict, judgement: dict) -> dict:
    useful = plausible = noise = missed = top3_useful = top3_total = 0
    for item in judgement.get("evidence", []):
        verdict = item.get("judgement")
        if verdict == "useful":
            useful += 1
        elif verdict == "plausible":
            plausible += 1
        elif verdict == "noise":
            noise += 1
        if item.get("id") in {1, 2, 3}:
            top3_total += 1
            if verdict == "useful":
                top3_useful += 1
    missed = len(judgement.get("missed", []))
    return {
        "top3_total": top3_total,
        "top3_useful": top3_useful,
        "actionable": useful > 0,
        "negative_control": row.get("kind") == "negative",
        "negative_silent": row.get("kind") == "negative" and row.get("evidence_count", 0) == 0,
        "missed_high_risk_count": missed,
        "useful_evidence_count": useful,
        "plausible_evidence_count": plausible,
        "noise_evidence_count": noise,
        "noise_top3_count": 0,
        "history_only_false_positive_count": 0,
    }


def bool_metric(metrics: dict, key: str) -> bool:
    return bool(metrics.get(key, False))


def int_metric(metrics: dict, key: str) -> int:
    value = metrics.get(key, 0)
    return value if isinstance(value, int) else 0


def main() -> int:
    rows = new_case_rows()
    legacy = legacy_result_rows()
    legacy_by_id = {row["case_id"]: row for row in legacy}
    existing_ids = {row["case_id"] for row in rows}
    rows.extend(row for case_id, row in legacy_by_id.items() if case_id not in existing_ids)

    judgements = legacy_judgements()
    for row in rows:
        if not row.get("comparison") and row["case_id"] in judgements:
            row["comparison"] = apply_legacy_metrics(row, judgements[row["case_id"]])

    judged = [row for row in rows if row.get("comparison")]
    total_cases = len(rows)
    judged_cases = len(judged)
    useful = sum(int_metric(row["comparison"], "useful_evidence_count") for row in judged)
    plausible = sum(int_metric(row["comparison"], "plausible_evidence_count") for row in judged)
    noise = sum(int_metric(row["comparison"], "noise_evidence_count") for row in judged)
    missed = sum(int_metric(row["comparison"], "missed_high_risk_count") for row in judged)
    top3_total = sum(int_metric(row["comparison"], "top3_total") for row in judged)
    top3_useful = sum(int_metric(row["comparison"], "top3_useful") for row in judged)
    noise_top3 = sum(int_metric(row["comparison"], "noise_top3_count") for row in judged)
    history_only_noise = sum(
        int_metric(row["comparison"], "history_only_false_positive_count") for row in judged
    )
    actionable = sum(1 for row in judged if bool_metric(row["comparison"], "actionable"))
    negative = [row for row in judged if bool_metric(row["comparison"], "negative_control")]
    negative_silent = sum(1 for row in negative if bool_metric(row["comparison"], "negative_silent"))

    precision_top3 = top3_useful / top3_total if top3_total else 0.0
    actionable_rate = actionable / judged_cases if judged_cases else 0.0
    negative_silence_rate = negative_silent / len(negative) if negative else 0.0

    lines = [
        "# 50-Case Risk Validation Summary",
        "",
        f"- target_cases: 50",
        f"- cases_run: {total_cases}",
        f"- cases_judged: {judged_cases}",
        f"- useful_evidence_count: {useful}",
        f"- plausible_evidence_count: {plausible}",
        f"- noise_evidence_count: {noise}",
        f"- missed_high_risk_count: {missed}",
        f"- precision_at_top3: {precision_top3:.2%}",
        f"- actionable_pr_rate: {actionable_rate:.2%}",
        f"- negative_control_silence_rate: {negative_silence_rate:.2%}",
        f"- noise_top3_count: {noise_top3}",
        f"- history_only_false_positive_count: {history_only_noise}",
        "",
        "## Cases",
        "",
        "| Case | Repo | Kind | Source | Evidence | Top Risks | Judged |",
        "|---|---|---:|---|---:|---:|---|",
    ]
    for row in rows:
        lines.append(
            "| {case_id} | {repo} | {kind} | {source} | {evidence_count} | {top_risk_count} | {judged} |".format(
                case_id=row.get("case_id", ""),
                repo=row.get("repo", ""),
                kind=row.get("kind", ""),
                source=row.get("source", ""),
                evidence_count=row.get("evidence_count", 0),
                top_risk_count=row.get("top_risk_count", 0),
                judged="yes" if row.get("comparison") else "no",
            )
        )

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text("\n".join(lines) + "\n")
    print(f"wrote {REPORT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
