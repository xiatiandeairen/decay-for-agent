"""Unit tests for the judge output normalizer."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


SCRIPTS_DIR = Path(__file__).resolve().parent


def _load():
    spec = importlib.util.spec_from_file_location(
        "normalize_judge_output",
        SCRIPTS_DIR / "normalize-judge-output.py",
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


MOD = _load()


def _mode(top3_total=1, top3_useful=1, actionable=True, missed=0, weak=0, notes="") -> dict:
    return {
        "top3_total": top3_total,
        "top3_useful": top3_useful,
        "actionable": actionable,
        "missed_high_risk_count": missed,
        "weak_only_top_risk_count": weak,
        "notes": notes,
    }


def _good(**overrides) -> dict:
    payload = {
        "case_id": "tokio-rs-tokio-7991",
        "model": "claude-opus-4-7",
        "manifest_hash": "a" * 64,
        "ground_truth_draft": {
            "high_value": True,
            "primary_signal_ids": ["error_propagation"],
            "subject_files": ["tokio/src/sync/mpsc/chan.rs"],
            "expected_behavior": "Fix panic on close with non-empty vector.",
            "rationale_from_pr_metadata": "Title says 'fix panic in Chan::recv_many'.",
        },
        "judgement_drafts": {
            "fact_only": _mode(),
            "semantic_filtered": _mode(),
            "ai_baseline": _mode(),
        },
    }
    payload.update(overrides)
    return payload


def _make(payload, *, as_raw: str | None = None):
    raw = as_raw if as_raw is not None else (payload if isinstance(payload, str) else json.dumps(payload))
    return MOD.NormalizeInput(raw=raw)


# ---- positive cases ----

def test_well_formed_passes() -> None:
    out = MOD.normalize(_make(_good()))
    assert out["case_id"] == "tokio-rs-tokio-7991"
    assert set(out["judgement_drafts"].keys()) == {"fact_only", "semantic_filtered", "ai_baseline"}


def test_strips_markdown_fence() -> None:
    payload = _good()
    fenced = "```json\n" + json.dumps(payload) + "\n```"
    out = MOD.normalize(_make(payload, as_raw=fenced))
    assert out["case_id"] == payload["case_id"]


def test_repairs_truncated_root_missing_one_brace() -> None:
    payload = _good()
    truncated = json.dumps(payload)[:-1] + "\n"  # drop the single outermost `}`
    out = MOD.normalize(_make(payload, as_raw=truncated))
    assert out["case_id"] == payload["case_id"]


def test_repairs_truncated_root_missing_two_braces() -> None:
    payload = _good()
    truncated = json.dumps(payload)[:-2] + "\n"  # drop outermost `}}`
    out = MOD.normalize(_make(payload, as_raw=truncated))
    assert out["case_id"] == payload["case_id"]


def test_unrepairable_truncation_still_fails() -> None:
    # Drop far more than 3 braces — the repair gives up at 3.
    payload = _good()
    truncated = json.dumps(payload)[:-30] + "\n"
    with pytest.raises(MOD.NormalizeError):
        MOD.normalize(_make(payload, as_raw=truncated))


# ---- negative cases (5+ as required) ----

def test_invalid_json_is_rejected() -> None:
    with pytest.raises(MOD.NormalizeError, match="not valid JSON"):
        MOD.normalize(_make("not json"))


def test_missing_ground_truth_draft_is_rejected() -> None:
    bad = _good()
    del bad["ground_truth_draft"]
    with pytest.raises(MOD.NormalizeError, match="ground_truth_draft"):
        MOD.normalize(_make(bad))


def test_missing_mode_in_judgement_drafts_is_rejected() -> None:
    bad = _good()
    del bad["judgement_drafts"]["semantic_filtered"]
    with pytest.raises(MOD.NormalizeError, match="missing modes"):
        MOD.normalize(_make(bad))


def test_unknown_risk_signal_in_ground_truth_is_rejected() -> None:
    bad = _good()
    bad["ground_truth_draft"]["primary_signal_ids"] = ["totally_made_up"]
    with pytest.raises(MOD.NormalizeError, match="not in allowed enum"):
        MOD.normalize(_make(bad))


def test_top3_useful_exceeds_top3_total_is_rejected() -> None:
    bad = _good()
    bad["judgement_drafts"]["fact_only"] = _mode(top3_total=1, top3_useful=2)
    with pytest.raises(MOD.NormalizeError, match="cannot exceed"):
        MOD.normalize(_make(bad))


def test_high_value_true_with_empty_primary_signal_is_rejected() -> None:
    bad = _good()
    bad["ground_truth_draft"]["high_value"] = True
    bad["ground_truth_draft"]["primary_signal_ids"] = []
    with pytest.raises(MOD.NormalizeError, match="non-empty primary_signal_ids"):
        MOD.normalize(_make(bad))


# ---- additional invariants ----

def test_high_value_false_allows_empty_primary_signal() -> None:
    payload = _good()
    payload["ground_truth_draft"]["high_value"] = False
    payload["ground_truth_draft"]["primary_signal_ids"] = []
    out = MOD.normalize(_make(payload))
    assert out["ground_truth_draft"]["high_value"] is False


def test_manifest_hash_not_hex_is_rejected() -> None:
    with pytest.raises(MOD.NormalizeError, match="64-char hex"):
        MOD.normalize(_make(_good(manifest_hash="nope")))


def test_unexpected_mode_in_judgement_drafts_is_rejected() -> None:
    bad = _good()
    bad["judgement_drafts"]["extra_mode"] = _mode()
    with pytest.raises(MOD.NormalizeError, match="unexpected modes"):
        MOD.normalize(_make(bad))


def test_unexpected_top_level_field_is_rejected() -> None:
    bad = _good()
    bad["surprise"] = "hi"
    with pytest.raises(MOD.NormalizeError, match="unexpected fields"):
        MOD.normalize(_make(bad))


def test_weak_only_must_be_zero_or_one() -> None:
    bad = _good()
    bad["judgement_drafts"]["fact_only"]["weak_only_top_risk_count"] = 2
    with pytest.raises(MOD.NormalizeError, match="must be 0 or 1"):
        MOD.normalize(_make(bad))


def test_expected_behavior_too_long_is_rejected() -> None:
    bad = _good()
    bad["ground_truth_draft"]["expected_behavior"] = "x" * 500
    with pytest.raises(MOD.NormalizeError, match="exceeds max 400"):
        MOD.normalize(_make(bad))
