#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "run-error-propagation-gate.py"
SPEC = importlib.util.spec_from_file_location("run_error_propagation_gate", SCRIPT)
assert SPEC and SPEC.loader
run_error_propagation_gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(run_error_propagation_gate)


def contract_stdout(quality_block: str = "", semantic_block: str = "") -> str:
    quality_count = 1 if quality_block else 0
    semantic_count = len([line for line in semantic_block.splitlines() if line.startswith("- id=")])
    return f"""Diff context:
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
  evidence={quality_count} prod_files_changed={quality_count} test_files_changed={quality_count} docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
{quality_block}

Semantic impact:
  evidence={semantic_count}
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
{semantic_block}

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""


POSITIVE_STDOUT = contract_stdout(
    quality_block="""- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/read.rs""",
    semantic_block="""- id=2 category=semantic kind=panic_to_error_propagation_changed subject=Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint) source=semantic_diff confidence=high value=direct
  file=src/read.rs""",
)

POSITIVE_DIFF = """diff --git a/src/read.rs b/src/read.rs
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
@@ -1,3 +1,7 @@
+fn test_raw_invalid_utf8() {
+    let j = &[b'"', b'\\xCE', b'\\xF8', b'"'];
+    assert!(serde_json::from_slice::<Box<RawValue>>(j).is_err());
+}
"""

NEGATIVE_STDOUT = contract_stdout(
    semantic_block="""- id=1 category=semantic kind=integer_platform_boundary_changed subject=let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX); source=semantic_diff confidence=high value=direct
  file=src/decode.rs""",
)

NEGATIVE_DIFF = """diff --git a/src/decode.rs b/src/decode.rs
--- a/src/decode.rs
+++ b/src/decode.rs
@@ -1,5 +1,5 @@
-        let to_read = *remaining as usize;
+        let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX);
"""

MISS_STDOUT = contract_stdout()
MISS_DIFF = ""

DUP_STDOUT = contract_stdout(
    semantic_block="""- id=1 category=semantic kind=error_propagation_changed subject=return Poll::Ready(None); source=semantic_diff confidence=medium value=direct
  file=src/body/incoming.rs
- id=2 category=semantic kind=error_propagation_changed subject=Poll::Ready(None) source=semantic_diff confidence=medium value=direct
  file=src/body/incoming.rs""",
)

DUP_DIFF = """diff --git a/src/body/incoming.rs b/src/body/incoming.rs
--- a/src/body/incoming.rs
+++ b/src/body/incoming.rs
@@ -1,5 +1,10 @@
-                    Err(e) => Poll::Ready(Some(Err(crate::Error::new_h2(e)))),
+                    Err(e) => {
+                        if let Some(h2::Reason::NO_ERROR) = e.reason() {
+                            return Poll::Ready(None);
+                        }
+                        Poll::Ready(Some(Err(crate::Error::new_h2(e))))
+                    }
"""


class ErrorPropagationGateTest(unittest.TestCase):
    def test_evaluate_reports_precision_recall_and_boundary_honesty(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pack = root / "pack.toml"
            pack.write_text(
                """
version = "error-propagation-gate.v1"
risk_item = "error_propagation_change"

[gates]
raw_contract_valid_rate_min = 1.0
detection_precision_min = 1.0
detection_recall_min = 0.75
review_worthy_precision_min = 1.0
boundary_honesty_rate_min = 0.5
duplicate_wrapper_case_count_max = 0

[[cases]]
id = "positive"
role = "high_value_positive"
expected_detection = "detect"
expected_judgement = "review_worthy_behavior_change"
max_wrapper_count = 1

[[cases]]
id = "negative"
role = "negative_control"
expected_detection = "suppress"
expected_judgement = "none"
max_wrapper_count = 0

[[cases]]
id = "missed"
role = "known_miss_positive"
expected_detection = "detect"
expected_judgement = "needs_more_context_before_review_claim"
max_wrapper_count = 1
"""
            )
            self.write_case(root / "cases" / "positive", POSITIVE_STDOUT, POSITIVE_DIFF)
            self.write_case(root / "cases" / "negative", NEGATIVE_STDOUT, NEGATIVE_DIFF)
            self.write_case(root / "cases" / "missed", MISS_STDOUT, MISS_DIFF)

            result = run_error_propagation_gate.evaluate(pack, root / "cases")

        self.assertEqual(3, result["metrics"]["total_cases"])
        self.assertEqual(1.0, result["metrics"]["detection_precision"])
        self.assertEqual(0.5, result["metrics"]["detection_recall"])
        self.assertEqual(["missed"], result["missed_positive_cases"])
        self.assertFalse(result["gates"]["pass"])

    def test_evaluate_counts_duplicate_wrapper_cases(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pack = root / "pack.toml"
            pack.write_text(
                """
version = "error-propagation-gate.v1"
risk_item = "error_propagation_change"

[gates]
duplicate_wrapper_case_count_max = 0

[[cases]]
id = "dup"
role = "boundary_positive"
expected_detection = "detect"
expected_judgement = "needs_more_context_before_review_claim"
max_wrapper_count = 1
"""
            )
            self.write_case(root / "cases" / "dup", DUP_STDOUT, DUP_DIFF)

            result = run_error_propagation_gate.evaluate(pack, root / "cases")

        self.assertEqual(0, result["metrics"]["duplicate_wrapper_case_count"])
        self.assertEqual([], result["duplicate_cases"])
        self.assertTrue(result["rows"][0]["judgement_match"])

    @staticmethod
    def write_case(case_dir: Path, stdout: str, diff: str) -> None:
        raw = case_dir / "raw"
        raw.mkdir(parents=True)
        (raw / "decay.stdout").write_text(stdout)
        (raw / "diff.patch").write_text(diff)
        (raw / "pr.json").write_text(json.dumps({"title": "Fix panic"}))


if __name__ == "__main__":
    unittest.main()
