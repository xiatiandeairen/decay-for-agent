"""Unit tests for the judge manifest hash gate."""

from __future__ import annotations

import hashlib
import importlib.util
import sys
from pathlib import Path

import pytest


SCRIPTS_DIR = Path(__file__).resolve().parent


def _load_module():
    spec = importlib.util.spec_from_file_location(
        "validate_judge_manifest",
        SCRIPTS_DIR / "validate-judge-manifest.py",
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


MOD = _load_module()


def _write_manifest(
    root: Path,
    prompt_sha: str,
    schema_sha: str,
    *,
    prompt_path: str = "prompts/judge.md",
    schema_path: str = "schemas/judge.json",
    omit_field: str | None = None,
) -> Path:
    lines = [
        'version = "head-to-head-judge-manifest.v1"',
        "[model]",
        'name = "claude-opus-4-7"',
        "[freeze]",
    ]
    fields = {
        "prompt_path": f'"{prompt_path}"',
        "prompt_sha256": f'"{prompt_sha}"',
        "schema_path": f'"{schema_path}"',
        "schema_sha256": f'"{schema_sha}"',
    }
    for key, val in fields.items():
        if key == omit_field:
            continue
        lines.append(f"{key} = {val}")
    manifest = root / "judge-manifest.toml"
    manifest.write_text("\n".join(lines) + "\n")
    return manifest


def _seed(root: Path, prompt_body: bytes, schema_body: bytes) -> tuple[Path, Path, str, str]:
    (root / "prompts").mkdir(parents=True)
    (root / "schemas").mkdir(parents=True)
    prompt = root / "prompts" / "judge.md"
    schema = root / "schemas" / "judge.json"
    prompt.write_bytes(prompt_body)
    schema.write_bytes(schema_body)
    return (
        prompt,
        schema,
        hashlib.sha256(prompt_body).hexdigest(),
        hashlib.sha256(schema_body).hexdigest(),
    )


def test_matching_hashes_pass(tmp_path: Path) -> None:
    _, _, p_sha, s_sha = _seed(tmp_path, b"judge prompt", b'{"x":1}')
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    assert MOD.validate_manifest(manifest, tmp_path).code == 0


def test_prompt_changed_one_char_is_rejected(tmp_path: Path) -> None:
    prompt, _, p_sha, s_sha = _seed(tmp_path, b"judge prompt", b'{"x":1}')
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    prompt.write_bytes(b"judge promptX")
    r = MOD.validate_manifest(manifest, tmp_path)
    assert r.code == 4 and "prompt sha mismatch" in r.message


def test_schema_changed_one_char_is_rejected(tmp_path: Path) -> None:
    _, schema, p_sha, s_sha = _seed(tmp_path, b"judge prompt", b'{"x":1}')
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    schema.write_bytes(b'{"x":2}')
    r = MOD.validate_manifest(manifest, tmp_path)
    assert r.code == 4 and "schema sha mismatch" in r.message


def test_missing_manifest_field_is_rejected(tmp_path: Path) -> None:
    _, _, p_sha, s_sha = _seed(tmp_path, b"judge prompt", b'{"x":1}')
    manifest = _write_manifest(tmp_path, p_sha, s_sha, omit_field="schema_sha256")
    r = MOD.validate_manifest(manifest, tmp_path)
    assert r.code == 2 and "missing required field" in r.message


def test_non_hex_sha_is_rejected(tmp_path: Path) -> None:
    _, _, _, s_sha = _seed(tmp_path, b"judge prompt", b'{"x":1}')
    manifest = _write_manifest(tmp_path, "abc", s_sha)
    r = MOD.validate_manifest(manifest, tmp_path)
    assert r.code == 2 and "64-char hex" in r.message


def test_missing_prompt_file_is_rejected(tmp_path: Path) -> None:
    prompt, _, p_sha, s_sha = _seed(tmp_path, b"judge prompt", b'{"x":1}')
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    prompt.unlink()
    r = MOD.validate_manifest(manifest, tmp_path)
    assert r.code == 3 and "prompt file not found" in r.message


def test_missing_manifest_file_is_rejected(tmp_path: Path) -> None:
    r = MOD.validate_manifest(tmp_path / "absent.toml", tmp_path)
    assert r.code == 2 and "manifest not found" in r.message


def test_actual_repo_judge_manifest_passes() -> None:
    # Sanity check on the committed manifest. If the judge prompt or schema
    # file gets edited without updating the manifest, this fails first.
    repo_root = Path(__file__).resolve().parents[2]
    manifest = repo_root / "eval" / "head-to-head" / "manifests" / "judge-v1.toml"
    if not manifest.exists():
        pytest.skip("judge-v1.toml not yet committed")
    r = MOD.validate_manifest(manifest, repo_root)
    assert r.ok, r.message
