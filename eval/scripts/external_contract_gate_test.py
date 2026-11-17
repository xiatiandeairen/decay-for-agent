#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "run-external-contract-gate.py"
SPEC = importlib.util.spec_from_file_location("run_external_contract_gate", SCRIPT)
assert SPEC and SPEC.loader
run_external_contract_gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(run_external_contract_gate)


def contract_stdout(stability_block: str = "", semantic_block: str = "") -> str:
    stability_count = len([line for line in stability_block.splitlines() if line.startswith("- id=")])
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
  evidence={stability_count}
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
{stability_block}

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


POSITIVE_STDOUT = contract_stdout(
    semantic_block="""- id=1 category=semantic kind=protocol_header_authority_changed subject=Host header authority -> host/port source=semantic_diff confidence=high value=direct
  file=examples/client.rs""",
)

POSITIVE_DIFF = """diff --git a/examples/client.rs b/examples/client.rs
--- a/examples/client.rs
+++ b/examples/client.rs
@@ -1,6 +1,6 @@
-    let authority = url.authority().unwrap().clone();
-    .header(hyper::header::HOST, authority.as_str())
+    let host = url.host().expect("uri has no host");
+    .header(hyper::header::HOST, format!("{}:{}", host, port))
"""

BOUNDARY_STDOUT = contract_stdout(
    stability_block="""- id=1 category=stability kind=public_api_added subject=ChannelMockConnector source=stability confidence=high value=direct
  file=src/mock.rs
- id=2 category=stability kind=public_api_signature_changed subject=new source=stability confidence=high value=direct
  file=src/mock.rs""",
)

BOUNDARY_DIFF = """diff --git a/src/mock.rs b/src/mock.rs
--- a/src/mock.rs
+++ b/src/mock.rs
@@ -1,5 +1,9 @@
+pub struct ChannelMockConnector {
+    calls: Sender<String>,
+}
+    pub fn new(calls: Sender<String>) -> ChannelMockConnector {
+        ChannelMockConnector { calls: calls }
+    }
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


class ExternalContractGateTest(unittest.TestCase):
    def test_evaluate_reports_precision_recall_and_boundary_honesty(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pack = root / "pack.toml"
            pack.write_text(
                """
version = "external-contract-gate.v1"
risk_item = "external_contract_change"

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
expected_judgement = "review_worthy_external_contract_change"
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

            result = run_external_contract_gate.evaluate(pack, root / "cases")

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
        (raw / "pr.json").write_text(json.dumps({"title": "Fix external contract"}))


if __name__ == "__main__":
    unittest.main()
