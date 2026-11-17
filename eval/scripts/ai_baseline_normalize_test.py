"""Unit tests for the AI baseline output normalizer."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


SCRIPTS_DIR = Path(__file__).resolve().parent


def _load_module():
    spec = importlib.util.spec_from_file_location(
        "normalize_ai_output",
        SCRIPTS_DIR / "normalize-ai-output.py",
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


MOD = _load_module()


# A diff snippet whose lines we can quote verbatim in evidence_refs.
DIFF_PATCH = """\
diff --git a/src/conn.rs b/src/conn.rs
@@ -42,7 +42,11 @@ impl Conn {
     pub fn close(&mut self) -> Result<(), Error> {
-        self.inner.shutdown()
+        if self.upgraded {
+            return Err(Error::shutdown_after_upgrade());
+        }
+        self.inner.shutdown()?;
+        Ok(())
     }
"""

CHANGED_FILES = ["src/conn.rs"]


def _good_payload(**overrides) -> dict:
    payload = {
        "case_id": "hyperium-hyper-3616",
        "model": "claude-opus-4-7",
        "manifest_hash": "a" * 64,
        "top_risks": [
            {
                "rank": 1,
                "risk_signal": "lifecycle_ordering",
                "subject": "src/conn.rs:close",
                "evidence_refs": ["return Err(Error::shutdown_after_upgrade());"],
                "confidence": "high",
                "value": "direct",
                "rationale": "Diff adds an early-return guarding shutdown after upgrade.",
            }
        ],
    }
    payload.update(overrides)
    return payload


def _make_input(payload: dict | str, *, diff: str = DIFF_PATCH, changed: list[str] | None = None):
    raw = payload if isinstance(payload, str) else json.dumps(payload)
    return MOD.NormalizeInput(
        raw=raw,
        diff_patch=diff,
        changed_files=changed if changed is not None else CHANGED_FILES,
    )


# ---- positive cases ----

def test_well_formed_passes() -> None:
    out = MOD.normalize(_make_input(_good_payload()))
    assert out["case_id"] == "hyperium-hyper-3616"
    assert len(out["top_risks"]) == 1


def test_empty_top_risks_passes() -> None:
    out = MOD.normalize(_make_input(_good_payload(top_risks=[])))
    assert out["top_risks"] == []


def test_strips_markdown_fence() -> None:
    payload = _good_payload()
    fenced = "```json\n" + json.dumps(payload) + "\n```"
    out = MOD.normalize(_make_input(fenced))
    assert out["case_id"] == payload["case_id"]


def test_repairs_truncated_root_brace() -> None:
    # Observed claude-opus-4-7 sampling failure: closes the array, drops the
    # outer `}`. The normalizer is allowed exactly one repair shot.
    payload = _good_payload()
    truncated = json.dumps(payload).rstrip("}") + "\n"
    out = MOD.normalize(_make_input(truncated))
    assert out["case_id"] == payload["case_id"]
    assert len(out["top_risks"]) == 1


def test_unrepairable_json_still_fails() -> None:
    with pytest.raises(MOD.NormalizeError, match="not valid JSON"):
        MOD.normalize(_make_input("{nope: this is broken in ways we won't try to fix"))


def test_evidence_ref_matches_after_stripping_unified_diff_prefix() -> None:
    # Diff text on disk has `+`/`-` line prefixes; models commonly drop them
    # when quoting the new code. Normalizer accepts both forms.
    risk = _good_payload()["top_risks"][0]
    risk["evidence_refs"] = ["return Err(Error::shutdown_after_upgrade());"]
    out = MOD.normalize(_make_input(_good_payload(top_risks=[risk])))
    assert len(out["top_risks"]) == 1


def test_evidence_ref_not_in_diff_even_stripped_is_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["evidence_refs"] = ["this string is not in the diff at all"]
    with pytest.raises(MOD.NormalizeError, match="not found verbatim"):
        MOD.normalize(_make_input(_good_payload(top_risks=[risk])))


def test_rationale_within_truncation_window_is_truncated_with_ellipsis() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["rationale"] = "a" * 230
    out = MOD.normalize(_make_input(_good_payload(top_risks=[risk])))
    rationale = out["top_risks"][0]["rationale"]
    assert len(rationale) == MOD.MAX_RATIONALE_CHARS
    assert rationale.endswith("…")


def test_rationale_beyond_truncation_window_still_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["rationale"] = "b" * 260
    with pytest.raises(MOD.NormalizeError, match="exceeds max 200"):
        MOD.normalize(_make_input(_good_payload(top_risks=[risk])))


def test_evidence_ref_with_mixed_diff_prefixes_is_accepted() -> None:
    # Model preserves `-` / `+` prefixes on some lines of the quoted hunk.
    risk = _good_payload()["top_risks"][0]
    risk["evidence_refs"] = [
        "-        self.inner.shutdown()\n+        if self.upgraded {"
    ]
    out = MOD.normalize(_make_input(_good_payload(top_risks=[risk])))
    assert len(out["top_risks"]) == 1


def test_evidence_ref_with_indentation_drift_is_accepted() -> None:
    # Off-by-one indentation: diff has `        return Err(...)` (8 spaces),
    # model writes the same line with 6 spaces. As long as the content sequence
    # matches, accept it.
    risk = _good_payload()["top_risks"][0]
    risk["evidence_refs"] = ["      return Err(Error::shutdown_after_upgrade());"]
    out = MOD.normalize(_make_input(_good_payload(top_risks=[risk])))
    assert len(out["top_risks"]) == 1


def test_evidence_ref_with_fabricated_content_still_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["evidence_refs"] = ["return some_function_that_is_not_in_the_diff();"]
    with pytest.raises(MOD.NormalizeError, match="not found verbatim"):
        MOD.normalize(_make_input(_good_payload(top_risks=[risk])))


# ---- negative cases (the 5 plan-listed shapes) ----

def test_invalid_json_is_rejected() -> None:
    with pytest.raises(MOD.NormalizeError, match="not valid JSON"):
        MOD.normalize(_make_input("not json at all"))


def test_missing_required_field_is_rejected() -> None:
    bad = _good_payload()
    del bad["case_id"]
    with pytest.raises(MOD.NormalizeError, match="missing required field 'case_id'"):
        MOD.normalize(_make_input(bad))


def test_more_than_three_top_risks_is_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    bad = _good_payload(top_risks=[risk, risk, risk, risk])
    with pytest.raises(MOD.NormalizeError, match="max is 3"):
        MOD.normalize(_make_input(bad))


def test_evidence_ref_not_in_diff_is_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["evidence_refs"] = ["this string does not appear in the diff"]
    bad = _good_payload(top_risks=[risk])
    with pytest.raises(MOD.NormalizeError, match="not found verbatim in diff.patch"):
        MOD.normalize(_make_input(bad))


def test_unknown_risk_signal_is_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["risk_signal"] = "magical_invented_signal"
    bad = _good_payload(top_risks=[risk])
    with pytest.raises(MOD.NormalizeError, match="not in allowed enum"):
        MOD.normalize(_make_input(bad))


# ---- additional invariants ----

def test_direct_with_low_confidence_is_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["confidence"] = "low"
    risk["value"] = "direct"
    bad = _good_payload(top_risks=[risk])
    with pytest.raises(MOD.NormalizeError, match="value=direct requires confidence"):
        MOD.normalize(_make_input(bad))


def test_subject_referring_unknown_file_is_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["subject"] = "src/unrelated.rs:fn"
    bad = _good_payload(top_risks=[risk])
    with pytest.raises(MOD.NormalizeError, match="references unknown file"):
        MOD.normalize(_make_input(bad))


def test_manifest_hash_not_hex_is_rejected() -> None:
    bad = _good_payload(manifest_hash="not-a-hex")
    with pytest.raises(MOD.NormalizeError, match="64-char hex"):
        MOD.normalize(_make_input(bad))


def test_rationale_too_long_is_rejected() -> None:
    risk = _good_payload()["top_risks"][0]
    risk["rationale"] = "x" * 250
    bad = _good_payload(top_risks=[risk])
    with pytest.raises(MOD.NormalizeError, match="exceeds max 200"):
        MOD.normalize(_make_input(bad))


def test_unknown_top_level_field_is_rejected() -> None:
    bad = _good_payload()
    bad["surprise"] = "hi"
    with pytest.raises(MOD.NormalizeError, match="unexpected fields"):
        MOD.normalize(_make_input(bad))
