#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "scripts"


def load_script(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


runner = load_script("run-frozen-benchmark.py")
summarizer = load_script("summarize-frozen-benchmark.py")


class FrozenBenchmarkTest(unittest.TestCase):
    def test_dry_run_writes_manifest_run_log_and_judgement_template(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            corpus = root / "corpus.toml"
            out = root / "benchmark"
            corpus.write_text(
                """
[[cases]]
id = "case-a"
repo = "owner/repo"
pr = 1
kind = "risk_signal"
status = "freeze_ready"
reason = "Example risk."
""".strip()
                + "\n"
            )

            rc = runner.main(["--corpus", str(corpus), "--out", str(out), "--dry-run", "--mode", "both"])

            self.assertEqual(0, rc)
            manifest = json.loads((out / "manifest.json").read_text())
            self.assertEqual(["fact", "semantic"], manifest["modes"])
            self.assertEqual(["case-a"], manifest["case_ids"])
            run_rows = [json.loads(line) for line in (out / "run-log.jsonl").read_text().splitlines()]
            self.assertEqual(["fact", "semantic"], [row["mode"] for row in run_rows])
            judgement = tomllib.loads((out / "judgements" / "case-a.toml").read_text())
            self.assertEqual("risk-eval-judgement.v1", judgement["version"])
            self.assertEqual("pending", judgement["status"])
            self.assertIn("ground_truth", judgement)
            self.assertIn("fact_only", judgement)
            self.assertIn("semantic_filtered", judgement)
            self.assertIn("gate", judgement)

            rc = runner.main(["--corpus", str(corpus), "--out", str(out), "--dry-run", "--mode", "fact"])

            self.assertEqual(0, rc)
            run_rows = [json.loads(line) for line in (out / "run-log.jsonl").read_text().splitlines()]
            self.assertEqual(["fact"], [row["mode"] for row in run_rows])

    def test_summarizer_counts_complete_judgements_and_pending_cases(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            corpus = root / "corpus.toml"
            benchmark = root / "benchmark"
            report = root / "report.md"
            corpus.write_text(
                """
[[cases]]
id = "case-a"
repo = "owner/repo"
pr = 1
kind = "risk_signal"
status = "freeze_ready"
reason = "Example risk."

[[cases]]
id = "case-b"
repo = "owner/repo"
pr = 2
kind = "risk_signal"
status = "freeze_ready"
reason = "Pending risk."
""".strip()
                + "\n"
            )
            judgement_dir = benchmark / "judgements"
            judgement_dir.mkdir(parents=True)
            (judgement_dir / "case-a.toml").write_text(
                """
case_id = "case-a"
status = "complete"

[fact_only]
top3_total = 3
top3_useful = 1
actionable = true
missed_high_risk_count = 1
weak_only_top_risk_count = 2

[semantic_filtered]
top3_total = 3
top3_useful = 2
actionable = true
missed_high_risk_count = 0
weak_only_top_risk_count = 0

[comparison]
semantic_improved_ranking = true
semantic_reduced_noise = true
semantic_introduced_noise = false
""".strip()
                + "\n"
            )

            summary = summarizer.summarize(summarizer.corpus_cases(corpus), benchmark)
            summarizer.write_report(summary, report)

            metrics = summary["metrics"]
            self.assertEqual(2, metrics["target_cases"])
            self.assertEqual(1, metrics["judged_cases"])
            self.assertEqual(1, metrics["pending_cases"])
            self.assertAlmostEqual(1 / 3, metrics["fact_precision_at_top3"])
            self.assertAlmostEqual(2 / 3, metrics["semantic_precision_at_top3"])
            self.assertEqual(1, metrics["semantic_improved_ranking_count"])
            self.assertIn("pending_cases: 1", report.read_text())


if __name__ == "__main__":
    unittest.main()
