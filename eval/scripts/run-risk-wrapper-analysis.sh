#!/usr/bin/env bash
set -euo pipefail

EVAL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASE_ID="${1:-}"
CODEX_BIN="${CODEX_BIN:-codex}"

if [[ -z "$CASE_ID" ]]; then
  echo "usage: eval/scripts/run-risk-wrapper-analysis.sh <case-id>" >&2
  exit 2
fi

command -v "$CODEX_BIN" >/dev/null || { echo "codex CLI is required" >&2; exit 2; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 2; }

"$EVAL_ROOT/scripts/run-pr-case.sh" "$CASE_ID"

CASE_DIR="$EVAL_ROOT/cases/$CASE_ID"
RAW_DIR="$CASE_DIR/raw"
BUNDLE="$RAW_DIR/risk-wrapper.json"
PROMPT="$RAW_DIR/risk-wrapper-prompt.md"
OUT="$RAW_DIR/semantic-confidence-evidence.json"
HONEST_OUT="$RAW_DIR/honest-review-analysis.json"
SCHEMA="$EVAL_ROOT/schemas/honest-review-analysis.schema.json"

python3 "$EVAL_ROOT/scripts/build-risk-wrapper.py" \
  --case-id "$CASE_ID" \
  --decay-stdout "$RAW_DIR/decay.stdout" \
  --diff-patch "$RAW_DIR/diff.patch" \
  --pr-json "$RAW_DIR/pr.json" \
  --out "$BUNDLE"

{
  printf '%s\n\n' '# Task'
  printf '%s\n' 'You are analyzing risk from machine-built decay risk wrappers that include honest_review.'
  printf '%s\n' 'Use only the JSON bundle below. Do not use external knowledge or web search.'
  printf '%s\n' 'Return JSON only, matching honest-review-analysis.v1.'
  printf '%s\n' 'For each wrapper with honest_review, preserve the same mechanism/impact/control statuses or lower them.'
  printf '%s\n' 'Every supported_by item must be copied exactly from honest_review observed or inferred.'
  printf '%s\n' 'Every honest_review unknown must remain visible in limits or uncertainties.'
  printf '%s\n' 'Copy all honest_review overall.non_claims into the output non_claims.'
  printf '%s\n' 'Do not claim a bug, do not prove complete coverage, and do not give a merge recommendation.'
  printf '\n%s\n' '# Bundle'
  printf '%s\n' '```json'
  cat "$BUNDLE"
  printf '%s\n' '```'
} > "$PROMPT"

"$CODEX_BIN" exec \
  --cd "$EVAL_ROOT/.." \
  --sandbox read-only \
  --output-schema "$SCHEMA" \
  --output-last-message "$HONEST_OUT" \
  - < "$PROMPT"

python3 "$EVAL_ROOT/scripts/validate-honest-review-analysis.py" \
  --bundle "$BUNDLE" \
  --analysis "$HONEST_OUT"

echo "wrote $HONEST_OUT"
