# head-to-head v1

This directory holds the AI head-to-head evaluation asset that decay uses to
prove (or disprove) that its semantic layer produces review-worthy risk
evidence that a plain "AI reads the PR diff" baseline cannot.

The asset is **frozen**: the model, the prompt, and the output schema are
pinned by sha256 in `manifests/v1.toml`. Once committed, do not edit the
manifest, prompt, or schema. To change anything, open a v2 manifest as
described under "Freeze and how to extend" below.

## Layout

```
eval/head-to-head/
  prompts/ai-baseline-v1.md           AI baseline prompt (frozen, sha-pinned)
  schemas/ai-baseline-output.schema.json  output schema (frozen, sha-pinned)
  manifests/v1.toml                   freeze contract; pins model + sha + thresholds
  runs/<run-id>/<case-id>/            raw + normalized output per case
  runs/<run-id>/summary.jsonl         per-case status summary
  judgements/<case-id>.toml           human judgement (`[ai_baseline]` section)
```

Related scripts under `eval/scripts/`:

- `run-ai-baseline.py` — gate → claude CLI → normalize → write run files
- `normalize-ai-output.py` — strip fences, repair common JSON truncations,
  enforce schema rules, verify evidence_refs against the diff
- `validate-head-to-head-manifest.py` — hash gate; refuses to run if the
  on-disk prompt or schema does not match the manifest sha
- `summarize-head-to-head.py` — three-way report:
  fact-only / semantic-filtered / ai-baseline

## Fairness contract

The AI baseline must see the same surface area as decay does. Inputs allowed
into the per-case bundle are listed in `manifests/v1.toml [fairness]`:

| allowed | denied |
|---|---|
| `diff.patch` | `pr.json`, `pr.title`, `pr.body`, `pr.discussion` |
| `changed_files.txt` (paths only, git-status prefix stripped) | `linked_issue`, `ci_logs`, `follow_up_commits`, `repository_source_outside_diff` |

The runner reads only the allowed files; any other file in the case directory
is silently ignored. The prompt explicitly forbids the model from assuming PR
metadata that isn't in the bundle. The runner-side fairness contract is
covered by `eval/scripts/head_to_head_schema_test.py`.

## "Clearly outperforms" thresholds

The decay output is considered to clearly outperform the AI baseline on the
frozen-v1 cohort when at least 3 of these 4 conditions hold (configured in
`manifests/v1.toml [thresholds]`):

| metric | rule |
|---|---|
| precision_at_top3 | decay - ai_baseline >= 10pp |
| actionable_pr_rate | decay - ai_baseline >= 10pp |
| missed_high_risk_count | ai_baseline - decay >= 5 cases |
| weak_only_top_risk_count | ai_baseline - decay >= 5 cases |

These thresholds are part of the freeze. Do not lower them to make a run
pass. If a run does not clear them, the conclusion is "did not clearly
outperform"; the right action is to improve decay, not the threshold.

## Running

```bash
# Smoke a single case
python3 eval/scripts/run-ai-baseline.py --case <case-id>

# Multi-case smoke covering several risk signals
python3 eval/scripts/run-ai-baseline.py \
    --case serde-rs-json-757 \
    --case hyperium-hyper-3616 \
    --case tokio-rs-tokio-8021 \
    --case hyper-3621 \
    --case tokio-rs-tokio-7922

# Full corpus (40 frozen-v1 cases)
python3 eval/scripts/run-ai-baseline.py --all

# Build the three-way comparison report from existing judgements
python3 eval/scripts/summarize-head-to-head.py --out eval/reports/head-to-head-v1.md
```

The runner first verifies the manifest hash gate; if the on-disk prompt or
schema does not match the sha pinned in `manifests/v1.toml`, the runner
exits non-zero before any claude CLI calls are made.

## Judgement workflow (for round 2)

This round wrote the harness and the smoke run; the AI baseline output is
captured but not yet graded. To compute real metrics:

1. For each case in `eval/risk-signals/frozen-v1.toml`, create
   `eval/head-to-head/judgements/<case-id>.toml` mirroring the
   `[fact_only]` / `[semantic_filtered]` field set:

   ```toml
   version = "ai-baseline-judgement.v1"
   case_id = "..."
   status = "complete"
   judge = "..."
   judged_at = "..."

   [ai_baseline]
   top3_total = 0
   top3_useful = 0
   actionable = false
   missed_high_risk_count = 0
   weak_only_top_risk_count = 0
   notes = ""
   ```

2. Re-run `summarize-head-to-head.py`; the report regenerates with real
   metric deltas instead of the 0-data skeleton.

## Smoke results (round 1)

The first smoke run hand-picked 5 cases covering all 8 risk signals except
`lifecycle_ordering` (the model surfaced it under `state_transition` for
`hyperium-hyper-3616`, which is a real grading question rather than a
runner failure):

| case | risk signals surfaced |
|---|---|
| `serde-rs-json-757` | assumption_boundary, error_propagation, test_gap_for_risk_path |
| `hyperium-hyper-3616` | state_transition, assumption_boundary, test_gap_for_risk_path |
| `tokio-rs-tokio-8021` | state_transition, concurrency_ordering, test_gap_for_risk_path |
| `hyper-3621` | external_contract, assumption_boundary, assumption_boundary |
| `tokio-rs-tokio-7922` | state_transition, concurrency_ordering, non_happy_path |

5/5 normalized successfully on the second attempt; the first attempt exposed
three normalizer gaps (truncated root `}`, mixed `+`/`-` prefixes in
evidence_refs, off-by-one indentation drift) which have unit-test coverage
and are now handled. The skeleton report is at
`eval/reports/head-to-head-v1.md`.

## Freeze and how to extend

The freeze invariant: a v1 manifest is good for as long as the v1 prompt and
v1 schema files keep their sha256 on disk. Any byte-level change to either
file invalidates the manifest hash gate and the runner refuses to run.

To change the model, prompt, or schema:

1. Copy `prompts/ai-baseline-v1.md` to `prompts/ai-baseline-v2.md`, edit.
2. Copy `schemas/ai-baseline-output.schema.json` to `...v2.schema.json`,
   edit (or reuse v1 if only the prompt changed).
3. Compute fresh sha256:
   `shasum -a 256 eval/head-to-head/prompts/ai-baseline-v2.md`
4. Copy `manifests/v1.toml` to `manifests/v2.toml`, update `manifest_id`,
   `model`, `freeze.*_path`, `freeze.*_sha256`, `frozen_by`, `frozen_at`.
5. Run new manifest through `validate-head-to-head-manifest.py` to confirm
   it passes the gate.
6. Run runner with `--manifest eval/head-to-head/manifests/v2.toml`.

Different manifests yield distinct three-way comparison reports. Do not
compare numbers across manifests as if they were a continuous time series.
