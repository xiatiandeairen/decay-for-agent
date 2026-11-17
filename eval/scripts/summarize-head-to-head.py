#!/usr/bin/env python3
"""Three-way head-to-head report: fact-only / semantic-filtered / ai-baseline.

Input:
  - frozen-v1 judgement directory (already populated by run-frozen-benchmark.py
    and graded by humans). Each `<case-id>.toml` contains `[fact_only]` and
    `[semantic_filtered]` sections with the same field set.
  - head-to-head judgement directory (`eval/head-to-head/judgements/`). Each
    `<case-id>.toml` contains an `[ai_baseline]` section with the same field
    set as the frozen-v1 sections.
  - head-to-head manifest (for the "clearly outperforms" thresholds).
  - frozen-v1 corpus (to know which case ids belong to the cohort).

Output:
  A Markdown report that compares the three runs across the four metrics:
    precision_at_top3, actionable_pr_rate, missed_high_risk_count,
    weak_only_top_risk_count.
  Plus a verdict line that reports how many threshold conditions the decay
  pipelines (fact-only and semantic-filtered) meet against the AI baseline.

Missing `[ai_baseline]` judgements are tolerated: the report will emit `N/A`
for that case row and exclude it from the AI baseline denominator. The intent
is that round 1 produces a skeleton report; round 2 fills the judgements and
the same script regenerates real metrics.
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
EVAL_ROOT = REPO_ROOT / "eval"
DEFAULT_CORPUS = EVAL_ROOT / "corpus" / "risk-signal-frozen-v1.toml"
DEFAULT_FROZEN_BENCHMARK = EVAL_ROOT / "benchmarks" / "frozen-v1"
DEFAULT_HEAD_TO_HEAD = EVAL_ROOT / "head-to-head"
DEFAULT_MANIFEST = DEFAULT_HEAD_TO_HEAD / "manifests" / "v1.toml"
DEFAULT_REPORT = EVAL_ROOT / "reports" / "head-to-head-v1.md"

# Mode keys we know about. Each maps to the TOML section name it lives in.
MODES = ("fact_only", "semantic_filtered", "ai_baseline")


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_corpus_case_ids(path: Path) -> list[str]:
    data = load_toml(path)
    cases = data.get("cases", [])
    return [str(c["id"]) for c in cases]


def _int_field(section: dict[str, Any], key: str) -> int:
    value = section.get(key, 0)
    return value if isinstance(value, int) and not isinstance(value, bool) else 0


def _bool_field(section: dict[str, Any], key: str) -> bool:
    return bool(section.get(key, False))


@dataclass(frozen=True)
class ModeMetrics:
    judged: int
    top3_total: int
    top3_useful: int
    actionable_count: int
    missed_high_risk_count: int
    weak_only_top_risk_count: int

    @property
    def precision_at_top3(self) -> float:
        return self.top3_useful / self.top3_total if self.top3_total else 0.0

    @property
    def actionable_pr_rate(self) -> float:
        return self.actionable_count / self.judged if self.judged else 0.0


def _load_frozen_judgement(case_id: str, frozen_benchmark: Path) -> dict[str, Any] | None:
    path = frozen_benchmark / "judgements" / f"{case_id}.toml"
    if not path.exists():
        return None
    data = load_toml(path)
    if data.get("status") != "complete":
        return None
    return data


def _load_ai_baseline_judgement(case_id: str, head_to_head_dir: Path) -> dict[str, Any] | None:
    path = head_to_head_dir / "judgements" / f"{case_id}.toml"
    if not path.exists():
        return None
    data = load_toml(path)
    if data.get("status") != "complete":
        return None
    return data


def collect_metrics(
    case_ids: list[str],
    frozen_benchmark: Path,
    head_to_head_dir: Path,
) -> dict[str, ModeMetrics]:
    counters: dict[str, dict[str, int]] = {
        mode: {
            "judged": 0,
            "top3_total": 0,
            "top3_useful": 0,
            "actionable_count": 0,
            "missed_high_risk_count": 0,
            "weak_only_top_risk_count": 0,
        }
        for mode in MODES
    }
    for case_id in case_ids:
        frozen = _load_frozen_judgement(case_id, frozen_benchmark)
        if frozen is not None:
            for mode in ("fact_only", "semantic_filtered"):
                section = frozen.get(mode, {})
                if not isinstance(section, dict):
                    continue
                _accumulate(counters[mode], section)
        ai = _load_ai_baseline_judgement(case_id, head_to_head_dir)
        if ai is not None:
            section = ai.get("ai_baseline", {})
            if isinstance(section, dict):
                _accumulate(counters["ai_baseline"], section)
    return {
        mode: ModeMetrics(
            judged=counters[mode]["judged"],
            top3_total=counters[mode]["top3_total"],
            top3_useful=counters[mode]["top3_useful"],
            actionable_count=counters[mode]["actionable_count"],
            missed_high_risk_count=counters[mode]["missed_high_risk_count"],
            weak_only_top_risk_count=counters[mode]["weak_only_top_risk_count"],
        )
        for mode in MODES
    }


def _accumulate(counter: dict[str, int], section: dict[str, Any]) -> None:
    counter["judged"] += 1
    counter["top3_total"] += _int_field(section, "top3_total")
    counter["top3_useful"] += _int_field(section, "top3_useful")
    if _bool_field(section, "actionable"):
        counter["actionable_count"] += 1
    counter["missed_high_risk_count"] += _int_field(section, "missed_high_risk_count")
    counter["weak_only_top_risk_count"] += _int_field(section, "weak_only_top_risk_count")


@dataclass(frozen=True)
class ThresholdVerdict:
    metric: str
    decay_value: float
    ai_value: float
    delta: float
    passes: bool
    rule: str


def evaluate_thresholds(
    decay: ModeMetrics,
    ai: ModeMetrics,
    manifest: dict[str, Any],
) -> list[ThresholdVerdict]:
    thresholds = manifest.get("thresholds", {})
    results: list[ThresholdVerdict] = []

    def add(metric: str, decay_value: float, ai_value: float, key: str) -> None:
        rule = thresholds.get(key, {})
        direction = rule.get("direction")
        if direction == "higher_is_better":
            min_delta = float(rule.get("min_delta_pp", 0))
            delta_pp = (decay_value - ai_value) * 100
            results.append(
                ThresholdVerdict(
                    metric=metric,
                    decay_value=decay_value,
                    ai_value=ai_value,
                    delta=delta_pp,
                    passes=delta_pp >= min_delta,
                    rule=f">= +{min_delta:.1f}pp",
                )
            )
        elif direction == "lower_is_better":
            min_delta = float(rule.get("min_delta_count", 0))
            delta = ai_value - decay_value
            results.append(
                ThresholdVerdict(
                    metric=metric,
                    decay_value=decay_value,
                    ai_value=ai_value,
                    delta=delta,
                    passes=delta >= min_delta,
                    rule=f"AI - decay >= {int(min_delta)}",
                )
            )

    add("precision_at_top3", decay.precision_at_top3, ai.precision_at_top3, "precision_at_top3")
    add("actionable_pr_rate", decay.actionable_pr_rate, ai.actionable_pr_rate, "actionable_pr_rate")
    add(
        "missed_high_risk_count",
        decay.missed_high_risk_count,
        ai.missed_high_risk_count,
        "missed_high_risk_count",
    )
    add(
        "weak_only_top_risk_count",
        decay.weak_only_top_risk_count,
        ai.weak_only_top_risk_count,
        "weak_only_top_risk_count",
    )
    return results


def _pct(value: float) -> str:
    return f"{value:.2%}"


def render_report(
    case_ids: list[str],
    metrics: dict[str, ModeMetrics],
    manifest: dict[str, Any],
    frozen_benchmark: Path,
    head_to_head_dir: Path,
) -> str:
    required_passed = int(manifest.get("thresholds", {}).get("required_metrics_passed", 3))
    fact_v_ai = evaluate_thresholds(metrics["fact_only"], metrics["ai_baseline"], manifest)
    semantic_v_ai = evaluate_thresholds(metrics["semantic_filtered"], metrics["ai_baseline"], manifest)

    fact_pass = sum(1 for v in fact_v_ai if v.passes)
    semantic_pass = sum(1 for v in semantic_v_ai if v.passes)

    lines: list[str] = []
    lines.append("# Head-to-head v1 report")
    lines.append("")
    lines.append(
        "This report compares decay's fact-only and semantic-filtered output against an AI baseline "
        "that reads only the PR diff (no title, body, discussion, linked issue, CI logs, or repository "
        "context). Methodology is frozen in `eval/head-to-head/manifests/v1.toml`."
    )
    lines.append("")
    lines.append("## Cohort")
    lines.append("")
    lines.append(f"- corpus cases: {len(case_ids)}")
    for mode, label in (
        ("fact_only", "fact-only judged"),
        ("semantic_filtered", "semantic-filtered judged"),
        ("ai_baseline", "ai-baseline judged"),
    ):
        lines.append(f"- {label}: {metrics[mode].judged}")
    lines.append("")
    lines.append("## Metrics")
    lines.append("")
    lines.append("| Metric | fact-only | semantic-filtered | ai-baseline |")
    lines.append("|---|---:|---:|---:|")
    lines.append(
        "| precision@top3 | {f} | {s} | {a} |".format(
            f=_pct(metrics["fact_only"].precision_at_top3),
            s=_pct(metrics["semantic_filtered"].precision_at_top3),
            a=_pct(metrics["ai_baseline"].precision_at_top3),
        )
    )
    lines.append(
        "| actionable_pr_rate | {f} | {s} | {a} |".format(
            f=_pct(metrics["fact_only"].actionable_pr_rate),
            s=_pct(metrics["semantic_filtered"].actionable_pr_rate),
            a=_pct(metrics["ai_baseline"].actionable_pr_rate),
        )
    )
    lines.append(
        "| missed_high_risk_count | {f} | {s} | {a} |".format(
            f=metrics["fact_only"].missed_high_risk_count,
            s=metrics["semantic_filtered"].missed_high_risk_count,
            a=metrics["ai_baseline"].missed_high_risk_count,
        )
    )
    lines.append(
        "| weak_only_top_risk_count | {f} | {s} | {a} |".format(
            f=metrics["fact_only"].weak_only_top_risk_count,
            s=metrics["semantic_filtered"].weak_only_top_risk_count,
            a=metrics["ai_baseline"].weak_only_top_risk_count,
        )
    )
    lines.append("")
    lines.append("## Threshold verdicts (vs ai-baseline)")
    lines.append("")
    lines.append(f"Need to pass {required_passed} of 4 thresholds to claim a clear win.")
    lines.append("")
    lines.append("### fact-only vs ai-baseline")
    lines.append("")
    lines.append("| Metric | decay | ai-baseline | delta | rule | pass |")
    lines.append("|---|---:|---:|---:|---|:-:|")
    for v in fact_v_ai:
        lines.append(_threshold_row(v))
    lines.append(f"\n**fact-only: {fact_pass} / 4 thresholds pass; need {required_passed}.**")
    lines.append("")
    lines.append("### semantic-filtered vs ai-baseline")
    lines.append("")
    lines.append("| Metric | decay | ai-baseline | delta | rule | pass |")
    lines.append("|---|---:|---:|---:|---|:-:|")
    for v in semantic_v_ai:
        lines.append(_threshold_row(v))
    lines.append(f"\n**semantic-filtered: {semantic_pass} / 4 thresholds pass; need {required_passed}.**")
    lines.append("")
    lines.append("## Per-case judgement coverage")
    lines.append("")
    lines.append("| case_id | fact | semantic | ai-baseline |")
    lines.append("|---|:-:|:-:|:-:|")
    for case_id in case_ids:
        frozen = _load_frozen_judgement(case_id, frozen_benchmark)
        ai = _load_ai_baseline_judgement(case_id, head_to_head_dir)
        has_fact = "Y" if frozen and isinstance(frozen.get("fact_only"), dict) else "—"
        has_sem = "Y" if frozen and isinstance(frozen.get("semantic_filtered"), dict) else "—"
        has_ai = "Y" if ai and isinstance(ai.get("ai_baseline"), dict) else "—"
        lines.append(f"| {case_id} | {has_fact} | {has_sem} | {has_ai} |")
    lines.append("")
    if metrics["ai_baseline"].judged == 0:
        lines.append(
            "_Note: no ai-baseline judgements available yet — this report is a skeleton. "
            "Once round 2 grades ai-baseline outputs, regenerate this report._"
        )
        lines.append("")
    return "\n".join(lines) + "\n"


def _threshold_row(v: ThresholdVerdict) -> str:
    if v.metric in {"precision_at_top3", "actionable_pr_rate"}:
        return "| {m} | {d} | {a} | {pp:+.2f}pp | {r} | {ok} |".format(
            m=v.metric,
            d=_pct(v.decay_value),
            a=_pct(v.ai_value),
            pp=v.delta,
            r=v.rule,
            ok="✓" if v.passes else "✗",
        )
    return "| {m} | {d} | {a} | {delta:+.0f} | {r} | {ok} |".format(
        m=v.metric,
        d=int(v.decay_value),
        a=int(v.ai_value),
        delta=v.delta,
        r=v.rule,
        ok="✓" if v.passes else "✗",
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--frozen-benchmark", type=Path, default=DEFAULT_FROZEN_BENCHMARK)
    parser.add_argument("--head-to-head", type=Path, default=DEFAULT_HEAD_TO_HEAD)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--out", type=Path, default=DEFAULT_REPORT)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    case_ids = load_corpus_case_ids(args.corpus)
    manifest = load_toml(args.manifest)
    metrics = collect_metrics(case_ids, args.frozen_benchmark, args.head_to_head)
    report = render_report(case_ids, metrics, manifest, args.frozen_benchmark, args.head_to_head)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(report)
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
