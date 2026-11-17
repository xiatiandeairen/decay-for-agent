#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CORPUS = ROOT / "corpus" / "risk-signal-frozen-v1.toml"
DEFAULT_BENCHMARK = ROOT / "benchmarks" / "frozen-v1"
DEFAULT_REPORT = ROOT / "reports" / "frozen-v1-benchmark.md"


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def corpus_cases(path: Path) -> list[dict[str, Any]]:
    data = load_toml(path)
    cases = data.get("cases", [])
    if not isinstance(cases, list):
        raise ValueError(f"{path}: cases must be a list")
    return cases


def bool_value(data: dict[str, Any], key: str) -> bool:
    return bool(data.get(key, False))


def int_value(data: dict[str, Any], key: str) -> int:
    value = data.get(key, 0)
    return value if isinstance(value, int) else 0


def load_judgement(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    data = load_toml(path)
    if data.get("status") != "complete":
        return None
    return data


def rate(numerator: int, denominator: int) -> float:
    return numerator / denominator if denominator else 0.0


def summarize(cases: list[dict[str, Any]], benchmark_dir: Path) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    for case in cases:
        case_id = str(case["id"])
        judgement = load_judgement(benchmark_dir / "judgements" / f"{case_id}.toml")
        row = {"case": case, "judgement": judgement}
        rows.append(row)

    judged = [row for row in rows if row["judgement"]]
    pending = [row for row in rows if not row["judgement"]]

    def mode_sum(mode: str, key: str) -> int:
        return sum(int_value(row["judgement"].get(mode, {}), key) for row in judged)

    def mode_bool_count(mode: str, key: str) -> int:
        return sum(1 for row in judged if bool_value(row["judgement"].get(mode, {}), key))

    fact_top3_total = mode_sum("fact_only", "top3_total")
    fact_top3_useful = mode_sum("fact_only", "top3_useful")
    sem_top3_total = mode_sum("semantic_filtered", "top3_total")
    sem_top3_useful = mode_sum("semantic_filtered", "top3_useful")

    metrics = {
        "target_cases": len(cases),
        "judged_cases": len(judged),
        "pending_cases": len(pending),
        "fact_precision_at_top3": rate(fact_top3_useful, fact_top3_total),
        "semantic_precision_at_top3": rate(sem_top3_useful, sem_top3_total),
        "fact_actionable_pr_rate": rate(mode_bool_count("fact_only", "actionable"), len(judged)),
        "semantic_actionable_pr_rate": rate(mode_bool_count("semantic_filtered", "actionable"), len(judged)),
        "fact_missed_high_risk_count": mode_sum("fact_only", "missed_high_risk_count"),
        "semantic_missed_high_risk_count": mode_sum("semantic_filtered", "missed_high_risk_count"),
        "fact_weak_only_top_risk_count": mode_sum("fact_only", "weak_only_top_risk_count"),
        "semantic_weak_only_top_risk_count": mode_sum("semantic_filtered", "weak_only_top_risk_count"),
        "semantic_improved_ranking_count": sum(
            1 for row in judged if bool_value(row["judgement"].get("comparison", {}), "semantic_improved_ranking")
        ),
        "semantic_reduced_noise_count": sum(
            1 for row in judged if bool_value(row["judgement"].get("comparison", {}), "semantic_reduced_noise")
        ),
        "semantic_introduced_noise_count": sum(
            1 for row in judged if bool_value(row["judgement"].get("comparison", {}), "semantic_introduced_noise")
        ),
    }
    return {"rows": rows, "metrics": metrics}


def pct(value: float) -> str:
    return f"{value:.2%}"


def write_report(summary: dict[str, Any], report: Path) -> None:
    metrics = summary["metrics"]
    rows = summary["rows"]

    lines = [
        "# Frozen v1 Benchmark Report",
        "",
        "This report compares fact-only decay output with semantic-filtered output on the fixed 40-case frozen eval v1 set.",
        "",
        "## Metrics",
        "",
        f"- target_cases: {metrics['target_cases']}",
        f"- judged_cases: {metrics['judged_cases']}",
        f"- pending_cases: {metrics['pending_cases']}",
        f"- fact_precision_at_top3: {pct(metrics['fact_precision_at_top3'])}",
        f"- semantic_precision_at_top3: {pct(metrics['semantic_precision_at_top3'])}",
        f"- fact_actionable_pr_rate: {pct(metrics['fact_actionable_pr_rate'])}",
        f"- semantic_actionable_pr_rate: {pct(metrics['semantic_actionable_pr_rate'])}",
        f"- fact_missed_high_risk_count: {metrics['fact_missed_high_risk_count']}",
        f"- semantic_missed_high_risk_count: {metrics['semantic_missed_high_risk_count']}",
        f"- fact_weak_only_top_risk_count: {metrics['fact_weak_only_top_risk_count']}",
        f"- semantic_weak_only_top_risk_count: {metrics['semantic_weak_only_top_risk_count']}",
        f"- semantic_improved_ranking_count: {metrics['semantic_improved_ranking_count']}",
        f"- semantic_reduced_noise_count: {metrics['semantic_reduced_noise_count']}",
        f"- semantic_introduced_noise_count: {metrics['semantic_introduced_noise_count']}",
        "",
        "## Cases",
        "",
        "| Case | Repo | PR | Judged | Fact actionable | Semantic actionable | Fact missed | Semantic missed |",
        "| --- | --- | ---: | --- | --- | --- | ---: | ---: |",
    ]

    for row in rows:
        case = row["case"]
        judgement = row["judgement"]
        fact = judgement.get("fact_only", {}) if judgement else {}
        semantic = judgement.get("semantic_filtered", {}) if judgement else {}
        lines.append(
            "| {case_id} | {repo} | {pr} | {judged} | {fact_actionable} | {semantic_actionable} | {fact_missed} | {semantic_missed} |".format(
                case_id=case.get("id", ""),
                repo=case.get("repo", ""),
                pr=case.get("pr", ""),
                judged="yes" if judgement else "no",
                fact_actionable=str(bool_value(fact, "actionable")).lower() if judgement else "",
                semantic_actionable=str(bool_value(semantic, "actionable")).lower() if judgement else "",
                fact_missed=int_value(fact, "missed_high_risk_count") if judgement else "",
                semantic_missed=int_value(semantic, "missed_high_risk_count") if judgement else "",
            )
        )

    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text("\n".join(lines) + "\n")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Summarize frozen-v1 benchmark judgements.")
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--benchmark", type=Path, default=DEFAULT_BENCHMARK)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    try:
        summary = summarize(corpus_cases(args.corpus), args.benchmark)
    except ValueError as err:
        print(err, file=sys.stderr)
        return 2
    write_report(summary, args.report)
    print(f"wrote {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
