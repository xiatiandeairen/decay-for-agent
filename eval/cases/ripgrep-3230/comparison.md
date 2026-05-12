# Comparison

Fill this after both `ground-truth.md` and `decay-evidence.md` exist.

## Metrics

```toml
judgement_status = "complete"
top3_total = 2
top3_useful = 0
actionable = false
negative_control = true
negative_silent = false
missed_high_risk_count = 0
useful_evidence_count = 0
plausible_evidence_count = 0
noise_evidence_count = 2
noise_top3_count = 2
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- None.

## Plausible But Unproven

- None.

## Noise

- id=1 `blast_radius` and id=2 `mixed_concerns` overstate a small mechanical clippy lint change.

## Missed High-Value Risk

- None.

## Reviewer Action Change

- No. Decay should have stayed quiet or explicitly recognized mechanical lint refactor.

## Root Cause Backlog

- evidence_missing: mechanical lint/refactor classifier.
- ranking_wrong:
- file_role_wrong:
- history_misleading:
- output_not_actionable: blast/mixed concern lacks semantic filter.
- ground_truth_insufficient:
