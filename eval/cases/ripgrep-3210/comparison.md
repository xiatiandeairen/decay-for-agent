# Comparison

Fill this after both `ground-truth.md` and `decay-evidence.md` exist.

## Metrics

```toml
judgement_status = "complete"
top3_total = 3
top3_useful = 3
actionable = true
negative_control = false
negative_silent = false
missed_high_risk_count = 0
useful_evidence_count = 5
plausible_evidence_count = 0
noise_evidence_count = 0
noise_top3_count = 0
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- id=1 `from_name` new high-risk maps to POSIX class parsing.
- id=2 `parse_class` worsened maps to bracket class parsing.
- id=3 `parse_posix_class` new high-risk maps directly to the new feature.
- id=4 `to_char_set` new high-risk maps to regex/class conversion.
- id=5 `tokens_to_regex` worsened maps to glob-to-regex behavior.

## Plausible But Unproven

- None.

## Noise

- None.

## Missed High-Value Risk

- None obvious from captured context.

## Reviewer Action Change

- Yes. Decay points directly to the parser/conversion functions that should receive review and regression tests.

## Root Cause Backlog

- evidence_missing:
- ranking_wrong:
- file_role_wrong:
- history_misleading:
- output_not_actionable:
- ground_truth_insufficient:
