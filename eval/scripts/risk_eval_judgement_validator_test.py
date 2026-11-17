#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "validate-risk-eval-judgement.py"
SPEC = importlib.util.spec_from_file_location("validate_risk_eval_judgement", SCRIPT)
assert SPEC and SPEC.loader
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


HOLDOUT_CASE = {
    "id": "serde-rs-json-757",
    "candidate_status": "freeze_ready",
    "gate_role": "high_value_positive",
    "high_value": True,
    "primary_signal_ids": ["error_propagation", "assumption_boundary"],
    "expected_behavior": "detect",
    "why_in_holdout": "Invalid UTF-8 panic-to-error transition is clear and scoped.",
    "forbidden_claims": [
        "do not generalize from RawValue invalid UTF-8 to all serde_json UTF-8 policy",
        "do not claim complete malformed-input coverage",
    ],
    "required_context": ["issue_link_or_repro", "production_diff", "regression_test"],
}


def valid_complete() -> dict:
    return {
        "version": "risk-eval-judgement.v1",
        "case_id": "serde-rs-json-757",
        "repo": "serde-rs/json",
        "pr": 757,
        "status": "complete",
        "judge": "local",
        "judged_at": "2026-06-02T10:00:00Z",
        "ground_truth_summary": "Invalid UTF-8 RawValue panic was converted to error return.",
        "ground_truth": {
            "gate_role": "high_value_positive",
            "high_value": True,
            "primary_signal_ids": ["error_propagation", "assumption_boundary"],
            "expected_behavior": "detect",
            "forbidden_claims": [
                "do not generalize from RawValue invalid UTF-8 to all serde_json UTF-8 policy",
                "do not claim complete malformed-input coverage",
            ],
            "required_context": ["issue_link_or_repro", "production_diff", "regression_test"],
            "notes": "",
        },
        "fact_only": {
            "top3_total": 3,
            "top3_useful": 1,
            "actionable": True,
            "missed_high_risk_count": 0,
            "weak_only_top_risk_count": 1,
            "detection": "partial",
            "value_verdict": "plausible",
            "evidence_valid": True,
            "overclaim": False,
            "forbidden_claim_breached": False,
            "unsupported_scope_silence": False,
            "next_check_useful": False,
            "notes": "",
        },
        "semantic_filtered": {
            "top3_total": 3,
            "top3_useful": 2,
            "actionable": True,
            "missed_high_risk_count": 0,
            "weak_only_top_risk_count": 0,
            "detection": "hit",
            "value_verdict": "useful",
            "evidence_valid": True,
            "overclaim": False,
            "forbidden_claim_breached": False,
            "unsupported_scope_silence": False,
            "next_check_useful": True,
            "notes": "",
        },
        "comparison": {
            "semantic_improved_ranking": True,
            "semantic_reduced_noise": True,
            "semantic_introduced_noise": False,
            "semantic_net_value": "better",
            "notes": "",
        },
        "gate": {
            "ground_truth_complete": True,
            "forbidden_claims_checked": True,
            "holdout_eligible": True,
            "notes": "",
        },
    }


class RiskEvalJudgementValidatorTest(unittest.TestCase):
    def test_complete_judgement_passes(self) -> None:
        self.assertEqual([], validator.validate_judgement(valid_complete(), HOLDOUT_CASE))

    def test_pending_template_with_blanks_passes(self) -> None:
        data = valid_complete()
        data["status"] = "pending"
        data["judge"] = ""
        data["judged_at"] = ""
        data["ground_truth_summary"] = ""
        data["ground_truth"]["gate_role"] = ""
        data["ground_truth"]["primary_signal_ids"] = []
        data["ground_truth"]["expected_behavior"] = ""
        data["ground_truth"]["forbidden_claims"] = []
        data["ground_truth"]["required_context"] = []
        data["fact_only"]["detection"] = ""
        data["fact_only"]["value_verdict"] = ""
        data["semantic_filtered"]["detection"] = ""
        data["semantic_filtered"]["value_verdict"] = ""
        data["comparison"]["semantic_net_value"] = ""
        data["gate"]["ground_truth_complete"] = False
        data["gate"]["forbidden_claims_checked"] = False
        data["gate"]["holdout_eligible"] = False

        self.assertEqual([], validator.validate_judgement(data, HOLDOUT_CASE))

    def test_missing_holdout_forbidden_claim_fails(self) -> None:
        data = valid_complete()
        data["ground_truth"]["forbidden_claims"] = ["do not claim complete malformed-input coverage"]

        errors = validator.validate_judgement(data, HOLDOUT_CASE)

        self.assertTrue(any("missing holdout contract claims" in error for error in errors))

    def test_overclaim_requires_forbidden_claim_breach(self) -> None:
        data = valid_complete()
        data["semantic_filtered"]["overclaim"] = True

        errors = validator.validate_judgement(data, HOLDOUT_CASE)

        self.assertTrue(any("overclaim requires forbidden_claim_breached" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
