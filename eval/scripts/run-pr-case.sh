#!/usr/bin/env bash
set -euo pipefail

EVAL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS="${DECAY_EVAL_CORPUS:-$EVAL_ROOT/corpus/pr-cases.toml}"
CASE_ID="${1:-}"

if [[ -z "$CASE_ID" ]]; then
  echo "usage: eval/scripts/run-pr-case.sh <case-id>" >&2
  exit 2
fi

command -v gh >/dev/null || { echo "gh is required" >&2; exit 2; }
command -v git >/dev/null || { echo "git is required" >&2; exit 2; }
command -v decay >/dev/null || { echo "decay is required; run cargo install --path ." >&2; exit 2; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 2; }

case_json="$(python3 "$EVAL_ROOT/scripts/toml_case.py" "$CORPUS" "$CASE_ID")"
if [[ -z "$case_json" ]]; then
  echo "case not found: $CASE_ID" >&2
  exit 2
fi

value() {
  local key="$1"
  printf '%s\n' "$case_json" | python3 -c 'import json,sys; print(json.load(sys.stdin).get(sys.argv[1], ""))' "$key"
}

REPO="$(value repo)"
PR="$(value pr)"
KIND="$(value kind)"
STATUS="$(value status)"

if [[ -z "$REPO" || -z "$PR" || -z "$KIND" ]]; then
  echo "invalid case metadata for $CASE_ID" >&2
  exit 2
fi

WORK_ROOT="${DECAY_EVAL_WORK_ROOT:-/private/tmp/decay-pr-eval}"
RUN_ROOT="$WORK_ROOT/$CASE_ID"
REPO_DIR="$RUN_ROOT/repo"
CASE_DIR="$EVAL_ROOT/cases/$CASE_ID"
RAW_DIR="$CASE_DIR/raw"
DB_PATH="$RAW_DIR/decay.db"

mkdir -p "$RUN_ROOT" "$RAW_DIR"

if [[ ! -d "$REPO_DIR/.git" ]]; then
  gh repo clone "$REPO" "$REPO_DIR" -- --quiet
fi

pushd "$REPO_DIR" >/dev/null
git fetch --quiet origin

pr_json="$(gh pr view "$PR" --repo "$REPO" --json number,title,url,state,baseRefName,baseRefOid,headRefName,headRefOid,labels,files,reviewDecision,comments,reviews,mergedAt,body)"
printf '%s\n' "$pr_json" > "$RAW_DIR/pr.json"

base_sha="$(printf '%s\n' "$pr_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["baseRefOid"])')"
head_sha="$(printf '%s\n' "$pr_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["headRefOid"])')"

if [[ -z "$base_sha" || -z "$head_sha" || "$base_sha" == "$head_sha" ]]; then
  echo "invalid base/head for $CASE_ID: base=$base_sha head=$head_sha" >&2
  exit 3
fi

git fetch --quiet origin "$base_sha"
git fetch --quiet origin "pull/$PR/head:refs/remotes/origin/pr/$PR/head"
git checkout --quiet "$head_sha"

git diff --name-status "$base_sha" "$head_sha" > "$RAW_DIR/files.txt"
git diff --stat "$base_sha" "$head_sha" > "$RAW_DIR/stat.txt"
git diff --find-renames --unified=80 "$base_sha" "$head_sha" > "$RAW_DIR/diff.patch"

if [[ ! -s "$RAW_DIR/files.txt" || ! -s "$RAW_DIR/diff.patch" ]]; then
  echo "empty diff for $CASE_ID" >&2
  exit 3
fi

set +e
DECAY_DB_PATH="$DB_PATH" decay diff "$base_sha" > "$RAW_DIR/decay.stdout" 2> "$RAW_DIR/decay.stderr"
diff_exit=$?
set -e

if ! grep -q "mode=git_ref" "$RAW_DIR/decay.stdout"; then
  echo "decay did not use git_ref mode for $CASE_ID" >&2
  exit 4
fi

for marker in "Diff context:" "Top risk findings:" "Complexity impact:" "Coverage impact:" "Stability impact:" "Quality impact:" "Semantic impact:" "History impact:"; do
  if ! grep -q "$marker" "$RAW_DIR/decay.stdout"; then
    echo "missing decay output marker '$marker' for $CASE_ID" >&2
    exit 4
  fi
done

python3 "$EVAL_ROOT/scripts/validate-decay-evidence.py" "$RAW_DIR/decay.stdout" >/dev/null

cat > "$CASE_DIR/metadata.toml" <<EOF
case_id = "$CASE_ID"
repo = "$REPO"
pr = $PR
kind = "$KIND"
status = "$STATUS"
base_sha = "$base_sha"
head_sha = "$head_sha"
diff_exit = $diff_exit
EOF

if [[ ! -f "$CASE_DIR/context.md" ]]; then
  python3 "$EVAL_ROOT/scripts/write-context.py" "$CASE_DIR" "$case_json" "$RAW_DIR/pr.json"
fi
if [[ ! -f "$CASE_DIR/ground-truth.md" ]]; then
  cp "$EVAL_ROOT/templates/ground-truth.md" "$CASE_DIR/ground-truth.md"
fi
python3 "$EVAL_ROOT/scripts/write-decay-evidence.py" "$RAW_DIR/decay.stdout" "$CASE_DIR/decay-evidence.md"
python3 "$EVAL_ROOT/scripts/build-semantic-prompt.py" "$RAW_DIR/decay.stdout" "$RAW_DIR/semantic-prompt.md"
if [[ ! -f "$CASE_DIR/comparison.md" ]]; then
  cp "$EVAL_ROOT/templates/comparison.md" "$CASE_DIR/comparison.md"
fi

popd >/dev/null

echo "wrote $CASE_DIR"
exit 0
