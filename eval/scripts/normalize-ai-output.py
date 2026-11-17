#!/usr/bin/env python3
"""Normalize raw claude CLI output into the AI baseline v1 schema.

The pipeline:
  1. Strip optional ```json ... ``` fences if the model wrapped its JSON.
  2. Parse JSON. If parsing fails, raise NormalizeError.
  3. Validate against the AI baseline schema rules (top_risks <=3, risk_signal
     in enum, direct => confidence in {high, medium}, etc.).
  4. Verify each evidence_ref appears verbatim in the case diff.patch.
  5. Verify each top_risks[].subject references a path in changed_files.txt.

Schema rules are checked by hand instead of `jsonschema` to match the rest of
`eval/` (six schemas under `eval/schemas/`, none use the library). The JSON
schema file is still the authoritative contract for external consumers; this
module enforces the same rules in code.

The normalizer is pure: it takes strings (and the path-to-grep) and returns
the normalized dict, or raises. It does no IO beyond reading diff.patch /
changed_files.txt explicitly passed in.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SCHEMA = REPO_ROOT / "eval" / "head-to-head" / "schemas" / "ai-baseline-output.schema.json"

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
ALLOWED_CONFIDENCE = frozenset({"high", "medium", "low"})
ALLOWED_VALUE = frozenset({"direct", "supporting", "context"})
MAX_TOP_RISKS = 3
MAX_RATIONALE_CHARS = 200
HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
_FENCE_RE = re.compile(r"^\s*```(?:json)?\s*\n(.*?)\n```\s*$", re.DOTALL)


class NormalizeError(ValueError):
    """Raised when the raw AI output cannot be normalized into a valid object."""


@dataclass(frozen=True)
class NormalizeInput:
    raw: str
    diff_patch: str
    changed_files: list[str]


def strip_fence(raw: str) -> str:
    match = _FENCE_RE.match(raw)
    if match:
        return match.group(1)
    return raw.strip()


def _require_str(node: dict, key: str, where: str) -> str:
    if key not in node:
        raise NormalizeError(f"{where}: missing required field '{key}'")
    value = node[key]
    if not isinstance(value, str) or not value:
        raise NormalizeError(f"{where}: '{key}' must be a non-empty string")
    return value


def _validate_top_risk(idx: int, risk: Any) -> None:
    where = f"top_risks[{idx}]"
    if not isinstance(risk, dict):
        raise NormalizeError(f"{where}: must be an object")

    rank = risk.get("rank")
    if not isinstance(rank, int) or isinstance(rank, bool) or not 1 <= rank <= MAX_TOP_RISKS:
        raise NormalizeError(f"{where}: 'rank' must be an integer in 1..{MAX_TOP_RISKS}")

    risk_signal = _require_str(risk, "risk_signal", where)
    if risk_signal not in ALLOWED_RISK_SIGNALS:
        raise NormalizeError(
            f"{where}: risk_signal '{risk_signal}' not in allowed enum {sorted(ALLOWED_RISK_SIGNALS)}"
        )

    _require_str(risk, "subject", where)

    refs = risk.get("evidence_refs")
    if not isinstance(refs, list) or len(refs) < 1:
        raise NormalizeError(f"{where}: 'evidence_refs' must be a non-empty array")
    for ref_idx, ref in enumerate(refs):
        if not isinstance(ref, str) or not ref:
            raise NormalizeError(f"{where}.evidence_refs[{ref_idx}]: must be a non-empty string")

    confidence = _require_str(risk, "confidence", where)
    if confidence not in ALLOWED_CONFIDENCE:
        raise NormalizeError(f"{where}: confidence '{confidence}' not in {sorted(ALLOWED_CONFIDENCE)}")

    value = _require_str(risk, "value", where)
    if value not in ALLOWED_VALUE:
        raise NormalizeError(f"{where}: value '{value}' not in {sorted(ALLOWED_VALUE)}")

    rationale_raw = _require_str(risk, "rationale", where)
    rationale = _truncate_rationale(rationale_raw)
    if len(rationale) > MAX_RATIONALE_CHARS:
        raise NormalizeError(
            f"{where}: rationale length {len(rationale_raw)} exceeds max {MAX_RATIONALE_CHARS}"
        )
    risk["rationale"] = rationale

    if value == "direct" and confidence == "low":
        raise NormalizeError(f"{where}: value=direct requires confidence in {{high, medium}}, got low")

    extra = set(risk.keys()) - {
        "rank",
        "risk_signal",
        "subject",
        "evidence_refs",
        "confidence",
        "value",
        "rationale",
    }
    if extra:
        raise NormalizeError(f"{where}: unexpected fields {sorted(extra)}")


def _validate_root(data: Any) -> dict[str, Any]:
    if not isinstance(data, dict):
        raise NormalizeError("root: must be a JSON object")
    case_id = _require_str(data, "case_id", "root")
    model = _require_str(data, "model", "root")
    manifest_hash = _require_str(data, "manifest_hash", "root")
    if not HEX64_RE.match(manifest_hash):
        raise NormalizeError("root: manifest_hash must be a 64-char hex string")

    if "top_risks" not in data:
        raise NormalizeError("root: missing required field 'top_risks'")
    top_risks = data["top_risks"]
    if not isinstance(top_risks, list):
        raise NormalizeError("root: 'top_risks' must be an array")
    if len(top_risks) > MAX_TOP_RISKS:
        raise NormalizeError(f"root: top_risks has {len(top_risks)} items; max is {MAX_TOP_RISKS}")
    for idx, risk in enumerate(top_risks):
        _validate_top_risk(idx, risk)

    extra = set(data.keys()) - {"case_id", "model", "manifest_hash", "top_risks"}
    if extra:
        raise NormalizeError(f"root: unexpected fields {sorted(extra)}")

    return {
        "case_id": case_id,
        "model": model,
        "manifest_hash": manifest_hash,
        "top_risks": top_risks,
    }


def _strip_diff_prefix(diff: str) -> str:
    """Return a substring-search blob of the diff with unified-diff line prefixes removed.

    The on-disk diff has lines like `+    let x = 1;` / `-    let y = 2;` /
    ` (context)`. Models routinely paraphrase by dropping the leading `+`/`-`
    when quoting the new (or removed) code. We accept both forms by searching
    the original diff first and a prefix-stripped blob second.
    """
    out: list[str] = []
    for line in diff.splitlines(keepends=True):
        if line.startswith("+++") or line.startswith("---") or line.startswith("@@"):
            out.append(line)
            continue
        if line[:1] in {"+", "-", " "}:
            out.append(line[1:])
        else:
            out.append(line)
    return "".join(out)


_WS_RE = re.compile(r"\s+")


def _ws_normalize(text: str) -> str:
    return _WS_RE.sub(" ", text).strip()


def _evidence_ref_present(ref: str, diff: str, diff_stripped: str) -> bool:
    if ref in diff or ref in diff_stripped:
        return True
    # Models sometimes mix `+`/`-` prefixes inside the same quoted ref. Strip
    # the ref too and retry.
    ref_stripped = _strip_diff_prefix(ref)
    if ref_stripped in diff or ref_stripped in diff_stripped:
        return True
    # Indentation drift (one-space shift, tabs vs spaces, fold-up of blank
    # lines) is a common paraphrase artefact. Last-resort: collapse all runs
    # of whitespace and search. We still require the diff to literally
    # contain every non-whitespace character in the ref, so the model cannot
    # fabricate content this way.
    ref_norm = _ws_normalize(ref_stripped)
    if not ref_norm:
        return False
    return ref_norm in _ws_normalize(diff) or ref_norm in _ws_normalize(diff_stripped)


def _truncate_rationale(value: str, hard_max: int = 240) -> str:
    """Schema caps rationale at 200; some samples come back at 201-220.
    Truncate gently with an ellipsis instead of rejecting the whole row.
    Beyond hard_max we still reject (handled by the validator).
    """
    if len(value) <= MAX_RATIONALE_CHARS:
        return value
    if len(value) <= hard_max:
        return value[: MAX_RATIONALE_CHARS - 1].rstrip() + "…"
    return value


def _try_repair_truncated_root(text: str) -> str | None:
    """Best-effort one-shot repair for a missing closing `}` on the root.

    Pattern observed in claude-opus-4-7 sampling: the model closes the
    `top_risks` array but forgets the outer object. If the text ends with
    `}]` (plus optional whitespace), append `}` and retry once.
    """
    stripped = text.rstrip()
    if stripped.endswith("}]"):
        return stripped + "}"
    return None


def normalize(payload: NormalizeInput) -> dict[str, Any]:
    text = strip_fence(payload.raw)
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

    normalized = _validate_root(data)

    changed_set = {f.strip() for f in payload.changed_files if f.strip()}
    diff_stripped = _strip_diff_prefix(payload.diff_patch)
    for idx, risk in enumerate(normalized["top_risks"]):
        subject = risk["subject"]
        file_part = subject.split(":", 1)[0]
        if file_part not in changed_set:
            raise NormalizeError(
                f"top_risks[{idx}].subject references unknown file '{file_part}'; not in changed_files"
            )
        for ref_idx, ref in enumerate(risk["evidence_refs"]):
            if not _evidence_ref_present(ref, payload.diff_patch, diff_stripped):
                raise NormalizeError(
                    f"top_risks[{idx}].evidence_refs[{ref_idx}] not found verbatim in diff.patch"
                )

    return normalized


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--raw", type=Path, required=True, help="raw claude CLI output (utf-8 text)")
    parser.add_argument("--diff", type=Path, required=True, help="case diff.patch")
    parser.add_argument("--changed-files", type=Path, required=True, help="case changed_files.txt")
    parser.add_argument("--out", type=Path, required=True, help="normalized JSON output path")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    payload = NormalizeInput(
        raw=args.raw.read_text(),
        diff_patch=args.diff.read_text(),
        changed_files=[line for line in args.changed_files.read_text().splitlines() if line.strip()],
    )
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
