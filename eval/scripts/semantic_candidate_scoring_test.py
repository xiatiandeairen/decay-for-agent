#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("score-semantic-candidates.py")
SPEC = importlib.util.spec_from_file_location("score_semantic_candidates", SCRIPT)
assert SPEC and SPEC.loader
score_semantic_candidates = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(score_semantic_candidates)


class SemanticCandidateScoringTest(unittest.TestCase):
    def test_panic_regression_with_tests_scores_high(self) -> None:
        row = {
            "title": "fix(client): panic when pool idle timeout set to zero",
            "body": "Fixes #123. Adds a regression test for the incorrect behavior.",
            "labels": ["bug"],
            "seed_keywords": ["panic", "regression"],
            "comments_count": 2,
        }

        total, reasons = score_semantic_candidates.score(row)

        self.assertGreaterEqual(total, 18)
        self.assertIn("+3 high_value_title", reasons)
        self.assertIn("+2 context:issue_linked", reasons)

    def test_dependency_release_note_noise_is_downranked(self) -> None:
        row = {
            "title": "build(deps): bump some-action from 1 to 3",
            "body": "Release notes mention security, breaking changes, panic fixes, and regressions.",
            "labels": ["dependencies", "github_actions"],
            "seed_keywords": ["security", "regression"],
            "comments_count": 0,
        }

        total, reasons = score_semantic_candidates.score(row)

        self.assertLess(total, 10)
        self.assertIn("-12 automation_dependency", reasons)


if __name__ == "__main__":
    unittest.main()
