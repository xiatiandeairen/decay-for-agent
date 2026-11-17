#!/usr/bin/env python3
"""Run the AI judge against a single case (or many).

Pipeline per case:
  1. Verify the judge manifest hash gate.
  2. Build the per-case input bundle. Unlike the AI baseline, the judge sees
     PR full metadata (pr.json), the diff, the changed-files list, decay's
     fact and semantic outputs, and the AI baseline's normalized output.
  3. Invoke claude CLI with the frozen judge prompt and model.
  4. Normalize the response into the judge schema (two-layer: ground_truth_draft
     + judgement_drafts × 3 modes).
  5. Mechanically recompute `top3_useful` per mode and cross-check.
  6. Persist raw.json + normalized.json + mechanical.json under
     `runs/judge-<run-id>/<case-id>/`.
"""

from __future__ import annotations

import argparse
import hashlib
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
DEFAULT_MANIFEST = HEAD_TO_HEAD_DIR / "manifests" / "judge-v1.toml"
DEFAULT_RUNS_DIR = HEAD_TO_HEAD_DIR / "runs"
DEFAULT_CASES_DIR = EVAL_ROOT / "cases"
DEFAULT_CORPUS = EVAL_ROOT / "corpus" / "risk-signal-frozen-v1.toml"

SCRIPTS_DIR = Path(__file__).resolve().parent


def _load_helper(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS_DIR / filename)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def _validator_mod():
    return _load_helper("validate_judge_manifest", "validate-judge-manifest.py")


def _normalizer_mod():
    return _load_helper("normalize_judge_output", "normalize-judge-output.py")


def _recompute_mod():
    return _load_helper("mechanical_recompute_judgement", "mechanical-recompute-judgement.py")


@dataclass(frozen=True)
class Manifest:
    model: str
    prompt_path: Path
    schema_path: Path
    max_attempts: int
    initial_backoff_seconds: float
    backoff_multiplier: float


def load_manifest(path: Path) -> Manifest:
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    freeze = data["freeze"]
    retry = data.get("retry", {})
    return Manifest(
        model=data["model"]["name"],
        prompt_path=REPO_ROOT / freeze["prompt_path"],
        schema_path=REPO_ROOT / freeze["schema_path"],
        max_attempts=int(retry.get("max_attempts", 3)),
        initial_backoff_seconds=float(retry.get("initial_backoff_seconds", 4)),
        backoff_multiplier=float(retry.get("backoff_multiplier", 2.0)),
    )


def manifest_hash(prompt_path: Path, schema_path: Path) -> str:
    h = hashlib.sha256()
    h.update(prompt_path.read_bytes())
    h.update(schema_path.read_bytes())
    return h.hexdigest()


def build_judge_input(
    case_id: str,
    case_raw_dir: Path,
    ai_baseline_normalized_path: Path | None,
) -> dict[str, str]:
    """Build the judge's per-case input bundle.

    The judge sees PR full metadata plus all three pipelines' outputs:
      - pr.json (full PR metadata: title, body, reviews, comments, labels)
      - diff.patch (unified diff)
      - files.txt (changed files in git diff --name-status format)
      - decay.stdout (fact-only pipeline output)
      - semantic-findings.json (semantic-filtered pipeline output)
      - ai-baseline.normalized.json (AI baseline pipeline output)

    Missing optional files are tolerated; missing required files raise
    FileNotFoundError.
    """
    bundle: dict[str, str] = {}

    required = ("pr.json", "diff.patch", "files.txt")
    for name in required:
        path = case_raw_dir / name
        if not path.exists():
            raise FileNotFoundError(f"case {case_id}: required input missing: {name}")
        bundle[name] = path.read_text()

    optional = ("decay.stdout", "semantic-findings.json")
    for name in optional:
        path = case_raw_dir / name
        bundle[name] = path.read_text() if path.exists() else ""

    if ai_baseline_normalized_path is not None and ai_baseline_normalized_path.exists():
        bundle["ai-baseline.normalized.json"] = ai_baseline_normalized_path.read_text()
    else:
        bundle["ai-baseline.normalized.json"] = ""

    return bundle


def render_user_message(case_id: str, mh: str, bundle: dict[str, str]) -> str:
    lines: list[str] = []
    lines.append(f"case_id: {case_id}")
    lines.append(f"manifest_hash: {mh}")
    for key in (
        "pr.json",
        "files.txt",
        "diff.patch",
        "decay.stdout",
        "semantic-findings.json",
        "ai-baseline.normalized.json",
    ):
        body = bundle.get(key, "")
        if not body:
            lines.append("")
            lines.append(f"{key}: (empty)")
            continue
        lines.append("")
        lines.append(f"{key}:")
        fence = "```json" if key.endswith(".json") else "```"
        lines.append(fence)
        lines.append(body.rstrip("\n"))
        lines.append("```")
    return "\n".join(lines) + "\n"


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
    full_prompt = prompt_path.read_text() + "\n\n---\n\n" + user_message
    cmd = [claude_bin, "-p", full_prompt, "--model", model, "--permission-mode", "plan"]
    completed = subprocess.run(cmd, text=True, capture_output=True, env={**os.environ})
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


def run_one_case(
    *,
    manifest: Manifest,
    case_id: str,
    cases_dir: Path,
    ai_baseline_run_dir: Path | None,
    runs_dir: Path,
    run_id: str,
    dry_run: bool,
    claude_bin: str = "claude",
) -> dict[str, Any]:
    case_raw_dir = cases_dir / case_id / "raw"
    ai_baseline_normalized = (
        ai_baseline_run_dir / case_id / "normalized.json" if ai_baseline_run_dir else None
    )
    bundle = build_judge_input(case_id, case_raw_dir, ai_baseline_normalized)

    mh = manifest_hash(manifest.prompt_path, manifest.schema_path)
    user_message = render_user_message(case_id, mh, bundle)

    case_run_dir = runs_dir / f"judge-{run_id}" / case_id
    case_run_dir.mkdir(parents=True, exist_ok=True)

    if dry_run:
        (case_run_dir / "dry-run-input.txt").write_text(user_message)
        return {
            "case_id": case_id,
            "status": "dry-run",
            "input_bytes": len(user_message),
            "has_pr_metadata": "pr.json" in user_message and len(bundle["pr.json"]) > 0,
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

    raw_path = case_run_dir / "raw.json"
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

    last = attempts[-1]
    norm_path = case_run_dir / "normalized.json"
    mech_path = case_run_dir / "mechanical.json"

    if not last.ok:
        norm_path.write_text(
            json.dumps(
                {"case_id": case_id, "status": "judge-failed", "reason": f"exit_code={last.exit_code}"},
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
        return {"case_id": case_id, "status": "judge-failed", "attempts": len(attempts)}

    normalizer = _normalizer_mod()
    try:
        normalized = normalizer.normalize(normalizer.NormalizeInput(raw=last.stdout))
    except normalizer.NormalizeError as err:
        norm_path.write_text(
            json.dumps(
                {"case_id": case_id, "status": "judge-failed", "reason": str(err)},
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
        return {"case_id": case_id, "status": "judge-failed", "normalize_error": str(err)}

    norm_path.write_text(json.dumps(normalized, indent=2, sort_keys=True) + "\n")

    # Mechanical recompute
    recompute = _recompute_mod()
    ai_top_risks = None
    if ai_baseline_normalized is not None and ai_baseline_normalized.exists():
        ai_data = json.loads(ai_baseline_normalized.read_text())
        if ai_data.get("status") != "ai-baseline-failed":
            ai_top_risks = ai_data.get("top_risks", [])
    mech = recompute.recompute(
        judge_output=normalized,
        top_risks_per_mode={
            "fact_only": None,
            "semantic_filtered": None,
            "ai_baseline": ai_top_risks,
        },
    )
    mech_path.write_text(json.dumps(mech, indent=2, sort_keys=True) + "\n")

    return {"case_id": case_id, "status": "ok", "attempts": len(attempts), "flags": mech["flags"]}


def load_corpus_case_ids(corpus_path: Path) -> list[str]:
    with corpus_path.open("rb") as handle:
        data = tomllib.load(handle)
    return [str(c["id"]) for c in data.get("cases", [])]


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--cases-dir", type=Path, default=DEFAULT_CASES_DIR)
    parser.add_argument(
        "--ai-baseline-run-dir",
        type=Path,
        help="ai-baseline run directory (used to locate per-case normalized.json)",
    )
    parser.add_argument("--runs-dir", type=Path, default=DEFAULT_RUNS_DIR)
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--case", action="append", default=[], dest="cases")
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--run-id", default=None)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--claude-bin", default="claude")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])

    validator = _validator_mod()
    gate = validator.validate_manifest(args.manifest.resolve())
    if not gate.ok:
        print(f"judge manifest hash gate failed: {gate.message}", file=sys.stderr)
        return gate.code

    manifest = load_manifest(args.manifest)
    case_ids = load_corpus_case_ids(args.corpus) if args.all else list(args.cases)
    if not case_ids:
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
                ai_baseline_run_dir=args.ai_baseline_run_dir,
                runs_dir=args.runs_dir,
                run_id=run_id,
                dry_run=args.dry_run,
                claude_bin=args.claude_bin,
            )
        except FileNotFoundError as err:
            row = {"case_id": case_id, "status": "case-input-missing", "error": str(err)}
        summary.append(row)
        print(json.dumps(row, sort_keys=True))

    run_dir = args.runs_dir / f"judge-{run_id}"
    (run_dir / "summary.jsonl").write_text("\n".join(json.dumps(r, sort_keys=True) for r in summary) + "\n")
    print(f"wrote {run_dir / 'summary.jsonl'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
