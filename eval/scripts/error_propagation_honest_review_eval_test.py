#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "evaluate-error-propagation-honest-review.py"
SPEC = importlib.util.spec_from_file_location("evaluate_error_propagation", SCRIPT)
assert SPEC and SPEC.loader
evaluate_error_propagation = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(evaluate_error_propagation)


def contract_stdout(changed_files: int, semantic_block: str = "") -> str:
    semantic_count = len([line for line in semantic_block.splitlines() if line.startswith("- id=")])
    return f"""Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files={changed_files}

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
  evidence={semantic_count}
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
{semantic_block}

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""


SERDE_STDOUT = contract_stdout(
    changed_files=2,
    semantic_block="""- id=1 category=semantic kind=panic_to_error_propagation_changed subject=Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint) source=semantic_diff confidence=high value=direct
  file=src/read.rs""",
)

SERDE_DIFF = """diff --git a/src/read.rs b/src/read.rs
--- a/src/read.rs
+++ b/src/read.rs
@@ -1,5 +1,8 @@
-        let raw = str::from_utf8(raw).unwrap();
+        let raw = match str::from_utf8(raw) {
+            Ok(raw) => raw,
+            Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint),
+        };
diff --git a/tests/test.rs b/tests/test.rs
--- a/tests/test.rs
+++ b/tests/test.rs
@@ -1,3 +1,9 @@
+fn test_raw_invalid_utf8() {
+    let j = &[b'"', b'\\xCE', b'\\xF8', b'"'];
+    let raw_value_err = serde_json::from_slice::<Box<RawValue>>(j).unwrap_err();
+    assert_eq!(raw_value_err.to_string(), "invalid unicode code point");
+}
"""

NO_WRAPPER_STDOUT = contract_stdout(changed_files=1)


class ErrorPropagationHonestReviewEvalTest(unittest.TestCase):
    def test_eval_reports_wrapper_hits_and_missed_ground_truth_separately(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cases = root / "cases"
            self.write_case(cases / "serde-rs-json-757", SERDE_STDOUT, SERDE_DIFF)
            self.write_case(cases / "missed-case", NO_WRAPPER_STDOUT, "")
            truth = root / "truth.toml"
            truth.write_text(
                """
[[cases]]
id = "serde-rs-json-757"
signal_ids = ["error_propagation"]
ground_truth_summary = "invalid utf8 panic"

[[cases]]
id = "missed-case"
signal_ids = ["error_propagation"]
ground_truth_summary = "known error propagation case"
"""
            )

            result = evaluate_error_propagation.evaluate(cases, truth)

        self.assertEqual(2, result["cases_scanned"])
        self.assertEqual(["serde-rs-json-757"], result["wrapper_cases"])
        self.assertEqual(["missed-case"], result["missed_ground_truth_cases"])
        self.assertEqual("confirmed", result["rows"][0]["mechanism"])
        self.assertEqual("partial", result["rows"][0]["control"])

    def test_markdown_summary_is_written(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "report.md"
            evaluate_error_propagation.write_markdown(
                {
                    "cases_scanned": 1,
                    "ground_truth_error_propagation_cases": ["a"],
                    "wrapper_cases": ["a"],
                    "review_worthy_cases": ["a"],
                    "missed_ground_truth_cases": [],
                    "overclaim_suspects": [],
                    "rows": [
                        {
                            "case_id": "a",
                            "ground_truth_error_propagation": True,
                            "mechanism": "confirmed",
                            "impact": "partial",
                            "control": "partial",
                            "judgement": "review_worthy_behavior_change",
                        }
                    ],
                },
                out,
            )

            text = out.read_text()

        self.assertIn("cases_scanned: 1", text)
        self.assertIn("review_worthy_behavior_change", text)

    @staticmethod
    def write_case(case_dir: Path, stdout: str, diff: str) -> None:
        raw = case_dir / "raw"
        raw.mkdir(parents=True)
        (raw / "decay.stdout").write_text(stdout)
        (raw / "diff.patch").write_text(diff)
        (raw / "pr.json").write_text(json.dumps({"title": "Fix panic"}))


if __name__ == "__main__":
    unittest.main()
