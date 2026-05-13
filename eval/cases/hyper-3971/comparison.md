# Comparison

Fill this after both `ground-truth.md` and `decay-evidence.md` exist.

## Metrics

```toml
judgement_status = "complete"
top3_total = 3
top3_useful = 1
actionable = true
negative_control = false
negative_silent = false
missed_high_risk_count = 1
useful_evidence_count = 1
plausible_evidence_count = 4
noise_evidence_count = 3
noise_top3_count = 1
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- id=1 `encode_headers_with_original_case` touched existing risky function maps to header casing serialization behavior.

## Plausible But Unproven

- id=2/id=3 `parse` touched existing risky functions are plausible protocol parsing context.
- id=6 `mixed_concerns` is plausible because API extension and HTTP/1 role code are both touched.

## Noise

- id=4/id=5 panic-like additions do not clearly explain the public API design risk from the captured context.
- id=7/id=8 history signals are supporting context only and should not count as useful alone.

## Missed High-Value Risk

- Missed: no explicit public API design/change evidence for `src/ext/mod.rs`, despite PR title and review comments focusing on API design.

## Reviewer Action Change

- Partial. Decay points to header encode/parse code, but misses the primary API design concern.

## Root Cause Backlog

- evidence_missing: public type/API addition detector; header validation semantic detector.
- ranking_wrong: panic-like aggregation ranks over API/protocol design.
- file_role_wrong:
- history_misleading: history signals can look important without direct evidence.
- output_not_actionable: top finding should name header casing/public API.
- ground_truth_insufficient:
