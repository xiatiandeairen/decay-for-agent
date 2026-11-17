#!/usr/bin/env python3
"""Run the AI baseline against a single frozen-v1 case (or all cases).

Pipeline per case:
  1. Verify the manifest hash gate (refuses if prompt/schema were touched).
  2. Build the per-case input bundle — only `diff.patch` and
     `changed_files.txt`, never PR title/body/discussion/issue/CI logs.
  3. Invoke claude CLI with the frozen model name and the prompt file.
  4. Pipe the response through the normalizer to enforce schema rules and
     verify every `evidence_refs` excerpt exists verbatim in the diff.
  5. Persist `raw.json` and `normalized.json` under
     `runs/<run-id>/<case-id>/`.

On transient failure the runner backs off and retries up to N times per the
manifest. Persistent failure produces `normalized.json = {"status":
"ai-baseline-failed", ...}` so summarize-head-to-head can exclude it from
metric denominators.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable


REPO_ROOT = Path(__file__).resolve().parents[2]
EVAL_ROOT = REPO_ROOT / "eval"
HEAD_TO_HEAD_DIR = EVAL_ROOT / "head-to-head"
DEFAULT_MANIFEST = HEAD_TO_HEAD_DIR / "manifests" / "v1.toml"
DEFAULT_RUNS_DIR = HEAD_TO_HEAD_DIR / "runs"
DEFAULT_CASES_DIR = EVAL_ROOT / "cases"
DEFAULT_CORPUS = EVAL_ROOT / "corpus" / "risk-signal-frozen-v1.toml"

# (bundle_key handed to claude, on-disk filename under <case>/raw/).
# Bundle key matches the prompt-side vocabulary; on-disk name matches the
# project's existing `eval/cases/<id>/raw/` layout. Any file outside this list
# (notably `pr.json`, which sits in the same directory) is never read.
ALLOWED_INPUT_FILES: tuple[tuple[str, str], ...] = (
    ("diff.patch", "diff.patch"),
    ("changed_files.txt", "files.txt"),
)
ALLOWED_INPUT_FILENAMES = tuple(name for _, name in ALLOWED_INPUT_FILES)
SCRIPTS_DIR = Path(__file__).resolve().parent


def _load_helper(module_name: str, filename: str):
    spec = importlib.util.spec_from_file_location(module_name, SCRIPTS_DIR / filename)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def _validator_mod():
    return _load_helper("validate_head_to_head_manifest", "validate-head-to-head-manifest.py")


def _normalizer_mod():
    return _load_helper("normalize_ai_output", "normalize-ai-output.py")


@dataclass(frozen=True)
class Manifest:
    model: str
    prompt_path: Path
    schema_path: Path
    max_attempts: int
    initial_backoff_seconds: float
    backoff_multiplier: float
    raw: dict[str, Any]


def load_manifest(path: Path) -> Manifest:
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    freeze = data["freeze"]
    model = data["model"]["name"]
    retry = data.get("retry", {})
    return Manifest(
        model=model,
        prompt_path=REPO_ROOT / freeze["prompt_path"],
        schema_path=REPO_ROOT / freeze["schema_path"],
        max_attempts=int(retry.get("max_attempts", 3)),
        initial_backoff_seconds=float(retry.get("initial_backoff_seconds", 4)),
        backoff_multiplier=float(retry.get("backoff_multiplier", 2.0)),
        raw=data,
    )


def _strip_git_status_prefix(line: str) -> str:
    """Strip `git status -s` style prefix from a changed-files line.

    The project's `files.txt` is produced by `git diff --name-status`, which
    yields lines like `M\\tsrc/foo.rs` or `R100\\told\\tnew`. The LLM only
    needs the final path component; status flags would leak vocabulary that
    isn't part of the diff text and would fail subject-in-changed-files
    matching downstream.
    """
    parts = line.rstrip("\n").split("\t")
    if not parts:
        return ""
    # `git diff --name-status` puts the status letter(s) first, then 1-2 paths.
    # For renames we take the destination (last column).
    if len(parts) >= 2 and parts[0] and parts[0][0] in "ACDMRTUXB":
        return parts[-1].strip()
    return line.strip()


def _normalize_changed_files(raw: str) -> str:
    return "\n".join(_strip_git_status_prefix(line) for line in raw.splitlines() if line.strip()) + "\n"


def build_case_input(case_dir: Path) -> dict[str, str]:
    """Build the per-case input bundle handed to claude CLI.

    Fairness contract: only the files listed in ALLOWED_INPUT_FILES are read.
    Any other file in the case directory — PR metadata, CI logs, linked issue
    dumps, etc. — is ignored. Missing required files raise FileNotFoundError.

    The returned bundle keys are the LLM-facing logical names; the on-disk
    filenames may differ to match the project's existing case layout.

    `files.txt` (produced by `git diff --name-status`) is normalized to a
    pure list of paths so that the LLM (and the downstream normalizer) sees
    `src/foo.rs` instead of `M\\tsrc/foo.rs`.
    """
    bundle: dict[str, str] = {}
    for bundle_key, filename in ALLOWED_INPUT_FILES:
        path = case_dir / filename
        if not path.exists():
            raise FileNotFoundError(f"case {case_dir.name}: required input missing: {filename}")
        content = path.read_text()
        if bundle_key == "changed_files.txt":
            content = _normalize_changed_files(content)
        bundle[bundle_key] = content
    return bundle


def render_user_message(case_id: str, manifest_hash: str, bundle: dict[str, str]) -> str:
    """Render the user-side message that follows the prompt-as-system."""
    lines: list[str] = []
    lines.append(f"case_id: {case_id}")
    lines.append(f"manifest_hash: {manifest_hash}")
    lines.append("")
    lines.append("changed_files.txt:")
    lines.append("```")
    lines.append(bundle["changed_files.txt"].rstrip("\n"))
    lines.append("```")
    lines.append("")
    lines.append("diff.patch:")
    lines.append("```diff")
    lines.append(bundle["diff.patch"].rstrip("\n"))
    lines.append("```")
    return "\n".join(lines) + "\n"


def manifest_hash(prompt_path: Path, schema_path: Path) -> str:
    """Combined sha of prompt + schema, used as the per-output manifest_hash."""
    import hashlib

    digest = hashlib.sha256()
    digest.update(prompt_path.read_bytes())
    digest.update(schema_path.read_bytes())
    return digest.hexdigest()


@dataclass(frozen=True)
class ClaudeCallResult:
    ok: bool
    stdout: str
    stderr: str
    exit_code: int


def call_claude(
    *,
    prompt_path: Path,
    model: str,
    user_message: str,
    claude_bin: str = "claude",
) -> ClaudeCallResult:
    """Invoke claude CLI once. Returns raw stdout regardless of JSON shape.

    The prompt file content is prepended to the user message because claude CLI
    `-p` flag expects a single positional prompt. We embed the prompt in the
    user message body and treat the call as one-shot.
    """
    full_prompt = prompt_path.read_text() + "\n\n---\n\n" + user_message
    cmd = [
        claude_bin,
        "-p",
        full_prompt,
        "--model",
        model,
        "--permission-mode",
        "plan",
    ]
    completed = subprocess.run(
        cmd,
        text=True,
        capture_output=True,
        env={**os.environ},
    )
    return ClaudeCallResult(
        ok=completed.returncode == 0,
        stdout=completed.stdout,
        stderr=completed.stderr,
        exit_code=completed.returncode,
    )


def call_with_retry(
    *,
    invoker: Callable[[], ClaudeCallResult],
    max_attempts: int,
    initial_backoff_seconds: float,
    backoff_multiplier: float,
    sleeper: Callable[[float], None] = time.sleep,
) -> list[ClaudeCallResult]:
    """Call `invoker()` up to max_attempts. Returns each attempt's result.

    Backoff is applied between attempts only (not after the last). Failure is
    decided by the caller from the returned list; we just record the trace.
    """
    attempts: list[ClaudeCallResult] = []
    backoff = initial_backoff_seconds
    for attempt in range(1, max_attempts + 1):
        result = invoker()
        attempts.append(result)
        if result.ok:
            return attempts
        if attempt < max_attempts:
            sleeper(backoff)
            backoff *= backoff_multiplier
    return attempts


def normalize_one(
    *,
    raw_text: str,
    case_id: str,
    bundle: dict[str, str],
) -> dict[str, Any]:
    normalizer = _normalizer_mod()
    payload = normalizer.NormalizeInput(
        raw=raw_text,
        diff_patch=bundle["diff.patch"],
        changed_files=[line for line in bundle["changed_files.txt"].splitlines() if line.strip()],
    )
    return normalizer.normalize(payload)


def write_case_outputs(
    case_run_dir: Path,
    *,
    case_id: str,
    attempts: list[ClaudeCallResult],
    normalized: dict[str, Any] | None,
    normalize_error: str | None,
) -> None:
    case_run_dir.mkdir(parents=True, exist_ok=True)
    raw_path = case_run_dir / "raw.json"
    norm_path = case_run_dir / "normalized.json"
    raw_path.write_text(
        json.dumps(
            {
                "case_id": case_id,
                "attempt_count": len(attempts),
                "attempts": [
                    {
                        "ok": a.ok,
                        "exit_code": a.exit_code,
                        "stdout": a.stdout,
                        "stderr_tail": a.stderr[-2000:],
                    }
                    for a in attempts
                ],
            },
            indent=2,
        )
        + "\n"
    )
    if normalized is not None:
        norm_path.write_text(json.dumps(normalized, indent=2, sort_keys=True) + "\n")
    else:
        norm_path.write_text(
            json.dumps(
                {
                    "case_id": case_id,
                    "status": "ai-baseline-failed",
                    "reason": normalize_error or "claude CLI did not return a usable response",
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )


def load_corpus_case_ids(corpus_path: Path) -> list[str]:
    with corpus_path.open("rb") as handle:
        data = tomllib.load(handle)
    cases = data.get("cases", [])
    return [str(c["id"]) for c in cases]


def run_one_case(
    *,
    manifest: Manifest,
    case_id: str,
    cases_dir: Path,
    runs_dir: Path,
    run_id: str,
    dry_run: bool,
    claude_bin: str = "claude",
) -> dict[str, Any]:
    case_dir = cases_dir / case_id / "raw"
    bundle = build_case_input(case_dir)
    mh = manifest_hash(manifest.prompt_path, manifest.schema_path)
    user_message = render_user_message(case_id, mh, bundle)
    case_run_dir = runs_dir / run_id / case_id

    if dry_run:
        case_run_dir.mkdir(parents=True, exist_ok=True)
        (case_run_dir / "dry-run-input.txt").write_text(user_message)
        return {
            "case_id": case_id,
            "status": "dry-run",
            "input_bytes": len(user_message),
        }

    if shutil.which(claude_bin) is None:
        return {"case_id": case_id, "status": "claude-cli-missing"}

    def invoker() -> ClaudeCallResult:
        return call_claude(
            prompt_path=manifest.prompt_path,
            model=manifest.model,
            user_message=user_message,
            claude_bin=claude_bin,
        )

    attempts = call_with_retry(
        invoker=invoker,
        max_attempts=manifest.max_attempts,
        initial_backoff_seconds=manifest.initial_backoff_seconds,
        backoff_multiplier=manifest.backoff_multiplier,
    )
    last = attempts[-1]
    normalized: dict[str, Any] | None = None
    normalize_error: str | None = None
    if last.ok:
        try:
            normalized = normalize_one(
                raw_text=last.stdout,
                case_id=case_id,
                bundle=bundle,
            )
        except Exception as err:  # NormalizeError or any unexpected
            normalize_error = str(err)
    else:
        normalize_error = f"claude CLI exit_code={last.exit_code} after {len(attempts)} attempts"

    write_case_outputs(
        case_run_dir,
        case_id=case_id,
        attempts=attempts,
        normalized=normalized,
        normalize_error=normalize_error,
    )
    return {
        "case_id": case_id,
        "status": "ok" if normalized is not None else "ai-baseline-failed",
        "attempts": len(attempts),
        "normalize_error": normalize_error,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--cases-dir", type=Path, default=DEFAULT_CASES_DIR)
    parser.add_argument("--runs-dir", type=Path, default=DEFAULT_RUNS_DIR)
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--case", action="append", default=[], dest="cases", help="case id; repeatable")
    parser.add_argument("--all", action="store_true", help="run every case in the corpus")
    parser.add_argument("--run-id", default=None, help="defaults to a timestamped uuid")
    parser.add_argument("--dry-run", action="store_true", help="build inputs but do not call claude CLI")
    parser.add_argument("--claude-bin", default="claude")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])

    validator = _validator_mod()
    gate = validator.validate_manifest(args.manifest.resolve())
    if not gate.ok:
        print(f"manifest hash gate failed: {gate.message}", file=sys.stderr)
        return gate.code

    manifest = load_manifest(args.manifest)

    if args.all:
        case_ids = load_corpus_case_ids(args.corpus)
    elif args.cases:
        case_ids = list(args.cases)
    else:
        print("must pass --case <id> [--case <id> ...] or --all", file=sys.stderr)
        return 2

    run_id = args.run_id or f"{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{uuid.uuid4().hex[:6]}"

    summary: list[dict[str, Any]] = []
    for case_id in case_ids:
        try:
            row = run_one_case(
                manifest=manifest,
                case_id=case_id,
                cases_dir=args.cases_dir,
                runs_dir=args.runs_dir,
                run_id=run_id,
                dry_run=args.dry_run,
                claude_bin=args.claude_bin,
            )
        except FileNotFoundError as err:
            row = {"case_id": case_id, "status": "case-input-missing", "error": str(err)}
        summary.append(row)
        print(json.dumps(row, sort_keys=True))

    run_dir = args.runs_dir / run_id
    run_dir.mkdir(parents=True, exist_ok=True)
    (run_dir / "summary.jsonl").write_text("\n".join(json.dumps(r, sort_keys=True) for r in summary) + "\n")
    print(f"wrote {run_dir / 'summary.jsonl'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
