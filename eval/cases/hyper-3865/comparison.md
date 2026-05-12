# Comparison

Fill this after both `ground-truth.md` and `decay-evidence.md` exist.

## Metrics

```toml
judgement_status = "complete"
top3_total = 3
top3_useful = 0
actionable = true
negative_control = false
negative_silent = false
missed_high_risk_count = 0
useful_evidence_count = 1
plausible_evidence_count = 2
noise_evidence_count = 7
noise_top3_count = 1
history_only_false_positive_count = 0
```

## Matched Useful Evidence

- id=8 `build_or_ci_changed` matches the review concern about build.rs/build-script cost.

## Plausible But Unproven

- id=9/id=10 blast/mixed concern matches the broad feature-gated refactor, but does not name cfg semantics.

## Noise

- id=1..id=7 complexity findings mostly point at existing complex functions touched by cfg rewrites, not proven new risk.

## Missed High-Value Risk

- No high severity miss, but decay lacks explicit cfg/feature-equivalence evidence.

## Reviewer Action Change

- Useful if reviewer sees `build_or_ci_changed`; weak because it is ranked below complexity noise.

## Root Cause Backlog

- evidence_missing: cfg alias / feature-gate equivalence detector.
- ranking_wrong: build script evidence should rank above complexity noise for build-refactor PRs.
- file_role_wrong:
- history_misleading:
- output_not_actionable: complexity findings lack note that only cfg gates changed.
- ground_truth_insufficient:
