#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "validate-honest-review-analysis.py"
SPEC = importlib.util.spec_from_file_location("validate_honest_review", SCRIPT)
assert SPEC and SPEC.loader
validate_honest_review = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validate_honest_review)


BUNDLE = {
    "wrappers": [
        {
            "wrapper_id": "w1",
            "honest_review": {
                "risk_item": "integer_boundary_change",
                "dimensions": {
                    "mechanism": {
                        "status": "confirmed",
                        "observed": ["removed cast", "added try_from"],
                        "inferred": ["strategy changed"],
                        "unknown": ["runtime behavior not executed"],
                    },
                    "impact": {
                        "status": "partial",
                        "observed": ["boundary role tokens: remaining"],
                        "inferred": ["likely affects read size"],
                        "unknown": ["full dataflow unavailable"],
                    },
                    "control": {
                        "status": "unknown",
                        "observed": [],
                        "inferred": [],
                        "unknown": ["no related test hunk"],
                    },
                },
                "overall": {
                    "non_claims": [
                        "not evidence that this PR introduced a bug",
                        "not proof of complete test coverage",
                    ]
                },
            },
        }
    ]
}


def valid_analysis() -> dict:
    return {
        "version": "honest-review-analysis.v1",
        "items": [
            {
                "wrapper_id": "w1",
                "risk_item": "integer_boundary_change",
                "dimension_judgements": {
                    "mechanism": {
                        "status": "confirmed",
                        "supported_by": ["removed cast", "strategy changed"],
                        "limits": ["runtime behavior not executed"],
                    },
                    "impact": {
                        "status": "partial",
                        "supported_by": ["likely affects read size"],
                        "limits": ["full dataflow unavailable"],
                    },
                    "control": {
                        "status": "unknown",
                        "supported_by": [],
                        "limits": ["no related test hunk"],
                    },
                },
                "overall_judgement": "review_worthy_boundary_change",
                "non_claims": [
                    "not evidence that this PR introduced a bug",
                    "not proof of complete test coverage",
                ],
                "uncertainties": ["no related test hunk"],
                "recommended_next_check": "Add or inspect related test hunk for the integer boundary.",
            }
        ],
    }


class HonestReviewAnalysisValidatorTest(unittest.TestCase):
    def test_valid_analysis_passes(self) -> None:
        self.assertEqual([], validate_honest_review.validate(BUNDLE, valid_analysis()))

    def test_status_upgrade_fails(self) -> None:
        analysis = valid_analysis()
        analysis["items"][0]["dimension_judgements"]["control"]["status"] = "confirmed"

        errors = validate_honest_review.validate(BUNDLE, analysis)

        self.assertTrue(any("exceeds honest_review status" in error for error in errors))

    def test_missing_non_claim_fails(self) -> None:
        analysis = valid_analysis()
        analysis["items"][0]["non_claims"] = ["not proof of complete test coverage"]

        errors = validate_honest_review.validate(BUNDLE, analysis)

        self.assertTrue(any("non_claims missing" in error for error in errors))

    def test_unsupported_fact_fails(self) -> None:
        analysis = valid_analysis()
        analysis["items"][0]["dimension_judgements"]["impact"]["supported_by"].append("this is a confirmed 32-bit bug")

        errors = validate_honest_review.validate(BUNDLE, analysis)

        self.assertTrue(any("supported_by is not in observed/inferred" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
