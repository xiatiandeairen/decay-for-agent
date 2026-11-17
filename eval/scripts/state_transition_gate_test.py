#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "run-state-transition-gate.py"
SPEC = importlib.util.spec_from_file_location("run_state_transition_gate", SCRIPT)
assert SPEC and SPEC.loader
run_state_transition_gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(run_state_transition_gate)


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
  file=tokio/src/sync/batch_semaphore.rs""",
    semantic_block="""- id=2 category=semantic kind=boundary_expression_changed subject=(new << Self::PERMIT_SHIFT) | (curr_bits & Self::CLOSED) source=semantic_diff confidence=high value=direct
  file=tokio/src/sync/batch_semaphore.rs""",
)

POSITIVE_DIFF = """diff --git a/tokio/src/sync/batch_semaphore.rs b/tokio/src/sync/batch_semaphore.rs
--- a/tokio/src/sync/batch_semaphore.rs
+++ b/tokio/src/sync/batch_semaphore.rs
@@ -1,5 +1,5 @@
-        let next = new << Self::PERMIT_SHIFT;
+        let next = (new << Self::PERMIT_SHIFT) | (curr_bits & Self::CLOSED);
diff --git a/tokio/tests/sync_semaphore.rs b/tokio/tests/sync_semaphore.rs
--- a/tokio/tests/sync_semaphore.rs
+++ b/tokio/tests/sync_semaphore.rs
@@ -1,3 +1,8 @@
+#[tokio::test]
+async fn closed_semaphore_stays_closed_after_forget() {
+    let result = timeout(Duration::from_secs(5), fut).await;
+    assert!(result.is_ok());
+}
"""

BOUNDARY_STDOUT = contract_stdout(
    quality_block="""- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h2/client.rs""",
    semantic_block="""- id=2 category=semantic kind=boundary_expression_changed subject=let cancel_result = this.cancel_rx.as_mut().map(|rx| Pin::new(rx).poll(cx)); source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs""",
)

BOUNDARY_DIFF = """diff --git a/src/proto/h2/client.rs b/src/proto/h2/client.rs
--- a/src/proto/h2/client.rs
+++ b/src/proto/h2/client.rs
@@ -1,5 +1,10 @@
+        let cancel_result = this.cancel_rx.as_mut().map(|rx| Pin::new(rx).poll(cx));
+        match cancel_result {
+            Some(Poll::Pending) | None => {}
+            _ => {}
+        }
diff --git a/tests/client.rs b/tests/client.rs
--- a/tests/client.rs
+++ b/tests/client.rs
@@ -1,3 +1,8 @@
+#[tokio::test]
+async fn drop_response_future_sends_reset() {
+    let result = timeout(Duration::from_secs(5), fut).await;
+    assert!(result.is_ok());
+}
"""

NEGATIVE_STDOUT = contract_stdout(
    semantic_block="""- id=1 category=semantic kind=panic_to_error_propagation_changed subject=Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint) source=semantic_diff confidence=high value=direct
  file=src/read.rs""",
)

NEGATIVE_DIFF = """diff --git a/src/read.rs b/src/read.rs
--- a/src/read.rs
+++ b/src/read.rs
@@ -1,5 +1,8 @@
-        let raw = str::from_utf8(raw).unwrap();
+        let raw = match str::from_utf8(raw) {
+            Ok(raw) => raw,
+            Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint),
+        };
"""


class StateTransitionGateTest(unittest.TestCase):
    def test_evaluate_reports_precision_recall_and_boundary_honesty(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pack = root / "pack.toml"
            pack.write_text(
                """
version = "state-transition-gate.v1"
risk_item = "state_transition_change"

[gates]
raw_contract_valid_rate_min = 1.0
detection_precision_min = 1.0
detection_recall_min = 1.0
review_worthy_precision_min = 1.0
boundary_honesty_rate_min = 1.0
duplicate_wrapper_case_count_max = 0

[[cases]]
id = "positive"
role = "high_value_positive"
expected_detection = "detect"
expected_judgement = "review_worthy_state_transition_change"
max_wrapper_count = 1

[[cases]]
id = "boundary"
role = "boundary_positive"
expected_detection = "detect"
expected_judgement = "needs_more_context_before_review_claim"
max_wrapper_count = 1

[[cases]]
id = "negative"
role = "negative_control"
expected_detection = "suppress"
expected_judgement = "none"
max_wrapper_count = 0
"""
            )
            self.write_case(root / "cases" / "positive", POSITIVE_STDOUT, POSITIVE_DIFF)
            self.write_case(root / "cases" / "boundary", BOUNDARY_STDOUT, BOUNDARY_DIFF)
            self.write_case(root / "cases" / "negative", NEGATIVE_STDOUT, NEGATIVE_DIFF)

            result = run_state_transition_gate.evaluate(pack, root / "cases")

        self.assertEqual(3, result["metrics"]["total_cases"])
        self.assertEqual(1.0, result["metrics"]["detection_precision"])
        self.assertEqual(1.0, result["metrics"]["detection_recall"])
        self.assertEqual(1.0, result["metrics"]["boundary_honesty_rate"])
        self.assertTrue(result["gates"]["pass"])

    @staticmethod
    def write_case(case_dir: Path, stdout: str, diff: str) -> None:
        raw = case_dir / "raw"
        raw.mkdir(parents=True)
        (raw / "decay.stdout").write_text(stdout)
        (raw / "diff.patch").write_text(diff)
        (raw / "pr.json").write_text(json.dumps({"title": "Fix state transition"}))


if __name__ == "__main__":
    unittest.main()
