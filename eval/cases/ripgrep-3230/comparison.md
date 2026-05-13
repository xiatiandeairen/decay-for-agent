# Comparison

Fill this after both `ground-truth.md` and `decay-evidence.md` exist.

## Metrics

```toml
judgement_status = "complete"
top3_total = 0
top3_useful = 0
actionable = false
negative_control = true
negative_silent = true
missed_high_risk_count = 0
useful_evidence_count = 0
plausible_evidence_count = 0
noise_evidence_count = 0
noise_top3_count = 0
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- None.

## Plausible But Unproven

- None.

## Noise

- None after mechanical lint filtering.

## Missed High-Value Risk

- None.

## Reviewer Action Change

- Yes for this negative-control purpose: decay stays quiet on a mechanical clippy lint refactor.

## Root Cause Backlog

- evidence_missing:
- ranking_wrong:
- file_role_wrong:
- history_misleading:
- output_not_actionable:
- ground_truth_insufficient:
