#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "replay-honest-review-benchmark.py"
SPEC = importlib.util.spec_from_file_location("replay_honest_review", SCRIPT)
assert SPEC and SPEC.loader
replay_honest_review = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(replay_honest_review)


STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=1

Top risk findings:
  none

Complexity impact:
  changed_functions=0 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk

Coverage impact:
  evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk

Stability impact:
  evidence=0
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal

Quality impact:
  evidence=0 prod_files_changed=0 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=1 category=semantic kind=integer_platform_boundary_changed subject=let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX); source=semantic_diff confidence=high value=direct
  file=src/decode.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

DIFF = """diff --git a/src/decode.rs b/src/decode.rs
--- a/src/decode.rs
+++ b/src/decode.rs
@@ -1,5 +1,5 @@
-let to_read = *remaining as usize;
+let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX);
"""


class HonestReviewReplayTest(unittest.TestCase):
    def test_replay_counts_missing_raw_and_honest_reviews(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            corpus = root / "corpus.toml"
            corpus.write_text(
                """
[[cases]]
id = "has-wrapper"
repo = "r"
pr = 1

[[cases]]
id = "missing"
repo = "r"
pr = 2
"""
            )
            raw = root / "cases" / "has-wrapper" / "raw"
            raw.mkdir(parents=True)
            (raw / "decay.stdout").write_text(STDOUT)
            (raw / "diff.patch").write_text(DIFF)
            (raw / "pr.json").write_text(json.dumps({"title": "fix 32-bit large body"}))

            result = replay_honest_review.replay(corpus, root / "cases")

        self.assertEqual(2, result["metrics"]["target_cases"])
        self.assertEqual(1, result["metrics"]["cases_with_raw"])
        self.assertEqual(1, result["metrics"]["cases_with_honest_review"])
        self.assertEqual({"integer_boundary_change": 1}, result["metrics"]["risk_item_counts"])
        self.assertEqual("missing_raw", result["rows"][1]["raw_status"])

    def test_markdown_report_is_written(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "report.md"
            replay_honest_review.write_markdown(
                {
                    "metrics": {
                        "target_cases": 1,
                        "cases_with_raw": 1,
                        "cases_with_wrappers": 1,
                        "cases_with_honest_review": 1,
                        "total_wrappers": 1,
                        "total_honest_reviews": 1,
                        "risk_item_counts": {"integer_boundary_change": 1},
                        "wrapper_type_counts": {"IntegerBoundaryWrapper": 1},
                        "dimension_counts": {"mechanism:confirmed": 1},
                        "ai_counts": {"ai_missing": 1},
                    },
                    "rows": [
                        {
                            "case_id": "a",
                            "wrapper_count": 1,
                            "honest_review_count": 1,
                            "risk_items": ["integer_boundary_change"],
                            "ai_status": "ai_missing",
                        }
                    ],
                },
                out,
            )

            text = out.read_text()

        self.assertIn("target_cases: 1", text)
        self.assertIn("integer_boundary_change", text)


if __name__ == "__main__":
    unittest.main()
