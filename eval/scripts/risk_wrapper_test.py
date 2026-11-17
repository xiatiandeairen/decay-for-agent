#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "build-risk-wrapper.py"
SPEC = importlib.util.spec_from_file_location("build_risk_wrapper", SCRIPT)
assert SPEC and SPEC.loader
build_risk_wrapper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_risk_wrapper)


SERDE_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/read.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=panic_to_error_propagation_changed subject=Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint) source=semantic_diff confidence=high value=direct
  file=src/read.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

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
@@ -1,3 +1,13 @@
+#[cfg(feature = "raw_value")]
+#[test]
+fn test_raw_invalid_utf8() {
+    use serde_json::value::RawValue;
+    let j = &[b'"', b'\\xCE', b'\\xF8', b'"'];
+    let raw_value_err = serde_json::from_slice::<Box<RawValue>>(j).unwrap_err();
+    assert_eq!(
+        raw_value_err.to_string(),
+        "invalid unicode code point at line 1 column 4",
+    );
+}
"""

UNRELATED_ERROR_STDOUT = """Diff context:
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
- id=1 category=semantic kind=panic_to_error_propagation_changed subject=Err(e) => return Err(e) source=semantic_diff confidence=high value=direct
  file=src/read.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

UNRELATED_ERROR_DIFF = """diff --git a/src/read.rs b/src/read.rs
--- a/src/read.rs
+++ b/src/read.rs
@@ -1,5 +1,7 @@
-        let header = parse_header(line).unwrap();
+        let header = parse_header(line)?;
         let raw = input.raw();
+        if raw.is_empty() {
+            return Err(Error::Empty);
+        }
"""

NO_TEST_ERROR_DIFF = """diff --git a/src/read.rs b/src/read.rs
--- a/src/read.rs
+++ b/src/read.rs
@@ -1,5 +1,8 @@
-        let raw = str::from_utf8(raw).unwrap();
+        let raw = match str::from_utf8(raw) {
+            Ok(raw) => raw,
+            Err(_) => return error(self, ErrorCode::InvalidUnicodeCodePoint),
+        };
"""

ERROR_FLOW_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/body/incoming.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=error_propagation_changed subject=Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => Poll::Ready(None) source=semantic_diff confidence=medium value=direct
  file=src/body/incoming.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

ERROR_FLOW_DIFF = """diff --git a/src/body/incoming.rs b/src/body/incoming.rs
--- a/src/body/incoming.rs
+++ b/src/body/incoming.rs
@@ -1,5 +1,12 @@
-                        Some(Err(e)) => return Poll::Ready(Some(Err(crate::Error::new_body(e)))),
+                        Some(Err(e)) => {
+                            return match e.reason() {
+                                Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => {
+                                    Poll::Ready(None)
+                                }
+                                _ => Poll::Ready(Some(Err(crate::Error::new_body(e)))),
+                            };
+                        }
diff --git a/tests/client.rs b/tests/client.rs
--- a/tests/client.rs
+++ b/tests/client.rs
@@ -1,3 +1,8 @@
+#[test]
+fn h2_cancel_is_not_body_error() {
+    let err = run_h2_cancel().unwrap_err();
+    assert!(err.is_canceled());
+}
"""

HYPER_STDOUT = """Diff context:
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
  evidence=1 prod_files_changed=1 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=src/proto/h1/decode.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=integer_platform_boundary_changed subject=let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX); source=semantic_diff confidence=high value=direct
  file=src/proto/h1/decode.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

HYPER_DIFF = """diff --git a/src/proto/h1/decode.rs b/src/proto/h1/decode.rs
--- a/src/proto/h1/decode.rs
+++ b/src/proto/h1/decode.rs
@@ -1,5 +1,5 @@
-                    let to_read = *remaining as usize;
+                    let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX);
"""

HYPER_WITH_TEST_DIFF = """diff --git a/src/proto/h1/decode.rs b/src/proto/h1/decode.rs
--- a/src/proto/h1/decode.rs
+++ b/src/proto/h1/decode.rs
@@ -1,5 +1,5 @@
-                    let to_read = *remaining as usize;
+                    let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX);
diff --git a/tests/h1.rs b/tests/h1.rs
--- a/tests/h1.rs
+++ b/tests/h1.rs
@@ -1,3 +1,8 @@
+#[test]
+fn large_content_length_on_32_bit() {
+    let length = u64::MAX;
+    assert_eq!(simulate_read(length), usize::MAX);
+}
"""

HYPER_AFTER_ONLY_DIFF = """diff --git a/src/proto/h1/decode.rs b/src/proto/h1/decode.rs
--- a/src/proto/h1/decode.rs
+++ b/src/proto/h1/decode.rs
@@ -1,5 +1,6 @@
                     let to_read = read_hint();
+                    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
"""

TOKIO_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=tokio/src/sync/batch_semaphore.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=boundary_expression_changed subject=(new << Self::PERMIT_SHIFT) | (curr_bits & Self::CLOSED) source=semantic_diff confidence=high value=direct
  file=tokio/src/sync/batch_semaphore.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

TOKIO_DIFF = """diff --git a/tokio/src/sync/batch_semaphore.rs b/tokio/src/sync/batch_semaphore.rs
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

STATE_CANCEL_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h2/client.rs

Semantic impact:
  evidence=2
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=boundary_expression_changed subject=let cancel_result = this.cancel_rx.as_mut().map(|rx| Pin::new(rx).poll(cx)); source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs
- id=3 category=semantic kind=boundary_expression_changed subject=pub(crate) fn cancel(self: Pin<&mut Self>) { source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

STATE_CANCEL_DIFF = """diff --git a/src/proto/h2/client.rs b/src/proto/h2/client.rs
--- a/src/proto/h2/client.rs
+++ b/src/proto/h2/client.rs
@@ -1,5 +1,12 @@
+        let cancel_result = this.cancel_rx.as_mut().map(|rx| Pin::new(rx).poll(cx));
+        match cancel_result {
+            Some(Poll::Pending) | None => {}
+            _ => {}
+        }
+pub(crate) fn cancel(self: Pin<&mut Self>) {
+    self.body_tx.send_reset(h2::Reason::CANCEL);
+}
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

LIFECYCLE_UPGRADE_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/server/conn/http1.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=lifecycle_ordering_changed subject=Pin::new(conn).graceful_shutdown() source=semantic_diff confidence=high value=direct
  file=src/server/conn/http1.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

LIFECYCLE_UPGRADE_DIFF = """diff --git a/src/server/conn/http1.rs b/src/server/conn/http1.rs
--- a/src/server/conn/http1.rs
+++ b/src/server/conn/http1.rs
@@ -1,5 +1,10 @@
-        Pin::new(self.inner.as_mut().unwrap()).graceful_shutdown()
+        // Connection (`inner`) is `None` if it was upgraded (and `poll` is `Ready`).
+        // In that case, we don't need to call `graceful_shutdown`.
+        if let Some(conn) = self.inner.as_mut() {
+            Pin::new(conn).graceful_shutdown()
+        }
diff --git a/tests/server.rs b/tests/server.rs
--- a/tests/server.rs
+++ b/tests/server.rs
@@ -1,3 +1,8 @@
+#[tokio::test]
+async fn http1_graceful_shutdown_after_upgrade() {
+    conn.as_mut().graceful_shutdown();
+    assert!(future.await.is_ok());
+}
"""

LIFECYCLE_INITIAL_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=3

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
  evidence=1 prod_files_changed=2 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h1/dispatch.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=lifecycle_ordering_changed subject=if self.conn.is_write_closed() || self.conn.has_initial_read_write_state() { source=semantic_diff confidence=high value=direct
  file=src/proto/h1/dispatch.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

LIFECYCLE_INITIAL_DIFF = """diff --git a/src/proto/h1/dispatch.rs b/src/proto/h1/dispatch.rs
--- a/src/proto/h1/dispatch.rs
+++ b/src/proto/h1/dispatch.rs
@@ -1,5 +1,8 @@
-        if self.conn.is_write_closed() {
+        if self.conn.is_write_closed() || self.conn.has_initial_read_write_state() {
             self.close();
         }
diff --git a/tests/server.rs b/tests/server.rs
--- a/tests/server.rs
+++ b/tests/server.rs
@@ -1,3 +1,8 @@
+#[tokio::test]
+async fn graceful_shutdown_before_first_request_no_block() {
+    future.as_mut().graceful_shutdown();
+    assert!(timeout(Duration::from_secs(1), future).await.is_ok());
+}
"""

LIFECYCLE_CANCEL_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=3

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
  evidence=1 prod_files_changed=2 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h2/client.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=lifecycle_ordering_changed subject=this.pipe.as_mut().send_reset(h2::Reason::CANCEL); source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

LIFECYCLE_CANCEL_DIFF = """diff --git a/src/proto/h2/client.rs b/src/proto/h2/client.rs
--- a/src/proto/h2/client.rs
+++ b/src/proto/h2/client.rs
@@ -1,5 +1,14 @@
+        let cancel_result = this.cancel_rx.as_mut().map(|rx| Pin::new(rx).poll(cx));
+        match cancel_result {
+            Some(Poll::Ready(Ok(()))) => {
+                this.pipe.as_mut().send_reset(h2::Reason::CANCEL);
+                drop(this.conn_drop_ref.take().expect("Future polled twice"));
+                return Poll::Ready(());
+            }
+            Some(Poll::Pending) | None => {}
+            _ => {}
+        }
diff --git a/tests/client.rs b/tests/client.rs
--- a/tests/client.rs
+++ b/tests/client.rs
@@ -1,3 +1,8 @@
+#[tokio::test]
+async fn h2_pipe_task_cancelled_on_response_future_drop() {
+    let result = timeout(Duration::from_secs(5), fut).await;
+    assert!(result.is_ok());
+}
"""

CONCURRENCY_BLOCKING_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

Top risk findings:
  none

Complexity impact:
  changed_functions=1 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk

Coverage impact:
  evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk

Stability impact:
  evidence=0
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal

Quality impact:
  evidence=2 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=build_or_ci_changed subject=build or ci source=quality confidence=low value=context
  file=.github/workflows/loom.yml
- id=2 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=tokio/src/runtime/blocking/pool.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=3 category=semantic kind=concurrency_ordering_changed subject=let mut is_counted_idle; source=semantic_diff confidence=high value=direct
  file=tokio/src/runtime/blocking/pool.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

CONCURRENCY_BLOCKING_DIFF = """diff --git a/tokio/src/runtime/blocking/pool.rs b/tokio/src/runtime/blocking/pool.rs
--- a/tokio/src/runtime/blocking/pool.rs
+++ b/tokio/src/runtime/blocking/pool.rs
@@ -1,5 +1,14 @@
+        // is this thread currently counted in `num_idle_threads`?
+        let mut is_counted_idle;
+        // mark this thread as currently counted in `num_idle_threads`.
+        is_counted_idle = true;
+        // since this is a legitimate wakeup,
+        // the `Spawner::spawn_task` has already decremented `num_idle_threads`.
+        is_counted_idle = false;
+        if is_counted_idle {
+            let prev_idle = self.metrics.dec_num_idle_threads();
+        }
diff --git a/tokio/src/runtime/tests/loom_blocking.rs b/tokio/src/runtime/tests/loom_blocking.rs
--- a/tokio/src/runtime/tests/loom_blocking.rs
+++ b/tokio/src/runtime/tests/loom_blocking.rs
@@ -1,3 +1,8 @@
+fn spawn_blocking_then_shutdown() {
+    loom::model(|| {
+        let _ = loom::future::block_on(jh3);
+    });
+}
"""

CONCURRENCY_WAKEUP_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h1/dispatch.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=concurrency_ordering_changed subject=let wants_write_again = self.can_write_again() && conn_ready; source=semantic_diff confidence=high value=direct
  file=src/proto/h1/dispatch.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

CONCURRENCY_WAKEUP_DIFF = """diff --git a/src/proto/h1/dispatch.rs b/src/proto/h1/dispatch.rs
--- a/src/proto/h1/dispatch.rs
+++ b/src/proto/h1/dispatch.rs
@@ -1,5 +1,11 @@
-            let _ = self.poll_flush(cx)?;
+            let conn_ready = self.poll_flush(cx)?.is_ready();
+            let wants_write_again = self.can_write_again() && conn_ready;
+            let wants_read_again = self.conn.wants_read_again();
+            if !(wants_write_again || wants_read_again) {
+                return Poll::Ready(Ok(()));
+            }
diff --git a/tests/ready_stream.rs b/tests/ready_stream.rs
--- a/tests/ready_stream.rs
+++ b/tests/ready_stream.rs
@@ -1,3 +1,8 @@
+#[tokio::test]
+async fn ready_stream_does_not_stall() {
+    assert!(task.await.is_ok());
+}
"""

CONCURRENCY_COMPLETED_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=4

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
  evidence=1 prod_files_changed=1 test_files_changed=3 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=tokio/src/runtime/io/driver/uring.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=concurrency_ordering_changed subject=Lifecycle::Completed(cqe) => { source=semantic_diff confidence=high value=direct
  file=tokio/src/runtime/io/driver/uring.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

CONCURRENCY_COMPLETED_DIFF = """diff --git a/tokio/src/runtime/io/driver/uring.rs b/tokio/src/runtime/io/driver/uring.rs
--- a/tokio/src/runtime/io/driver/uring.rs
+++ b/tokio/src/runtime/io/driver/uring.rs
@@ -1,5 +1,12 @@
+            Lifecycle::Completed(cqe) => {
+                if let Lifecycle::Cancelled(CancelData::Open(_)) = lifecycle {
+                    unsafe { OwnedFd::from_raw_fd(fd as i32) };
+                }
+            }
diff --git a/tokio/tests/fs_uring_completed_then_dropped_before_repoll.rs b/tokio/tests/fs_uring_completed_then_dropped_before_repoll.rs
--- a/tokio/tests/fs_uring_completed_then_dropped_before_repoll.rs
+++ b/tokio/tests/fs_uring_completed_then_dropped_before_repoll.rs
@@ -1,3 +1,8 @@
+async fn completed_then_dropped_before_repoll(path: PathBuf) {
+    timeout(Duration::from_secs(2), second_rx.recv()).await.unwrap();
+    handle.abort();
+}
"""

EXTERNAL_HOST_STDOUT = """Diff context:
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
- id=1 category=semantic kind=protocol_header_authority_changed subject=Host header authority -> host/port source=semantic_diff confidence=high value=direct
  file=examples/client.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

EXTERNAL_HOST_DIFF = """diff --git a/examples/client.rs b/examples/client.rs
--- a/examples/client.rs
+++ b/examples/client.rs
@@ -1,6 +1,6 @@
-    let authority = url.authority().unwrap().clone();
-    .header(hyper::header::HOST, authority.as_str())
+    let host = url.host().expect("uri has no host");
+    .header(hyper::header::HOST, format!("{}:{}", host, port))
"""

EXTERNAL_COMMAND_STDOUT = """Diff context:
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
- id=1 category=semantic kind=default_value_changed subject=ARGS_GZIP: &["gzip", "-d", "-c"] -> &["gzip", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

EXTERNAL_COMMAND_DIFF = """diff --git a/crates/cli/src/decompress.rs b/crates/cli/src/decompress.rs
--- a/crates/cli/src/decompress.rs
+++ b/crates/cli/src/decompress.rs
@@ -1,3 +1,3 @@
-const ARGS_GZIP: &[&str] = &["gzip", "-d", "-c"];
+const ARGS_GZIP: &[&str] = &["gzip", "-d", "-c", "--"];
"""

EXTERNAL_KEEPALIVE_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

Top risk findings:
  none

Complexity impact:
  changed_functions=0 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk

Coverage impact:
  evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk

Stability impact:
  evidence=1
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=1 category=stability kind=public_api_function_added subject=http10_disable_keep_alive source=stability confidence=high value=direct
  file=src/server.rs

Quality impact:
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/server.rs

Semantic impact:
  evidence=0
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

EXTERNAL_KEEPALIVE_DIFF = """diff --git a/src/server.rs b/src/server.rs
--- a/src/server.rs
+++ b/src/server.rs
@@ -1,5 +1,13 @@
+    /// Set whether to disable keep alive for HTTP/1.0.
+    pub fn http10_disable_keep_alive(&mut self, disable: bool) -> &mut Self {
+        self.conn_builder.http10_disable_keep_alive(disable);
+        self
+    }
+    /// enforcing Connection: close for HTTP/1.0 Request or Response
diff --git a/tests/server.rs b/tests/server.rs
--- a/tests/server.rs
+++ b/tests/server.rs
@@ -1,3 +1,8 @@
+#[test]
+fn http10_disable_keep_alive_turns_off_reuse() {
+    assert!(true);
+}
"""

EXTERNAL_API_BOUNDARY_STDOUT = """Diff context:
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
  evidence=2
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=1 category=stability kind=public_api_added subject=ChannelMockConnector source=stability confidence=high value=direct
  file=src/mock.rs
- id=2 category=stability kind=public_api_signature_changed subject=new source=stability confidence=high value=direct
  file=src/mock.rs

Quality impact:
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=3 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/mock.rs

Semantic impact:
  evidence=0
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

EXTERNAL_API_BOUNDARY_DIFF = """diff --git a/src/mock.rs b/src/mock.rs
--- a/src/mock.rs
+++ b/src/mock.rs
@@ -1,5 +1,9 @@
+pub struct ChannelMockConnector {
+    calls: Sender<String>,
+}
+    pub fn new(calls: Sender<String>) -> ChannelMockConnector {
+        ChannelMockConnector { calls: calls }
+    }
diff --git a/tests/mock.rs b/tests/mock.rs
--- a/tests/mock.rs
+++ b/tests/mock.rs
@@ -1,3 +1,7 @@
+#[test]
+fn connector_uses_ssl_verifier() {
+    assert!(true);
+}
"""

MULTI_PRIMARY_STDOUT = """Diff context:
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
  evidence=2
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=1 category=semantic kind=error_propagation_changed subject=return Poll::Ready(None); source=semantic_diff confidence=medium value=direct
  file=src/body/incoming.rs
- id=2 category=semantic kind=error_propagation_changed subject=Poll::Ready(None) source=semantic_diff confidence=medium value=direct
  file=src/body/incoming.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

MULTI_PRIMARY_DIFF = """diff --git a/src/body/incoming.rs b/src/body/incoming.rs
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

READINESS_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=tokio/src/net/udp.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=readiness_error_propagation_changed subject=.async_io(Interest::READABLE | Interest::ERROR, || self.io.recv(buf)) source=semantic_diff confidence=high value=direct
  file=tokio/src/net/udp.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

READINESS_DIFF = """diff --git a/tokio/src/net/udp.rs b/tokio/src/net/udp.rs
--- a/tokio/src/net/udp.rs
+++ b/tokio/src/net/udp.rs
@@ -1,5 +1,5 @@
-            .async_io(Interest::READABLE, || self.io.recv(buf))
+            .async_io(Interest::READABLE | Interest::ERROR, || self.io.recv(buf))
diff --git a/tokio/tests/udp.rs b/tokio/tests/udp.rs
--- a/tokio/tests/udp.rs
+++ b/tokio/tests/udp.rs
@@ -1,3 +1,8 @@
+#[tokio::test]
+async fn send_to_recv_closed_returns_err() {
+    let err = sender.recv(&mut recv_buf).await.unwrap_err();
+    assert!(err.kind() == io::ErrorKind::ConnectionRefused);
+}
"""

DROP_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base=base
  head=head
  changed_files=2

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
  evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h1/dispatch.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=drop_error_propagation_changed subject=sender.send_error(crate::Error::new_incomplete()) source=semantic_diff confidence=high value=direct
  file=src/proto/h1/dispatch.rs

History impact:
  evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
"""

DROP_DIFF = """diff --git a/src/proto/h1/dispatch.rs b/src/proto/h1/dispatch.rs
--- a/src/proto/h1/dispatch.rs
+++ b/src/proto/h1/dispatch.rs
@@ -1,5 +1,19 @@
-    body_tx: Option<crate::body::Sender>,
+    body_tx: SenderDropGuard,
+
+struct SenderDropGuard(Option<crate::body::Sender>);
+
+impl Drop for SenderDropGuard {
+    fn drop(&mut self) {
+        if let Some(mut sender) = self.0.take() {
+            sender.send_error(crate::Error::new_incomplete());
+        }
+    }
+}
diff --git a/tests/client.rs b/tests/client.rs
--- a/tests/client.rs
+++ b/tests/client.rs
@@ -1,3 +1,9 @@
+#[tokio::test]
+async fn dropped_conn_sends_incomplete_body_error() {
+    let err = res.body_mut().frame().await.expect("body frame").unwrap_err();
+    assert!(err.is_incomplete_message(), "{err:?}");
+}
"""


class RiskWrapperTest(unittest.TestCase):
    def test_error_propagation_wrapper_groups_direct_and_supporting_evidence(self) -> None:
        bundle = build_risk_wrapper.build_bundle("serde-rs-json-757", SERDE_STDOUT, SERDE_DIFF, {"title": "fix raw utf8"})

        self.assertEqual("risk-context-wrapper.v1", bundle["version"])
        wrapper = bundle["wrappers"][0]
        self.assertEqual("ErrorPropagationWrapper", wrapper["wrapper_type"])
        self.assertEqual("panic_to_error_propagation_changed", wrapper["primary_evidence"][0]["kind"])
        self.assertEqual("prod_change_with_tests", wrapper["supporting_evidence"][0]["kind"])
        self.assertIn("str::from_utf8(raw).unwrap()", "\n".join(wrapper["wrapper_context"]["before_panic_lines"]))
        self.assertIn("InvalidUnicodeCodePoint", "\n".join(wrapper["wrapper_context"]["after_error_lines"]))
        self.assertEqual(["raw"], wrapper["wrapper_context"]["shared_input_tokens"])
        self.assertIn("test_raw_invalid_utf8", "\n".join(wrapper["wrapper_context"]["related_test_lines"]))
        self.assertIn("Use only evidence and context in this wrapper.", wrapper["analysis_contract"]["rules"])

        review = wrapper["honest_review"]
        self.assertEqual("honest-review.v1", review["version"])
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertIn("full public API contract is not available in this wrapper", review["dimensions"]["impact"]["unknown"])
        self.assertEqual("review_worthy_behavior_change", review["overall"]["judgement"])
        self.assertIn("not evidence that this PR introduced a bug", review["overall"]["non_claims"])
        self.assertIn(
            "removed panic-like error handling: code contains `let raw = str::from_utf8(raw).unwrap()`",
            review["dimensions"]["mechanism"]["observed"],
        )

    def test_error_propagation_honest_review_does_not_confirm_without_same_input(self) -> None:
        bundle = build_risk_wrapper.build_bundle("negative", UNRELATED_ERROR_STDOUT, UNRELATED_ERROR_DIFF, None)

        wrapper = bundle["wrappers"][0]
        review = wrapper["honest_review"]
        self.assertEqual([], wrapper["wrapper_context"]["shared_input_tokens"])
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertIn("whether both changes handle the same input/error path", review["dimensions"]["mechanism"]["unknown"])
        self.assertNotEqual("review_worthy_behavior_change", review["overall"]["judgement"])

    def test_error_propagation_honest_review_does_not_claim_test_control_without_test_hunk(self) -> None:
        bundle = build_risk_wrapper.build_bundle("no-test", SERDE_STDOUT, NO_TEST_ERROR_DIFF, None)

        wrapper = bundle["wrappers"][0]
        review = wrapper["honest_review"]
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("unknown", review["dimensions"]["control"]["status"])
        self.assertIn("no related test hunk was included in the wrapper", review["dimensions"]["control"]["unknown"])
        self.assertIn("not proof of complete test coverage", review["overall"]["non_claims"])

    def test_general_error_propagation_wrapper_is_partial_not_confirmed(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyper-3275",
            ERROR_FLOW_STDOUT,
            ERROR_FLOW_DIFF,
            {"title": "fix(client): early server response shouldn't propagate NO_ERROR"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ErrorPropagationWrapper", wrapper["wrapper_type"])
        self.assertEqual(
            "Did this diff change error propagation or terminal error handling?",
            wrapper["risk_question"],
        )
        self.assertEqual("error_propagation_changed", wrapper["primary_evidence"][0]["kind"])
        self.assertIn("Some(Err(e))", "\n".join(wrapper["wrapper_context"]["before_error_flow_lines"]))
        self.assertIn("Reason::NO_ERROR", "\n".join(wrapper["wrapper_context"]["after_error_flow_lines"]))
        review = wrapper["honest_review"]
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertIn(
            "removed error-flow behavior: code contains `Some(Err(e)) => return Poll::Ready(Some(Err(crate::Error::new_body(e))))`",
            review["dimensions"]["mechanism"]["observed"],
        )
        self.assertIn(
            "whether this changes caller-visible behavior is not proven by the wrapper alone",
            review["dimensions"]["mechanism"]["unknown"],
        )
        self.assertNotIn(
            "impact likely reaches a caller-visible deserialization path",
            review["dimensions"]["impact"]["inferred"],
        )
        self.assertIn(
            "PR metadata is consistent with protocol error or terminal-stream behavior change",
            review["dimensions"]["impact"]["inferred"],
        )

    def test_integer_boundary_wrapper_extracts_conversion_context(self) -> None:
        bundle = build_risk_wrapper.build_bundle("hyper-4056", HYPER_STDOUT, HYPER_DIFF, None)

        wrapper = bundle["wrappers"][0]
        self.assertEqual("IntegerBoundaryWrapper", wrapper["wrapper_type"])
        self.assertEqual("integer_platform_boundary_changed", wrapper["primary_evidence"][0]["kind"])
        self.assertIn("*remaining as usize", "\n".join(wrapper["wrapper_context"]["before_conversion_lines"]))
        self.assertIn("usize::try_from", "\n".join(wrapper["wrapper_context"]["after_conversion_lines"]))
        self.assertTrue(wrapper["wrapper_context"]["platform_hints"])
        self.assertEqual(["remaining"], wrapper["wrapper_context"]["shared_integer_tokens"])
        review = wrapper["honest_review"]
        self.assertEqual("integer_boundary_change", review["risk_item"])
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("unknown", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_boundary_change", review["overall"]["judgement"])
        self.assertIn("not proof of 32-bit behavior without target-specific execution", review["overall"]["non_claims"])

    def test_integer_boundary_control_is_partial_only_with_boundary_test_signal(self) -> None:
        bundle = build_risk_wrapper.build_bundle("hyper-4056", HYPER_STDOUT, HYPER_WITH_TEST_DIFF, None)

        review = bundle["wrappers"][0]["honest_review"]
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertIn("test execution on relevant target architectures is not proven", review["dimensions"]["control"]["unknown"])

    def test_integer_boundary_honest_review_does_not_confirm_after_only_conversion(self) -> None:
        bundle = build_risk_wrapper.build_bundle("after-only", HYPER_STDOUT, HYPER_AFTER_ONLY_DIFF, None)

        wrapper = bundle["wrappers"][0]
        review = wrapper["honest_review"]
        self.assertEqual([], wrapper["wrapper_context"]["before_conversion_lines"])
        self.assertEqual("weak", review["dimensions"]["mechanism"]["status"])
        self.assertIn("missing one side of the before/after integer boundary change", review["dimensions"]["mechanism"]["unknown"])

    def test_state_transition_wrapper_extracts_state_tokens(self) -> None:
        bundle = build_risk_wrapper.build_bundle("tokio-rs-tokio-8021", TOKIO_STDOUT, TOKIO_DIFF, None)

        wrapper = bundle["wrappers"][0]
        self.assertEqual("StateTransitionWrapper", wrapper["wrapper_type"])
        self.assertEqual(["CLOSED"], wrapper["wrapper_context"]["state_tokens"])
        self.assertIn("Self::CLOSED", "\n".join(wrapper["wrapper_context"]["after_state_lines"]))
        self.assertIn("Self::PERMIT_SHIFT", "\n".join(wrapper["wrapper_context"]["before_related_lines"]))
        review = wrapper["honest_review"]
        self.assertEqual("state_transition_change", review["risk_item"])
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_state_transition_change", review["overall"]["judgement"])
        self.assertIn("not proof of all concurrent interleavings", review["overall"]["non_claims"])

    def test_state_transition_wrapper_stays_partial_for_cancel_path_without_full_before_side(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyper-4042",
            STATE_CANCEL_STDOUT,
            STATE_CANCEL_DIFF,
            {"title": "fix(http2): cancel pipe_task and send RST_STREAM on response future drop"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("StateTransitionWrapper", wrapper["wrapper_type"])
        self.assertIn("cancel", " ".join(wrapper["wrapper_context"]["state_tokens"]).lower())
        review = wrapper["honest_review"]
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("needs_more_context_before_review_claim", review["overall"]["judgement"])

    def test_lifecycle_ordering_wrapper_confirms_graceful_shutdown_upgrade_guard(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyperium-hyper-3616",
            LIFECYCLE_UPGRADE_STDOUT,
            LIFECYCLE_UPGRADE_DIFF,
            {"title": "fix: avoid panic when calling graceful_shutdown if H1 connection was upgraded"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("LifecycleOrderingWrapper", wrapper["wrapper_type"])
        self.assertIn("graceful_shutdown", " ".join(wrapper["wrapper_context"]["lifecycle_tokens"]).lower())
        review = wrapper["honest_review"]
        self.assertEqual("lifecycle_ordering_change", review["risk_item"])
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_lifecycle_ordering_change", review["overall"]["judgement"])
        self.assertIn("not proof of all async cleanup interleavings", review["overall"]["non_claims"])

    def test_lifecycle_ordering_wrapper_confirms_initial_state_shutdown_fix(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyperium-hyper-3261",
            LIFECYCLE_INITIAL_STDOUT,
            LIFECYCLE_INITIAL_DIFF,
            {"title": "fix(http1): http1 server graceful shutdown fix"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("LifecycleOrderingWrapper", wrapper["wrapper_type"])
        self.assertIn("has_initial_read_write_state", "\n".join(wrapper["wrapper_context"]["after_lifecycle_lines"]))
        review = wrapper["honest_review"]
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_lifecycle_ordering_change", review["overall"]["judgement"])

    def test_lifecycle_ordering_wrapper_stays_partial_for_cancel_reset_cleanup(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyper-4042-lifecycle",
            LIFECYCLE_CANCEL_STDOUT,
            LIFECYCLE_CANCEL_DIFF,
            {"title": "fix(http2): cancel pipe_task and send RST_STREAM on response future drop"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("LifecycleOrderingWrapper", wrapper["wrapper_type"])
        review = wrapper["honest_review"]
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("needs_more_context_before_review_claim", review["overall"]["judgement"])

    def test_concurrency_ordering_wrapper_confirms_blocking_pool_race_fix(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "tokio-rs-tokio-7922",
            CONCURRENCY_BLOCKING_STDOUT,
            CONCURRENCY_BLOCKING_DIFF,
            {"title": "runtime: fix race condition during the blocking pool shutdown"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ConcurrencyOrderingWrapper", wrapper["wrapper_type"])
        self.assertIn("is_counted_idle", " ".join(wrapper["wrapper_context"]["concurrency_tokens"]))
        review = wrapper["honest_review"]
        self.assertEqual("concurrency_ordering_change", review["risk_item"])
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_concurrency_ordering_change", review["overall"]["judgement"])

    def test_concurrency_ordering_wrapper_confirms_wakeup_repoll_fix(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyperium-hyper-3952",
            CONCURRENCY_WAKEUP_STDOUT,
            CONCURRENCY_WAKEUP_DIFF,
            {"title": "fix(http1): poll_loop writes when ready"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ConcurrencyOrderingWrapper", wrapper["wrapper_type"])
        self.assertIn("wants_write_again", "\n".join(wrapper["wrapper_context"]["after_concurrency_lines"]))
        review = wrapper["honest_review"]
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_concurrency_ordering_change", review["overall"]["judgement"])

    def test_concurrency_ordering_wrapper_stays_partial_for_completed_before_repoll_cleanup(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "tokio-rs-tokio-7983",
            CONCURRENCY_COMPLETED_STDOUT,
            CONCURRENCY_COMPLETED_DIFF,
            {"title": "rt: do not leak fd when cancelling io_uring open operation"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ConcurrencyOrderingWrapper", wrapper["wrapper_type"])
        review = wrapper["honest_review"]
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("needs_more_context_before_review_claim", review["overall"]["judgement"])

    def test_external_contract_wrapper_confirms_host_header_contract_fix(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyper-3621",
            EXTERNAL_HOST_STDOUT,
            EXTERNAL_HOST_DIFF,
            {"title": "fix: Do not include user information in Host header"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ExternalContractWrapper", wrapper["wrapper_type"])
        review = wrapper["honest_review"]
        self.assertEqual("external_contract_change", review["risk_item"])
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("unknown", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_external_contract_change", review["overall"]["judgement"])

    def test_external_contract_wrapper_confirms_command_argument_separator_fix(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "ripgrep-3224",
            EXTERNAL_COMMAND_STDOUT,
            EXTERNAL_COMMAND_DIFF,
            {"title": "Separate flags from paths in decompression tool invocations"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ExternalContractWrapper", wrapper["wrapper_type"])
        self.assertIn("ARGS_GZIP", " ".join(wrapper["wrapper_context"]["contract_tokens"]))
        review = wrapper["honest_review"]
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("review_worthy_external_contract_change", review["overall"]["judgement"])

    def test_external_contract_wrapper_confirms_public_keep_alive_api_addition(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyper-3936",
            EXTERNAL_KEEPALIVE_STDOUT,
            EXTERNAL_KEEPALIVE_DIFF,
            {"title": "fix(http1): add http10_disable_keep_alive() for server and client"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ExternalContractWrapper", wrapper["wrapper_type"])
        review = wrapper["honest_review"]
        self.assertEqual("confirmed", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("review_worthy_external_contract_change", review["overall"]["judgement"])

    def test_external_contract_wrapper_stays_partial_for_generic_public_api_surface(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyperium-hyper-518",
            EXTERNAL_API_BOUNDARY_STDOUT,
            EXTERNAL_API_BOUNDARY_DIFF,
            {"title": "fix(client): keep the underlying connector when setting an SSL verifier"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ExternalContractWrapper", wrapper["wrapper_type"])
        review = wrapper["honest_review"]
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("needs_more_context_before_review_claim", review["overall"]["judgement"])

    def test_error_propagation_case_does_not_also_create_state_transition_wrapper(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyperium-hyper-3275",
            ERROR_FLOW_STDOUT,
            ERROR_FLOW_DIFF,
            {"title": "fix(client): early server response shouldn't propagate NO_ERROR"},
        )

        wrappers = [wrapper["wrapper_type"] for wrapper in bundle["wrappers"]]
        self.assertEqual(["ErrorPropagationWrapper"], wrappers)

    def test_same_file_direct_error_evidence_is_grouped_into_one_wrapper(self) -> None:
        bundle = build_risk_wrapper.build_bundle("hyper-3998", MULTI_PRIMARY_STDOUT, MULTI_PRIMARY_DIFF, None)

        wrappers = [w for w in bundle["wrappers"] if w["wrapper_type"] == "ErrorPropagationWrapper"]
        self.assertEqual(1, len(wrappers))
        self.assertEqual(2, len(wrappers[0]["primary_evidence"]))
        self.assertEqual(
            {"return Poll::Ready(None);", "Poll::Ready(None)"},
            {item["subject"] for item in wrappers[0]["primary_evidence"]},
        )

    def test_readiness_error_wrapper_routes_to_error_propagation_and_stays_partial(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "tokio-rs-tokio-8001",
            READINESS_STDOUT,
            READINESS_DIFF,
            {"title": "Surface errors from SO_ERROR on recv for UDP sockets on Linux"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ErrorPropagationWrapper", wrapper["wrapper_type"])
        self.assertEqual("readiness_error_propagation_changed", wrapper["primary_evidence"][0]["kind"])
        self.assertIn("Interest::READABLE", "\n".join(wrapper["wrapper_context"]["before_readiness_lines"]))
        self.assertIn("Interest::ERROR", "\n".join(wrapper["wrapper_context"]["after_readiness_lines"]))
        review = wrapper["honest_review"]
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertIn(
            "readiness registration now includes explicit error wakeups for the same receive path",
            review["dimensions"]["mechanism"]["inferred"],
        )
        self.assertEqual("partial", review["dimensions"]["impact"]["status"])
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("needs_more_context_before_review_claim", review["overall"]["judgement"])

    def test_drop_error_wrapper_routes_to_error_propagation_and_stays_partial(self) -> None:
        bundle = build_risk_wrapper.build_bundle(
            "hyperium-hyper-4069",
            DROP_STDOUT,
            DROP_DIFF,
            {"title": "fix(http1): send error when dispatcher is dropped mid-body"},
        )

        wrapper = bundle["wrappers"][0]
        self.assertEqual("ErrorPropagationWrapper", wrapper["wrapper_type"])
        self.assertEqual("drop_error_propagation_changed", wrapper["primary_evidence"][0]["kind"])
        self.assertIn("body_tx: Option<crate::body::Sender>", "\n".join(wrapper["wrapper_context"]["before_drop_lines"]))
        self.assertIn("send_error(crate::Error::new_incomplete())", "\n".join(wrapper["wrapper_context"]["after_drop_lines"]))
        review = wrapper["honest_review"]
        self.assertEqual("partial", review["dimensions"]["mechanism"]["status"])
        self.assertIn(
            "drop path now sends an explicit incomplete-message error",
            review["dimensions"]["mechanism"]["inferred"],
        )
        self.assertEqual("partial", review["dimensions"]["control"]["status"])
        self.assertEqual("needs_more_context_before_review_claim", review["overall"]["judgement"])

    def test_cli_writes_bundle_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            stdout = tmp_path / "decay.stdout"
            diff = tmp_path / "diff.patch"
            out = tmp_path / "bundle.json"
            stdout.write_text(SERDE_STDOUT)
            diff.write_text(SERDE_DIFF)

            code = build_risk_wrapper.main.__globals__["main"]
            self.assertTrue(callable(code))
            bundle = build_risk_wrapper.build_bundle("case", stdout.read_text(), diff.read_text(), None)
            out.write_text(json.dumps(bundle))
            self.assertTrue(out.exists())


if __name__ == "__main__":
    unittest.main()
