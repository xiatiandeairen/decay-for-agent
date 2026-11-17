#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "decay_evidence.py"
SPEC = importlib.util.spec_from_file_location("decay_evidence", SCRIPT)
assert SPEC and SPEC.loader
decay_evidence = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(decay_evidence)


VALID_STDOUT = """Diff context:
  repo_root=/tmp/repo
  mode=git_ref
  base_input=HEAD
  base=abc
  head=def
  changed_files=1

Top risk findings:
1. [high] src/lib.rs
   semantic error_propagation_changed
   action: inspect changed path

Complexity impact:
  changed_functions=0 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk

Coverage impact:
  evidence=1
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
- id=1 category=coverage kind=parse_coverage_gap subject=decay scan coverage is partial source=scan confidence=high value=coverage_limit
  file=src/lib.rs

Stability impact:
  evidence=1
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=2 category=stability kind=panic_like_added subject=unwrap added source=stability confidence=high value=direct
  file=src/lib.rs

Quality impact:
  evidence=1 prod_files_changed=1 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=3 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=src/lib.rs

Semantic impact:
  evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=4 category=semantic kind=error_propagation_changed subject=Err(e) => Poll::Ready(None) source=semantic_diff confidence=medium value=direct
  file=src/lib.rs

History impact:
  evidence=1
  value_note=context only; excluded from top risk unless direct diff evidence exists
- id=5 category=history kind=recently_bugfixed_area subject=fix: parser panic source=git_history confidence=low value=context
  file=src/lib.rs
"""


class DecayEvidenceTest(unittest.TestCase):
    def test_parse_stdout_extracts_git_context_and_evidence(self) -> None:
        report = decay_evidence.parse_stdout(VALID_STDOUT)

        self.assertEqual("decay-evidence.v1", report["version"])
        self.assertEqual("git_ref", report["git_context"]["mode"])
        self.assertEqual(1, report["git_context"]["changed_files"])
        self.assertEqual([1, 2, 3, 4, 5], [item["id"] for item in report["evidence"]])
        self.assertEqual("src/lib.rs", report["evidence"][0]["file"])
        self.assertEqual("supporting", report["evidence"][2]["value"])

    def test_parse_stdout_rejects_missing_confidence_and_value(self) -> None:
        invalid = VALID_STDOUT.replace(
            "source=quality confidence=low value=supporting",
            "source=quality",
            1,
        )
        with self.assertRaisesRegex(decay_evidence.EvidenceContractError, "confidence/value required"):
            decay_evidence.parse_stdout(invalid)

    def test_parse_stdout_rejects_wrong_coverage_value(self) -> None:
        invalid = VALID_STDOUT.replace("confidence=high value=coverage_limit", "confidence=medium value=direct", 1)
        with self.assertRaisesRegex(decay_evidence.EvidenceContractError, "coverage evidence must be"):
            decay_evidence.parse_stdout(invalid)

    def test_parse_stdout_rejects_non_contiguous_ids(self) -> None:
        invalid = VALID_STDOUT.replace("- id=5", "- id=7", 1)
        with self.assertRaisesRegex(decay_evidence.EvidenceContractError, "must be contiguous"):
            decay_evidence.parse_stdout(invalid)

    def test_parse_stdout_rejects_missing_section_header(self) -> None:
        invalid = VALID_STDOUT.replace(
            """History impact:
  evidence=1
  value_note=context only; excluded from top risk unless direct diff evidence exists
- id=5 category=history kind=recently_bugfixed_area subject=fix: parser panic source=git_history confidence=low value=context
  file=src/lib.rs
""",
            "",
            1,
        )
        with self.assertRaisesRegex(decay_evidence.EvidenceContractError, "missing section headers"):
            decay_evidence.parse_stdout(invalid)


if __name__ == "__main__":
    unittest.main()
