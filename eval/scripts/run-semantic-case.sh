#!/usr/bin/env bash
set -euo pipefail

EVAL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASE_ID="${1:-}"
CODEX_BIN="${CODEX_BIN:-codex}"

if [[ -z "$CASE_ID" ]]; then
  echo "usage: eval/scripts/run-semantic-case.sh <case-id>" >&2
  exit 2
fi

command -v "$CODEX_BIN" >/dev/null || { echo "codex CLI is required" >&2; exit 2; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 2; }

"$EVAL_ROOT/scripts/run-pr-case.sh" "$CASE_ID"

CASE_DIR="$EVAL_ROOT/cases/$CASE_ID"
RAW_DIR="$CASE_DIR/raw"
PROMPT="$RAW_DIR/semantic-prompt.md"
OUT="$RAW_DIR/semantic-findings.json"
SCHEMA="$EVAL_ROOT/schemas/semantic-findings.schema.json"

python3 "$EVAL_ROOT/scripts/build-semantic-prompt.py" "$RAW_DIR/decay.stdout" "$PROMPT"

"$CODEX_BIN" exec \
  --cd "$EVAL_ROOT/.." \
  --sandbox read-only \
  --output-schema "$SCHEMA" \
  --output-last-message "$OUT" \
  - < "$PROMPT"

python3 "$EVAL_ROOT/scripts/validate-semantic-findings.py" "$RAW_DIR/decay.stdout" "$OUT"

echo "wrote $OUT"
