#!/usr/bin/env python3
"""Validate a judge manifest against the on-disk prompt / schema.

Same shape as `validate-head-to-head-manifest.py` but uses the judge prompt /
schema fields. Kept as a separate file so the AI baseline runner and the
judge runner do not share a manifest by accident.

Exit codes:
  0  manifest matches on-disk judge prompt / schema
  2  manifest file missing or shape is wrong
  3  prompt or schema file missing
  4  prompt or schema sha256 does not match manifest
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class ValidationResult:
    ok: bool
    code: int
    message: str


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _require(manifest: dict, key_path: list[str]) -> object:
    node = manifest
    for key in key_path:
        if not isinstance(node, dict) or key not in node:
            raise KeyError(".".join(key_path))
        node = node[key]
    return node


def validate_manifest(manifest_path: Path, repo_root: Path = REPO_ROOT) -> ValidationResult:
    if not manifest_path.exists():
        return ValidationResult(False, 2, f"manifest not found: {manifest_path}")
    try:
        with manifest_path.open("rb") as handle:
            manifest = tomllib.load(handle)
    except tomllib.TOMLDecodeError as err:
        return ValidationResult(False, 2, f"manifest is not valid TOML: {err}")

    try:
        prompt_rel = str(_require(manifest, ["freeze", "prompt_path"]))
        prompt_sha = str(_require(manifest, ["freeze", "prompt_sha256"]))
        schema_rel = str(_require(manifest, ["freeze", "schema_path"]))
        schema_sha = str(_require(manifest, ["freeze", "schema_sha256"]))
    except KeyError as missing:
        return ValidationResult(False, 2, f"manifest missing required field: {missing}")

    if not isinstance(prompt_sha, str) or len(prompt_sha) != 64:
        return ValidationResult(False, 2, "manifest freeze.prompt_sha256 is not a 64-char hex string")
    if not isinstance(schema_sha, str) or len(schema_sha) != 64:
        return ValidationResult(False, 2, "manifest freeze.schema_sha256 is not a 64-char hex string")

    prompt_path = (repo_root / prompt_rel).resolve()
    schema_path = (repo_root / schema_rel).resolve()
    if not prompt_path.exists():
        return ValidationResult(False, 3, f"prompt file not found: {prompt_path}")
    if not schema_path.exists():
        return ValidationResult(False, 3, f"schema file not found: {schema_path}")

    actual_prompt_sha = file_sha256(prompt_path)
    if actual_prompt_sha != prompt_sha:
        return ValidationResult(
            False,
            4,
            f"prompt sha mismatch\n  manifest: {prompt_sha}\n  on-disk:  {actual_prompt_sha}",
        )

    actual_schema_sha = file_sha256(schema_path)
    if actual_schema_sha != schema_sha:
        return ValidationResult(
            False,
            4,
            f"schema sha mismatch\n  manifest: {schema_sha}\n  on-disk:  {actual_schema_sha}",
        )

    return ValidationResult(True, 0, "manifest matches on-disk prompt and schema")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    result = validate_manifest(args.manifest.resolve(), args.repo_root.resolve())
    print(result.message)
    return result.code


if __name__ == "__main__":
    raise SystemExit(main())
