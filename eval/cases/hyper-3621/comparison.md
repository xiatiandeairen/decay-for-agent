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
useful_evidence_count = 3
plausible_evidence_count = 3
noise_evidence_count = 1
noise_top3_count = 0
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- id=4/id=5/id=6 `protocol_header_authority_changed` directly match the Host header authority -> host/port risk in the examples.

## Plausible But Unproven

- id=7 `default_value_changed` catches a real protocol/default boundary, but it is not the stated Host header issue.
- id=2/id=3 blast/mixed concern is plausible given 8 files across examples and protocol code.

## Noise

- id=1 `panic_like_added` does not explain the Host header/userinfo risk and appears generic.

## Missed High-Value Risk

- None for the stated Host header risk.

## Reviewer Action Change

- Yes. The top three findings now explicitly tell the reviewer to validate Host header semantics, credential leakage, and compatibility tests.

## Root Cause Backlog

- evidence_missing:
- ranking_wrong:
- file_role_wrong:
- history_misleading:
- output_not_actionable:
- ground_truth_insufficient:
