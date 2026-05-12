# Execution plan

## Target

Build a corpus of 50 accepted high-value diffs. Preserve enough context that any
decay miss/noise/overload can become a regression fixture or evidence backlog
item.

## Phase 1: candidate mining

Mine candidates without looking at decay output.

Sources:

- Local mxcore history.
- Open-source GitHub PRs and issues.
- Follow-up fix clusters after important changes.
- Reverts and hotfixes.
- Review comments that request behavior/API/security/test changes.
- High-churn files only when combined with another objective value signal.

Output:

- `candidates/mxcore.toml`
- `candidates/open-source.toml`

## Phase 2: gate review

For each candidate, record:

- objective value signal,
- concrete risk prompt point,
- reviewer action before merge,
- available context,
- decision: accepted / filtered,
- reason.

Do not count medium-confidence cases as high-value validation.

## Phase 3: freeze accepted cases

For each accepted case:

- save `metadata.toml`,
- save `context.md`,
- save `diff.patch`,
- save raw PR/issue/review/follow-up/history artifacts under `raw/`,
- save `decay.stdout` only after context is frozen.

## Phase 4: independent analysis

For each frozen case:

- Ground truth analysis reads metadata/context/diff/raw, not decay output.
- Decay evidence analysis reads decay output, not ground truth.
- Comparison happens only after both are complete.

## Phase 5: bad-case suite

Create one bad case for every:

- missed high/medium ground truth risk,
- low-value or generic true positive,
- false positive,
- evidence overload,
- candidate-selection failure.

Each bad case records:

- expected risk,
- actual decay output,
- failure type,
- required new evidence source,
- minimal regression fixture shape,
- confidence.

## Phase 6: reporting

The final report must answer:

- How many naturally sampled high-value diffs were accepted?
- What evidence was useful to reviewers?
- Which high-value risks were missed?
- Which misses repeat across repos?
- What new evidence sources should be prioritized?

