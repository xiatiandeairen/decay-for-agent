# Comparison

Fill this after both `ground-truth.md` and `decay-evidence.md` exist.

## Metrics

```toml
judgement_status = "complete"
top3_total = 2
top3_useful = 0
actionable = false
negative_control = false
negative_silent = false
missed_high_risk_count = 0
useful_evidence_count = 0
plausible_evidence_count = 1
noise_evidence_count = 1
noise_top3_count = 1
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- None.

## Plausible But Unproven

- id=2 `mixed_concerns` is plausible because the refactor spans body modules and HTTP/1 dispatch, but it is broad.

## Noise

- id=1 `panic_like_added` appears to be moved/refactored code rather than a newly introduced runtime failure.

## Missed High-Value Risk

- None high severity. The better signal would be move/split preservation evidence rather than panic-like evidence.

## Reviewer Action Change

- No clear action change from decay output.

## Root Cause Backlog

- evidence_missing: move-aware refactor preservation evidence.
- ranking_wrong: moved panic-like line ranks too high.
- file_role_wrong:
- history_misleading:
- output_not_actionable: mixed_concerns is too generic for this refactor.
- ground_truth_insufficient:
