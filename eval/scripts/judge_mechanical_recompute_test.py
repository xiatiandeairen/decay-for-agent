"""Unit tests for the mechanical recompute of `top3_useful`."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest


SCRIPTS_DIR = Path(__file__).resolve().parent


def _load():
    spec = importlib.util.spec_from_file_location(
        "mechanical_recompute_judgement",
        SCRIPTS_DIR / "mechanical-recompute-judgement.py",
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


MOD = _load()


def _r(subject: str, signal: str) -> dict:
    return {"subject": subject, "risk_signal": signal}


# ---- mechanical_top3_useful core ----

def test_double_match_counts() -> None:
    n = MOD.mechanical_top3_useful(
        [_r("src/a.rs:fn1", "error_propagation")],
        ["src/a.rs"],
        ["error_propagation"],
    )
    assert n == 1


def test_file_match_only_does_not_count() -> None:
    n = MOD.mechanical_top3_useful(
        [_r("src/a.rs:fn1", "state_transition")],
        ["src/a.rs"],
        ["error_propagation"],
    )
    assert n == 0


def test_signal_match_only_does_not_count() -> None:
    n = MOD.mechanical_top3_useful(
        [_r("src/b.rs:fn1", "error_propagation")],
        ["src/a.rs"],
        ["error_propagation"],
    )
    assert n == 0


def test_no_match_counts_zero() -> None:
    n = MOD.mechanical_top3_useful(
        [_r("src/c.rs:fn1", "lifecycle_ordering")],
        ["src/a.rs"],
        ["error_propagation"],
    )
    assert n == 0


def test_subject_with_function_suffix_strips_to_file() -> None:
    n = MOD.mechanical_top3_useful(
        [_r("src/a.rs:complex::fn::name", "error_propagation")],
        ["src/a.rs"],
        ["error_propagation"],
    )
    assert n == 1


def test_empty_ground_truth_returns_zero() -> None:
    n = MOD.mechanical_top3_useful(
        [_r("src/a.rs", "error_propagation")],
        [],
        [],
    )
    assert n == 0


def test_multiple_top_risks_all_double_match() -> None:
    n = MOD.mechanical_top3_useful(
        [
            _r("src/a.rs:fn1", "error_propagation"),
            _r("src/b.rs:fn2", "state_transition"),
            _r("src/a.rs:fn3", "state_transition"),
        ],
        ["src/a.rs", "src/b.rs"],
        ["error_propagation", "state_transition"],
    )
    assert n == 3


# ---- recompute orchestration + cross-check ----

def _judge(
    *,
    case_id="case-x",
    files=("src/a.rs",),
    signals=("error_propagation",),
    drafts=None,
):
    if drafts is None:
        drafts = {
            "fact_only": {"top3_useful": 0},
            "semantic_filtered": {"top3_useful": 0},
            "ai_baseline": {"top3_useful": 0},
        }
    return {
        "case_id": case_id,
        "ground_truth_draft": {
            "subject_files": list(files),
            "primary_signal_ids": list(signals),
        },
        "judgement_drafts": drafts,
    }


def test_recompute_agree_emits_no_flag() -> None:
    judge = _judge(
        drafts={
            "fact_only": {"top3_useful": 1},
            "semantic_filtered": {"top3_useful": 0},
            "ai_baseline": {"top3_useful": 1},
        }
    )
    top3 = {
        "fact_only": [_r("src/a.rs", "error_propagation")],
        "semantic_filtered": [_r("src/b.rs", "state_transition")],
        "ai_baseline": [_r("src/a.rs", "error_propagation")],
    }
    out = MOD.recompute(judge_output=judge, top_risks_per_mode=top3)
    assert out["per_mode"]["fact_only"]["agree"] is True
    assert out["per_mode"]["semantic_filtered"]["agree"] is True
    assert out["per_mode"]["ai_baseline"]["agree"] is True
    assert out["flags"] == []


def test_recompute_disagree_emits_flag() -> None:
    # Judge claims ai_baseline got 2 useful, mechanical says only 1.
    judge = _judge(
        drafts={
            "fact_only": {"top3_useful": 0},
            "semantic_filtered": {"top3_useful": 0},
            "ai_baseline": {"top3_useful": 2},
        }
    )
    top3 = {
        "fact_only": [],
        "semantic_filtered": [],
        "ai_baseline": [
            _r("src/a.rs", "error_propagation"),
            _r("src/c.rs", "state_transition"),
        ],
    }
    out = MOD.recompute(judge_output=judge, top_risks_per_mode=top3)
    assert out["per_mode"]["ai_baseline"]["agree"] is False
    assert out["per_mode"]["ai_baseline"]["ai_subjective"] == 2
    assert out["per_mode"]["ai_baseline"]["mechanical"] == 1
    assert len(out["flags"]) == 1
    assert "ai_baseline" in out["flags"][0]


def test_recompute_skips_modes_with_no_top3_data() -> None:
    judge = _judge()
    top3 = {"fact_only": None, "semantic_filtered": None, "ai_baseline": None}
    out = MOD.recompute(judge_output=judge, top_risks_per_mode=top3)
    for mode in ("fact_only", "semantic_filtered", "ai_baseline"):
        assert out["per_mode"][mode]["mechanical"] is None
        assert out["per_mode"][mode]["agree"] is None
    assert out["flags"] == []
