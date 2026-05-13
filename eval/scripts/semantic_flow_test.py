#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import tempfile
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


build_prompt = load_script("build-semantic-prompt.py")
validator = load_script("validate-semantic-findings.py")


SAMPLE_STDOUT = """Diff context:
  mode=git_ref

Top risk findings:
1. [high] src/lib.rs
   semantic default_value_changed DEFAULT_TIMEOUT: 30 -> 5
   action: review behavior boundary change

Complexity impact:
  changed_functions=1 evidence=0

Stability impact:
  evidence=0

Quality impact:
  evidence=1
- id=1 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=src/lib.rs

Semantic impact:
  evidence=1
- id=2 category=semantic kind=default_value_changed subject=DEFAULT_TIMEOUT: 30 -> 5 source=semantic_diff
  file=src/lib.rs

History impact:
  evidence=0
"""


class SemanticFlowTest(unittest.TestCase):
    def test_prompt_contains_allowed_evidence_and_weak_signal_rule(self) -> None:
        prompt = build_prompt.build_prompt(SAMPLE_STDOUT)

        self.assertIn("# Allowed Evidence Ids\n\n1, 2", prompt)
        self.assertIn("Do not promote weak-only signals", prompt)
        self.assertIn("Semantic impact", prompt)

    def test_validator_accepts_source_refs_to_existing_evidence(self) -> None:
        payload = {
            "version": "semantic-findings.v1",
            "findings": [
                {
                    "claim": "Default timeout changed.",
                    "supporting_facts": ["id=2 default value changed from 30 to 5"],
                    "source_refs": [{"type": "evidence", "id": 2}],
                    "confidence": "high",
                    "limitation": "Does not prove the new timeout is incorrect.",
                    "review_action": "Review timeout compatibility tests.",
                }
            ],
        }

        validator.validate(SAMPLE_STDOUT, payload)

    def test_validator_rejects_unlinked_claim(self) -> None:
        payload = {
            "version": "semantic-findings.v1",
            "findings": [
                {
                    "claim": "Unlinked claim.",
                    "supporting_facts": ["unsupported"],
                    "source_refs": [{"type": "evidence", "id": 9}],
                    "confidence": "low",
                    "limitation": "none",
                    "review_action": "review",
                }
            ],
        }

        with self.assertRaisesRegex(ValueError, "not in fact evidence"):
            validator.validate(SAMPLE_STDOUT, payload)

    def test_validator_cli_reads_json_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            stdout_path = tmp_path / "decay.stdout"
            json_path = tmp_path / "semantic.json"
            stdout_path.write_text(SAMPLE_STDOUT)
            json_path.write_text(json.dumps({"version": "semantic-findings.v1", "findings": []}))

            self.assertEqual(validator.main.__globals__["Path"](stdout_path), stdout_path)
            self.assertEqual(validator.load_json(json_path)["version"], "semantic-findings.v1")


if __name__ == "__main__":
    unittest.main()
