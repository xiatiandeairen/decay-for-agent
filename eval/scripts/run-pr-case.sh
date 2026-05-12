#!/usr/bin/env bash
set -euo pipefail

EVAL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS="$EVAL_ROOT/corpus/pr-cases.toml"
CASE_ID="${1:-}"

if [[ -z "$CASE_ID" ]]; then
  echo "usage: eval/scripts/run-pr-case.sh <case-id>" >&2
  exit 2
fi

case_block="$(awk -v id="$CASE_ID" '
  /^\[\[cases\]\]/ { in_case = 1; block = $0 "\n"; next }
  in_case { block = block $0 "\n" }
  in_case && $0 == "" {
    if (block ~ "id = \"" id "\"") { print block; found = 1; exit }
    in_case = 0; block = ""
  }
  END {
    if (!found && in_case && block ~ "id = \"" id "\"") print block
  }
' "$CORPUS")"

if [[ -z "$case_block" ]]; then
  echo "case not found: $CASE_ID" >&2
  exit 2
fi

value() {
  local key="$1"
  printf '%s\n' "$case_block" | awk -F' = ' -v key="$key" '$1 == key { gsub(/^"|"$/, "", $2); print $2; exit }'
}

REPO="$(value repo)"
PR="$(value pr)"
KIND="$(value kind)"

if [[ -z "$REPO" || -z "$PR" || -z "$KIND" ]]; then
  echo "invalid case metadata for $CASE_ID" >&2
  exit 2
fi

command -v gh >/dev/null || { echo "gh is required" >&2; exit 2; }
command -v git >/dev/null || { echo "git is required" >&2; exit 2; }
command -v decay >/dev/null || { echo "decay is required; run cargo install --path ." >&2; exit 2; }

WORK_ROOT="${DECAY_EVAL_WORK_ROOT:-/private/tmp/decay-pr-eval}"
RUN_ROOT="$WORK_ROOT/$CASE_ID"
REPO_DIR="$RUN_ROOT/repo"
RESULT_DIR="$EVAL_ROOT/results/$CASE_ID"
DB_PATH="$RESULT_DIR/decay.db"

mkdir -p "$RUN_ROOT" "$RESULT_DIR"

if [[ ! -d "$REPO_DIR/.git" ]]; then
  gh repo clone "$REPO" "$REPO_DIR" -- --quiet
fi

pushd "$REPO_DIR" >/dev/null
git fetch --quiet origin

pr_json="$(gh pr view "$PR" --repo "$REPO" --json number,title,url,state,baseRefName,baseRefOid,headRefName,headRefOid,labels,files,reviewDecision,comments,reviews,mergedAt)"
printf '%s\n' "$pr_json" > "$RESULT_DIR/pr.json"

base_sha="$(printf '%s\n' "$pr_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["baseRefOid"])')"
head_sha="$(printf '%s\n' "$pr_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["headRefOid"])')"

git fetch --quiet origin "$base_sha"
git fetch --quiet origin "pull/$PR/head:refs/remotes/origin/pr/$PR/head"

git checkout --quiet "$head_sha"
set +e
DECAY_DB_PATH="$DB_PATH" decay diff "$base_sha" > "$RESULT_DIR/diff.stdout" 2> "$RESULT_DIR/diff.stderr"
diff_exit=$?
set -e

git diff --name-status "$base_sha" "$head_sha" > "$RESULT_DIR/files.txt"
git diff --stat "$base_sha" "$head_sha" > "$RESULT_DIR/stat.txt"

cat > "$RESULT_DIR/run.toml" <<EOF
case_id = "$CASE_ID"
repo = "$REPO"
pr = $PR
kind = "$KIND"
base_sha = "$base_sha"
head_sha = "$head_sha"
diff_exit = $diff_exit
EOF

popd >/dev/null

echo "wrote $RESULT_DIR"
exit 0
