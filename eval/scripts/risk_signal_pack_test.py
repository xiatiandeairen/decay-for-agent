#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("validate-risk-signal-pack.py")
SPEC = importlib.util.spec_from_file_location("validate_risk_signal_pack", SCRIPT)
assert SPEC and SPEC.loader
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

ROOT = Path(__file__).resolve().parents[1]
PACK = ROOT / "risk-signals"


class RiskSignalPackTest(unittest.TestCase):
    def test_current_pack_is_valid(self) -> None:
        self.assertEqual([], validator.validate_pack(PACK))

    def test_unknown_dev_signal_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "dev-cases.toml"
            text = path.read_text()
            path.write_text(text.replace('signal_ids = ["assumption_boundary"]', 'signal_ids = ["not_a_signal"]', 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("unknown signal not_a_signal" in error for error in errors))

    def test_negative_case_must_use_near_miss_signals(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "dev-cases.toml"
            text = path.read_text()
            text = text.replace(
                'near_miss_signals = ["assumption_boundary"]',
                'signal_ids = ["assumption_boundary"]',
                1,
            )
            path.write_text(text)

            errors = validator.validate_pack(root)

        self.assertTrue(any("negative cases must use near_miss_signals" in error for error in errors))

    def test_frozen_eval_below_target_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "eval-candidates.toml"
            text = path.read_text()
            path.write_text(text.replace('status = "freeze_ready"', 'status = "frozen"', 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("frozen eval set has 1 cases" in error for error in errors))

    def test_eval_candidate_requires_production_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "eval-candidates.toml"
            text = path.read_text()
            path.write_text(text.replace('production_files = ["tokio/src/sync/mpsc/chan.rs"]', "production_files = []", 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("production_files must be a non-empty list" in error for error in errors))

    def test_pilot_case_must_be_freeze_ready(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "eval-candidates.toml"
            text = path.read_text()
            path.write_text(text.replace('id = "tokio-rs-tokio-7991"\nstatus = "freeze_ready"', 'id = "tokio-rs-tokio-7991"\nstatus = "needs_more_context"', 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("case tokio-rs-tokio-7991 status must be freeze_ready" in error for error in errors))

    def test_pilot_actual_cases_must_match_ids(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "frozen-pilot-v1.toml"
            text = path.read_text()
            path.write_text(text.replace("actual_cases = 26", "actual_cases = 25", 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("actual_cases 25 does not match case_ids 26" in error for error in errors))

    def test_frozen_v1_building_requires_matching_gap(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "frozen-v1.toml"
            text = path.read_text()
            text = text.replace('\nstatus = "complete"', '\nstatus = "building"', 1)
            path.write_text(text.replace("missing_cases = 0", "missing_cases = 1", 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("gap.missing_cases 1 does not match target-case delta 0" in error for error in errors))

    def test_frozen_v1_complete_requires_40_cases(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "frozen-v1.toml"
            text = path.read_text()
            path.write_text(text.replace('  "tokio-rs-tokio-8076",\n', "", 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("complete set must have 40 cases, got 39" in error for error in errors))

    def test_frozen_v1_case_must_be_freeze_ready(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "eval-candidates.toml"
            text = path.read_text()
            path.write_text(text.replace('id = "tokio-rs-tokio-7991"\nstatus = "freeze_ready"', 'id = "tokio-rs-tokio-7991"\nstatus = "rejected"', 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("frozen-v1.toml: case tokio-rs-tokio-7991 status must be freeze_ready" in error for error in errors))

    def test_holdout_positive_must_reference_freeze_ready(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "holdout-v1.toml"
            text = path.read_text()
            path.write_text(text.replace('candidate_status = "freeze_ready"', 'candidate_status = "rejected"', 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("high_value_positive must reference freeze_ready case" in error for error in errors))

    def test_holdout_boundary_must_reference_needs_more_context(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "risk-signals"
            shutil.copytree(PACK, root)
            path = root / "holdout-v1.toml"
            text = path.read_text()
            old = 'id = "hyperium-hyper-3290"\ncandidate_status = "needs_more_context"'
            new = 'id = "hyperium-hyper-3290"\ncandidate_status = "freeze_ready"'
            path.write_text(text.replace(old, new, 1))

            errors = validator.validate_pack(root)

        self.assertTrue(any("boundary_needs_context must reference needs_more_context case" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
