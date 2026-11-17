"""Schema-file structural tests for the judge v1 contract.

These tests focus on the declarative shape of the judge schema file itself.
Behavioral validation is covered in judge_normalize_test.py.
"""

from __future__ import annotations

import json
import tomllib
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = REPO_ROOT / "eval" / "head-to-head" / "schemas" / "judge-output.schema.json"
PROMPT_PATH = REPO_ROOT / "eval" / "head-to-head" / "prompts" / "judge-v1.md"
MANIFEST_PATH = REPO_ROOT / "eval" / "head-to-head" / "manifests" / "judge-v1.toml"


def test_schema_file_is_valid_json() -> None:
    data = json.loads(SCHEMA_PATH.read_text())
    assert isinstance(data, dict)
    assert data.get("title") == "Judge output v1"


def test_schema_has_two_layer_structure() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    top_required = set(schema["required"])
    assert top_required == {"case_id", "model", "manifest_hash", "ground_truth_draft", "judgement_drafts"}


def test_schema_ground_truth_draft_required_fields() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    gt = schema["properties"]["ground_truth_draft"]
    assert set(gt["required"]) == {
        "high_value",
        "primary_signal_ids",
        "subject_files",
        "expected_behavior",
        "rationale_from_pr_metadata",
    }
    psi_enum = gt["properties"]["primary_signal_ids"]["items"]["enum"]
    assert set(psi_enum) == {
        "assumption_boundary",
        "non_happy_path",
        "state_transition",
        "error_propagation",
        "lifecycle_ordering",
        "concurrency_ordering",
        "external_contract",
        "test_gap_for_risk_path",
    }


def test_schema_judgement_drafts_three_modes() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    drafts = schema["properties"]["judgement_drafts"]
    assert set(drafts["required"]) == {"fact_only", "semantic_filtered", "ai_baseline"}
    for mode in ("fact_only", "semantic_filtered", "ai_baseline"):
        assert "$ref" in drafts["properties"][mode]


def test_schema_mode_judgement_required_fields() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    mode_def = schema["definitions"]["mode_judgement"]
    assert set(mode_def["required"]) == {
        "top3_total",
        "top3_useful",
        "actionable",
        "missed_high_risk_count",
        "weak_only_top_risk_count",
        "notes",
    }


def test_schema_manifest_hash_is_64_hex() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    assert schema["properties"]["manifest_hash"]["pattern"] == "^[0-9a-f]{64}$"


def test_prompt_describes_both_layers() -> None:
    text = PROMPT_PATH.read_text()
    assert "ground_truth_draft" in text
    assert "judgement_drafts" in text
    for signal in [
        "assumption_boundary",
        "non_happy_path",
        "state_transition",
        "error_propagation",
        "lifecycle_ordering",
        "concurrency_ordering",
        "external_contract",
        "test_gap_for_risk_path",
    ]:
        assert signal in text


def test_prompt_explains_judge_asymmetry() -> None:
    text = PROMPT_PATH.read_text().lower()
    # Judge is allowed to see PR metadata — the asymmetry must be stated so
    # the model knows what it has access to.
    assert "pr metadata" in text
    assert "asymmetry" in text or "deliberately allowed" in text


def test_manifest_fairness_allows_pr_metadata() -> None:
    manifest = tomllib.loads(MANIFEST_PATH.read_text())
    allowed = set(manifest["fairness"]["allowed_inputs"])
    # Unlike the ai-baseline fairness contract, the judge MUST be able to read pr.json.
    assert "pr.json" in allowed
    assert "diff.patch" in allowed
    # And the judge consumes all three pipelines' outputs.
    assert "decay.stdout" in allowed
    assert "semantic-findings.json" in allowed
    assert "ai-baseline.normalized.json" in allowed
