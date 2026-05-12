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
useful_evidence_count = 8
plausible_evidence_count = 1
noise_evidence_count = 0
noise_top3_count = 0
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- id=2..id=9 semantic `default_value_changed` exactly capture `--` added to decompression command argument arrays.

## Plausible But Unproven

- id=1 `test_gap` is plausible because this production bug fix needs regression coverage, but the captured PR context does not prove tests are absent or required.

## Noise

- None, though repeated semantic findings should eventually be grouped.

## Missed High-Value Risk

- None obvious.

## Reviewer Action Change

- Yes. Decay points directly to command argument arrays and would prompt checking every decompression tool.

## Root Cause Backlog

- evidence_missing:
- ranking_wrong:
- file_role_wrong:
- history_misleading:
- output_not_actionable: group repeated command-array changes into one finding.
- ground_truth_insufficient:
