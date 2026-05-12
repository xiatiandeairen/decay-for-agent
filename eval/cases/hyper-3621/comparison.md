# Comparison

Fill this after both `ground-truth.md` and `decay-evidence.md` exist.

## Metrics

```toml
judgement_status = "complete"
top3_total = 3
top3_useful = 0
actionable = false
negative_control = false
negative_silent = false
missed_high_risk_count = 1
useful_evidence_count = 0
plausible_evidence_count = 2
noise_evidence_count = 2
noise_top3_count = 1
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- None confidently matches the stated Host header userinfo risk.

## Plausible But Unproven

- id=4 `default_value_changed` catches a real protocol/default boundary, but it is not the stated Host header issue.
- id=2/id=3 blast/mixed concern is plausible given 8 files across examples and protocol code.

## Noise

- id=1 `panic_like_added` does not explain the Host header/userinfo risk and appears generic.
- The quality findings are too broad to drive the key review action by themselves.

## Missed High-Value Risk

- Missed: Host header construction changed from URI authority to host/port to avoid leaking user information.

## Reviewer Action Change

- Decay would not reliably direct reviewer to validate Host header security/compatibility.

## Root Cause Backlog

- evidence_missing: protocol/header semantic diff, URI authority/userinfo detector.
- ranking_wrong: generic panic/default evidence ranks above true Host risk.
- file_role_wrong:
- history_misleading:
- output_not_actionable: no header-specific wording.
- ground_truth_insufficient:
