"""Unit tests for the head-to-head manifest hash gate."""

from __future__ import annotations

import hashlib
import importlib.util
import sys
from pathlib import Path

import pytest


SCRIPTS_DIR = Path(__file__).resolve().parent


def _load_module():
    spec = importlib.util.spec_from_file_location(
        "validate_head_to_head_manifest",
        SCRIPTS_DIR / "validate-head-to-head-manifest.py",
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
    prompt_path: str = "prompts/p.md",
    schema_path: str = "schemas/s.json",
    omit_field: str | None = None,
) -> Path:
    lines = [
        'version = "head-to-head-manifest.v1"',
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
    manifest = root / "manifest.toml"
    manifest.write_text("\n".join(lines) + "\n")
    return manifest


def _seed_files(root: Path, prompt_body: bytes, schema_body: bytes) -> tuple[Path, Path, str, str]:
    (root / "prompts").mkdir(parents=True)
    (root / "schemas").mkdir(parents=True)
    prompt = root / "prompts" / "p.md"
    schema = root / "schemas" / "s.json"
    prompt.write_bytes(prompt_body)
    schema.write_bytes(schema_body)
    return (
        prompt,
        schema,
        hashlib.sha256(prompt_body).hexdigest(),
        hashlib.sha256(schema_body).hexdigest(),
    )


def test_matching_hashes_pass(tmp_path: Path) -> None:
    _, _, p_sha, s_sha = _seed_files(tmp_path, b"hello", b"{}\n")
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    result = MOD.validate_manifest(manifest, tmp_path)
    assert result.ok is True
    assert result.code == 0


def test_prompt_changed_one_char_is_rejected(tmp_path: Path) -> None:
    prompt, _, p_sha, s_sha = _seed_files(tmp_path, b"hello", b"{}\n")
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    prompt.write_bytes(b"hellX")  # one-byte difference
    result = MOD.validate_manifest(manifest, tmp_path)
    assert result.ok is False
    assert result.code == 4
    assert "prompt sha mismatch" in result.message


def test_schema_changed_one_char_is_rejected(tmp_path: Path) -> None:
    _, schema, p_sha, s_sha = _seed_files(tmp_path, b"hello", b"{}\n")
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    schema.write_bytes(b"{ }\n")
    result = MOD.validate_manifest(manifest, tmp_path)
    assert result.ok is False
    assert result.code == 4
    assert "schema sha mismatch" in result.message


def test_missing_manifest_field_is_rejected(tmp_path: Path) -> None:
    _, _, p_sha, s_sha = _seed_files(tmp_path, b"hello", b"{}\n")
    manifest = _write_manifest(tmp_path, p_sha, s_sha, omit_field="schema_sha256")
    result = MOD.validate_manifest(manifest, tmp_path)
    assert result.ok is False
    assert result.code == 2
    assert "missing required field" in result.message


def test_non_hex_sha_field_is_rejected(tmp_path: Path) -> None:
    _, _, _, s_sha = _seed_files(tmp_path, b"hello", b"{}\n")
    manifest = _write_manifest(tmp_path, "not-a-sha", s_sha)
    result = MOD.validate_manifest(manifest, tmp_path)
    assert result.ok is False
    assert result.code == 2
    assert "64-char hex" in result.message


def test_missing_prompt_file_is_rejected(tmp_path: Path) -> None:
    prompt, _, p_sha, s_sha = _seed_files(tmp_path, b"hello", b"{}\n")
    manifest = _write_manifest(tmp_path, p_sha, s_sha)
    prompt.unlink()
    result = MOD.validate_manifest(manifest, tmp_path)
    assert result.ok is False
    assert result.code == 3
    assert "prompt file not found" in result.message


def test_missing_manifest_file_is_rejected(tmp_path: Path) -> None:
    result = MOD.validate_manifest(tmp_path / "absent.toml", tmp_path)
    assert result.ok is False
    assert result.code == 2
    assert "manifest not found" in result.message
