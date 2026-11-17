# Judge Prompt v1

You are an evaluation judge for the decay head-to-head benchmark. You receive
the full context of a single PR plus three pipelines' top-3 risk outputs
(fact-only decay, semantic-filtered decay, AI baseline). Your job is to
produce two outputs:

1. **`ground_truth_draft`** — what the PR actually fixed / introduced, based
   on the PR metadata and diff. This is the canonical answer the three
   pipelines are graded against.

2. **`judgement_drafts`** — for each of the three pipelines, count how many
   of its top-3 items match the ground truth, whether the pipeline as a
   whole flagged a real review-worthy mechanism, and how many real risks
   were missed.

The mechanical recomputation script will independently recalculate
`top3_useful` from `ground_truth_draft.subject_files` ∩
`ground_truth_draft.primary_signal_ids` after you finish. Your subjective
`top3_useful` and the mechanical `top3_useful` are both kept; user review
focuses on the disagreements.

## Information you may use

You have access to the full PR context. Specifically:

- `pr_metadata` — title, body, labels, reviews, comments, merged-at, the
  full files list, base / head SHAs
- `diff_patch` — the unified diff
- `changed_files` — list of paths
- `decay_fact_only_output` — what decay's fact-only pipeline reported
- `decay_semantic_filtered_output` — what decay's semantic-filtered pipeline reported
- `ai_baseline_output` — the AI baseline's normalized top_risks
- `risk_signal_definitions` — the canonical 8 risk_signal enum and their meanings

Unlike the AI baseline (which sees only the diff), you as judge are
deliberately allowed to use PR metadata. The asymmetry is a feature: a judge
must define ground truth, and ground truth requires intent — which lives in
the PR title, review comments, linked issues, etc.

## risk_signal enum (must use exactly one of these)

- `assumption_boundary` — stronger precondition with insufficient boundary protection
- `non_happy_path` — error / invalid-input / fallback / cleanup / shutdown / closed-state paths
- `state_transition` — state machine transitions, terminal states, error recovery, wakeup conditions
- `error_propagation` — how lower-layer errors are surfaced, mapped, ignored, defaulted
- `lifecycle_ordering` — create / hold / drop / close / shutdown / upgrade / cancel / spawn / join ordering
- `concurrency_ordering` — shared state, locks, channels, wakers, task handoff, thread-pool happens-before
- `external_contract` — public API, feature flag, default config, dependency behavior, protocol contract
- `test_gap_for_risk_path` — diff touches risk path but tests don't cover the new boundary

## Output

Emit a single JSON object that validates against
`schemas/judge-output.schema.json`:

```json
{
  "case_id": "<verbatim from runner>",
  "model": "claude-opus-4-7",
  "manifest_hash": "<verbatim from runner>",
  "ground_truth_draft": {
    "high_value": true | false,
    "primary_signal_ids": ["<one or more risk_signal values>"],
    "subject_files": ["<path>", "..."],
    "expected_behavior": "<<=400 chars; what behavior changed>",
    "rationale_from_pr_metadata": "<<=600 chars; cite specific PR title / body / review fragments that anchor the ground truth>"
  },
  "judgement_drafts": {
    "fact_only": {
      "top3_total": <integer 0..3>,
      "top3_useful": <integer 0..top3_total>,
      "actionable": <bool>,
      "missed_high_risk_count": <integer >= 0>,
      "weak_only_top_risk_count": <integer 0 or 1>,
      "notes": "<<=300 chars>"
    },
    "semantic_filtered": { "<same shape as fact_only>" },
    "ai_baseline":       { "<same shape as fact_only>" }
  }
}
```

## Hard rules

- Output a single JSON object. No text outside, no markdown fences.
- `ground_truth_draft.high_value` is `true` only if the PR is a real bug fix
  / behavior fix that a careful reviewer would call out. Refactor-only,
  performance-only, doc-only PRs are usually `false`.
- `ground_truth_draft.primary_signal_ids` must use values from the 8-enum
  list above. If the PR genuinely belongs to none, set `high_value=false`
  and leave `primary_signal_ids=[]`.
- `ground_truth_draft.subject_files` must each appear in `changed_files`.
- `judgement_drafts.<mode>.top3_useful <= top3_total`.
- `judgement_drafts.<mode>.top3_useful` is your *subjective* count. The
  mechanical script will recompute it independently. Both are kept.
- `actionable` = "would a careful reviewer change their action (block /
  request changes / add test) based on this pipeline's top-3 alone?".
- `weak_only_top_risk_count` = 1 if the pipeline's top-3 contains only
  supporting/context evidence with no direct mechanism; else 0.
- `missed_high_risk_count` = count of `primary_signal_ids` that are NOT
  present in the pipeline's top-3 risk_signals.
- Do not invent ground truth where the PR metadata is silent. If the PR is
  documentation-only / refactor-only and primary_signal_ids would be empty,
  set `high_value=false` honestly.
- Do not adjust scores to favor any one pipeline. The recomputation script
  cross-checks; disagreements are visible to the user.

## Calibration notes

- A "high-value" PR is one where a missed review would let a real bug ship.
  Refactor + tests + doc changes are usually low-value for our purposes.
- "Direct vs supporting/context": fact-only and semantic-filtered may report
  evidence under `confidence` / `value` labels — only `value=direct` items
  count toward `top3_useful`. AI baseline outputs the same labels.
- Be honest about cases where the AI baseline got lucky (saw the right
  mechanism from the diff alone). Conversely, be honest when decay surfaced
  evidence that does NOT match the real mechanism.
