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
missed_high_risk_count = 1
useful_evidence_count = 3
plausible_evidence_count = 3
noise_evidence_count = 0
noise_top3_count = 0
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- id=1 `handshake` worsened points at client connection logic affected by the keep-alive behavior.
- id=3/id=4 `role.rs:parse` worsened points at HTTP/1 role/parser logic central to the change.

## Plausible But Unproven

- id=2 touched existing `proto/h1/io.rs:parse` is plausible supporting context.
- id=5/id=6 blast/mixed concern is plausible because client/server/proto code are all touched.

## Noise

- None obvious.

## Missed High-Value Risk

- Missed: no explicit evidence for public config API addition or HTTP/1.0 keep_alive/security advisory semantics.

## Reviewer Action Change

- Yes. Decay points reviewer to core connection/parser functions, but does not explain the protocol mechanism.

## Root Cause Backlog

- evidence_missing: protocol flag/API semantic detector; issue/advisory metadata ingestion.
- ranking_wrong:
- file_role_wrong:
- history_misleading:
- output_not_actionable: complexity evidence should mention protocol role when path indicates HTTP.
- ground_truth_insufficient:
