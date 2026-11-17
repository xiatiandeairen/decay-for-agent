#!/usr/bin/env python3
"""Normalize raw claude CLI output from the judge runner into the v1 schema.

Reuses three helpers from the ai-baseline normalizer for shape recovery:

  - `strip_fence`: drop optional ```json...``` wrap
  - `_try_repair_truncated_root`: append a missing `}` if the model closed
    every nested container except the root
  - `MAX_RATIONALE_CHARS` / `_truncate_rationale`: tolerate slight overshoot

The schema rules are different from the AI baseline (judge has a two-layer
shape: `ground_truth_draft` + `judgement_drafts` × 3 modes), so the
validation logic is bespoke.

The normalizer is pure: it takes strings and returns a dict or raises.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = Path(__file__).resolve().parent

ALLOWED_RISK_SIGNALS = frozenset(
    {
        "assumption_boundary",
        "non_happy_path",
        "state_transition",
        "error_propagation",
        "lifecycle_ordering",
        "concurrency_ordering",
        "external_contract",
        "test_gap_for_risk_path",
    }
)
ALLOWED_MODES = ("fact_only", "semantic_filtered", "ai_baseline")
MAX_TOP_RISKS = 3
HEX64_RE = __import__("re").compile(r"^[0-9a-f]{64}$")


def _load_ai_baseline_helpers():
    spec = importlib.util.spec_from_file_location(
        "normalize_ai_output_helpers",
        SCRIPTS_DIR / "normalize-ai-output.py",
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


_AI_HELPERS = _load_ai_baseline_helpers()


class NormalizeError(ValueError):
    """Raised when the raw judge output cannot be normalized into a valid object."""


@dataclass(frozen=True)
class NormalizeInput:
    raw: str


def _require_type(node: dict, key: str, expected: type, where: str) -> Any:
    if key not in node:
        raise NormalizeError(f"{where}: missing required field '{key}'")
    value = node[key]
    if isinstance(expected, tuple):
        if not isinstance(value, expected):
            raise NormalizeError(f"{where}: '{key}' has wrong type")
    else:
        if not isinstance(value, expected):
            raise NormalizeError(f"{where}: '{key}' has wrong type")
        if expected is str and not value:
            raise NormalizeError(f"{where}: '{key}' must be a non-empty string")
    return value


def _validate_mode(mode_name: str, mode: Any) -> None:
    where = f"judgement_drafts.{mode_name}"
    if not isinstance(mode, dict):
        raise NormalizeError(f"{where}: must be an object")

    top3_total = mode.get("top3_total")
    if not isinstance(top3_total, int) or isinstance(top3_total, bool) or not 0 <= top3_total <= 3:
        raise NormalizeError(f"{where}: 'top3_total' must be an integer in 0..3")
    top3_useful = mode.get("top3_useful")
    if not isinstance(top3_useful, int) or isinstance(top3_useful, bool) or not 0 <= top3_useful <= 3:
        raise NormalizeError(f"{where}: 'top3_useful' must be an integer in 0..3")
    if top3_useful > top3_total:
        raise NormalizeError(f"{where}: top3_useful ({top3_useful}) cannot exceed top3_total ({top3_total})")

    actionable = mode.get("actionable")
    if not isinstance(actionable, bool):
        raise NormalizeError(f"{where}: 'actionable' must be a boolean")

    missed = mode.get("missed_high_risk_count")
    if not isinstance(missed, int) or isinstance(missed, bool) or missed < 0:
        raise NormalizeError(f"{where}: 'missed_high_risk_count' must be a non-negative integer")

    weak = mode.get("weak_only_top_risk_count")
    if not isinstance(weak, int) or isinstance(weak, bool) or weak not in (0, 1):
        raise NormalizeError(f"{where}: 'weak_only_top_risk_count' must be 0 or 1")

    notes = mode.get("notes")
    if not isinstance(notes, str):
        raise NormalizeError(f"{where}: 'notes' must be a string")
    if len(notes) > 300:
        raise NormalizeError(f"{where}: 'notes' length {len(notes)} exceeds max 300")

    extra = set(mode.keys()) - {
        "top3_total",
        "top3_useful",
        "actionable",
        "missed_high_risk_count",
        "weak_only_top_risk_count",
        "notes",
    }
    if extra:
        raise NormalizeError(f"{where}: unexpected fields {sorted(extra)}")


def _validate_ground_truth_draft(gt: Any) -> None:
    where = "ground_truth_draft"
    if not isinstance(gt, dict):
        raise NormalizeError(f"{where}: must be an object")

    if "high_value" not in gt or not isinstance(gt["high_value"], bool):
        raise NormalizeError(f"{where}: 'high_value' must be a boolean")

    psi = gt.get("primary_signal_ids")
    if not isinstance(psi, list):
        raise NormalizeError(f"{where}: 'primary_signal_ids' must be a list")
    for i, sig in enumerate(psi):
        if not isinstance(sig, str):
            raise NormalizeError(f"{where}.primary_signal_ids[{i}]: must be a string")
        if sig not in ALLOWED_RISK_SIGNALS:
            raise NormalizeError(
                f"{where}.primary_signal_ids[{i}]: '{sig}' not in allowed enum {sorted(ALLOWED_RISK_SIGNALS)}"
            )

    subs = gt.get("subject_files")
    if not isinstance(subs, list):
        raise NormalizeError(f"{where}: 'subject_files' must be a list")
    for i, f in enumerate(subs):
        if not isinstance(f, str) or not f:
            raise NormalizeError(f"{where}.subject_files[{i}]: must be a non-empty string")

    eb = gt.get("expected_behavior")
    if not isinstance(eb, str):
        raise NormalizeError(f"{where}: 'expected_behavior' must be a string")
    if len(eb) > 400:
        raise NormalizeError(f"{where}: 'expected_behavior' length {len(eb)} exceeds max 400")

    rat = gt.get("rationale_from_pr_metadata")
    if not isinstance(rat, str):
        raise NormalizeError(f"{where}: 'rationale_from_pr_metadata' must be a string")
    if len(rat) > 600:
        raise NormalizeError(f"{where}: 'rationale_from_pr_metadata' length {len(rat)} exceeds max 600")

    if gt["high_value"] and not psi:
        raise NormalizeError(f"{where}: high_value=true requires non-empty primary_signal_ids")

    extra = set(gt.keys()) - {
        "high_value",
        "primary_signal_ids",
        "subject_files",
        "expected_behavior",
        "rationale_from_pr_metadata",
    }
    if extra:
        raise NormalizeError(f"{where}: unexpected fields {sorted(extra)}")


def _validate_root(data: Any) -> dict[str, Any]:
    if not isinstance(data, dict):
        raise NormalizeError("root: must be a JSON object")
    case_id = _require_type(data, "case_id", str, "root")
    model = _require_type(data, "model", str, "root")
    manifest_hash = _require_type(data, "manifest_hash", str, "root")
    if not HEX64_RE.match(manifest_hash):
        raise NormalizeError("root: manifest_hash must be a 64-char hex string")

    if "ground_truth_draft" not in data:
        raise NormalizeError("root: missing required field 'ground_truth_draft'")
    _validate_ground_truth_draft(data["ground_truth_draft"])

    if "judgement_drafts" not in data:
        raise NormalizeError("root: missing required field 'judgement_drafts'")
    drafts = data["judgement_drafts"]
    if not isinstance(drafts, dict):
        raise NormalizeError("root.judgement_drafts: must be an object")
    missing_modes = [m for m in ALLOWED_MODES if m not in drafts]
    if missing_modes:
        raise NormalizeError(f"root.judgement_drafts: missing modes {missing_modes}")
    for mode_name in ALLOWED_MODES:
        _validate_mode(mode_name, drafts[mode_name])
    extra_modes = set(drafts.keys()) - set(ALLOWED_MODES)
    if extra_modes:
        raise NormalizeError(f"root.judgement_drafts: unexpected modes {sorted(extra_modes)}")

    extra = set(data.keys()) - {"case_id", "model", "manifest_hash", "ground_truth_draft", "judgement_drafts"}
    if extra:
        raise NormalizeError(f"root: unexpected fields {sorted(extra)}")

    return data


def _try_repair_truncated_root(text: str) -> str | None:
    """The judge schema is nested (ground_truth_draft + judgement_drafts × 3
    modes), so the AI-baseline ``}]`` heuristic doesn't apply — the tail is
    usually ``}}}``. Try appending 1..3 closing braces and keep the first
    candidate that parses cleanly.
    """
    stripped = text.rstrip()
    if not stripped or not stripped.endswith("}"):
        return None
    for n in range(1, 4):
        candidate = stripped + ("}" * n)
        try:
            json.loads(candidate)
            return candidate
        except json.JSONDecodeError:
            continue
    return None


def normalize(payload: NormalizeInput) -> dict[str, Any]:
    text = _AI_HELPERS.strip_fence(payload.raw)
    try:
        data = json.loads(text)
    except json.JSONDecodeError as err:
        repaired = _try_repair_truncated_root(text)
        if repaired is None:
            raise NormalizeError(f"raw output is not valid JSON: {err}") from err
        try:
            data = json.loads(repaired)
        except json.JSONDecodeError as err2:
            raise NormalizeError(
                f"raw output is not valid JSON even after truncation repair: {err2}"
            ) from err2

    return _validate_root(data)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--raw", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    payload = NormalizeInput(raw=args.raw.read_text())
    try:
        normalized = normalize(payload)
    except NormalizeError as err:
        print(f"normalize failed: {err}", file=sys.stderr)
        return 1
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(normalized, indent=2, sort_keys=True) + "\n")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
