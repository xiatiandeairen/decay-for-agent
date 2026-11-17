"""Schema-file and runner-input fairness tests for the head-to-head v1 contract.

This test pack focuses on three orthogonal invariants:

1. The on-disk JSON Schema file itself is well-formed JSON and declares the
   structural rules the normalizer enforces in code. We do not require a
   `jsonschema` validator at runtime; instead we assert the schema literal
   contains the right enum / maxItems / required entries, so a future edit
   that drops a rule is loudly visible.

2. The 8 risk_signal enum names in the schema match the enum in the prompt
   file (single source of truth alignment).

3. The runner-side fairness contract: when the runner constructs the per-case
   input that is handed to claude CLI, it must include only the allowed
   inputs (`diff.patch`, `changed_files.txt`) and must refuse every denied
   input (PR title, body, discussion, linked issue, CI logs, etc.).

The runner's input-construction helper is exercised in isolation here; the
real claude CLI is mocked out in the runner tests (skipped in this file —
this file targets the contract surface).
"""

from __future__ import annotations

import importlib.util
import json
import re
import sys
import tomllib
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = Path(__file__).resolve().parent
SCHEMA_PATH = REPO_ROOT / "eval" / "head-to-head" / "schemas" / "ai-baseline-output.schema.json"
PROMPT_PATH = REPO_ROOT / "eval" / "head-to-head" / "prompts" / "ai-baseline-v1.md"
MANIFEST_PATH = REPO_ROOT / "eval" / "head-to-head" / "manifests" / "v1.toml"


def _load_module(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS_DIR / filename)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


# ---- schema-file structural checks ----

def test_schema_file_is_valid_json() -> None:
    data = json.loads(SCHEMA_PATH.read_text())
    assert isinstance(data, dict)
    assert data.get("title") == "AI baseline output v1"


def test_schema_top_risks_has_max_items_three() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    assert schema["properties"]["top_risks"]["maxItems"] == 3


def test_schema_risk_signal_enum_is_canonical_eight() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    risk_signal_enum = schema["properties"]["top_risks"]["items"]["properties"]["risk_signal"]["enum"]
    assert set(risk_signal_enum) == {
        "assumption_boundary",
        "non_happy_path",
        "state_transition",
        "error_propagation",
        "lifecycle_ordering",
        "concurrency_ordering",
        "external_contract",
        "test_gap_for_risk_path",
    }


def test_schema_required_fields_declared() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    assert set(schema["required"]) == {"case_id", "model", "manifest_hash", "top_risks"}
    item_required = set(schema["properties"]["top_risks"]["items"]["required"])
    assert item_required == {
        "rank",
        "risk_signal",
        "subject",
        "evidence_refs",
        "confidence",
        "value",
        "rationale",
    }


def test_schema_confidence_and_value_enums_match_normalizer() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    item_props = schema["properties"]["top_risks"]["items"]["properties"]
    assert set(item_props["confidence"]["enum"]) == {"high", "medium", "low"}
    assert set(item_props["value"]["enum"]) == {"direct", "supporting", "context"}


def test_schema_manifest_hash_is_64_hex() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    pattern = schema["properties"]["manifest_hash"]["pattern"]
    assert pattern == "^[0-9a-f]{64}$"


# ---- prompt / schema enum alignment ----

def test_prompt_lists_all_eight_risk_signals() -> None:
    text = PROMPT_PATH.read_text()
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
        assert signal in text, f"prompt missing risk_signal: {signal}"


def test_prompt_forbids_pr_metadata() -> None:
    text = PROMPT_PATH.read_text()
    # The prompt must explicitly tell the model not to assume PR-level metadata.
    for forbidden in ["title", "body", "discussion", "linked issue", "CI log"]:
        assert forbidden.lower() in text.lower(), (
            f"prompt does not name '{forbidden}' in its fairness constraints; "
            f"the model may silently rely on it."
        )


# ---- fairness contract from manifest ----

def test_manifest_lists_allowed_and_denied_inputs() -> None:
    with MANIFEST_PATH.open("rb") as handle:
        manifest = tomllib.load(handle)
    fairness = manifest["fairness"]
    assert set(fairness["allowed_inputs"]) == {"diff.patch", "changed_files.txt"}
    denied = set(fairness["denied_inputs"])
    must_be_denied = {
        "pr.title",
        "pr.body",
        "pr.discussion",
        "linked_issue",
        "ci_logs",
        "repository_source_outside_diff",
    }
    missing = must_be_denied - denied
    assert not missing, f"manifest fairness.denied_inputs is missing: {sorted(missing)}"


# ---- runner-side input-construction fairness ----

def test_runner_input_helper_only_includes_allowed_files(tmp_path: Path) -> None:
    runner = _load_module("run_ai_baseline", "run-ai-baseline.py")

    case_dir = tmp_path / "fake-case"
    case_dir.mkdir()
    (case_dir / "diff.patch").write_text("diff body")
    (case_dir / "files.txt").write_text("src/a.rs\n")
    # Deliberately put forbidden inputs into the same case directory.
    (case_dir / "pr.json").write_text('{"title": "should not leak"}')
    (case_dir / "pr.body.md").write_text("forbidden PR body")
    (case_dir / "ci.log").write_text("forbidden CI log")
    (case_dir / "linked-issue.json").write_text('{"number": 42}')

    bundle = runner.build_case_input(case_dir)
    # The helper must return a dict mapping the allowed filenames only.
    assert set(bundle.keys()) == {"diff.patch", "changed_files.txt"}
    # And the values must be the file contents, not paths to other files.
    assert bundle["diff.patch"] == "diff body"
    assert "should not leak" not in json.dumps(bundle)
    assert "forbidden PR body" not in json.dumps(bundle)


def test_runner_strips_git_status_prefix(tmp_path: Path) -> None:
    runner = _load_module("run_ai_baseline_strip", "run-ai-baseline.py")
    case_dir = tmp_path / "fake-case-3"
    case_dir.mkdir()
    (case_dir / "diff.patch").write_text("any diff")
    (case_dir / "files.txt").write_text(
        "M\tsrc/foo.rs\n"
        "A\tsrc/new.rs\n"
        "R100\tsrc/old.rs\tsrc/renamed.rs\n"
    )
    bundle = runner.build_case_input(case_dir)
    paths = [p for p in bundle["changed_files.txt"].splitlines() if p]
    assert paths == ["src/foo.rs", "src/new.rs", "src/renamed.rs"]
    # Confirm the LLM never sees `M\t` or `A\t` literal prefixes.
    assert "\t" not in bundle["changed_files.txt"]


def test_runner_rejects_case_dir_missing_diff(tmp_path: Path) -> None:
    runner = _load_module("run_ai_baseline_missing", "run-ai-baseline.py")
    case_dir = tmp_path / "fake-case-2"
    case_dir.mkdir()
    (case_dir / "files.txt").write_text("src/a.rs\n")
    with pytest.raises(FileNotFoundError):
        runner.build_case_input(case_dir)
