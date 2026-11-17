# 40-Case Frozen Eval v1

## Purpose

`frozen-v1.toml` is the product-effect validation set for M3/v0.2.0.
It is stricter than the M3.1 pilot set:

- pilot: supports semantic-flow development and benchmark smoke;
- frozen v1: supports product-effect comparison between fact-only and semantic-filtered output.

The v1 set must not be edited to improve tool scores. Once a case enters v1,
it can only be removed if the source record becomes unavailable or the
ground-truth record is proven wrong.

## Current State

- target: 40 cases
- current seed: 40 cases
- status: `complete`
- gap: 0 cases

The 40 current cases are high-confidence risk cases promoted from the
M3.1 review and frozen-fill review process. They are executable through:

```bash
DECAY_EVAL_CORPUS=eval/corpus/risk-signal-frozen-v1.toml eval/scripts/run-semantic-case.sh <case-id>
```

## Admission Gate

New cases must pass the same gate as the pilot:

1. candidate is selected before looking at decay output;
2. PR, issue, review, test, and follow-up context is stored;
3. ground-truth review says the risk is independently reproducible;
4. risk-signal review says the case maps to current M3 signal scope;
5. skeptic review has no unresolved blocker;
6. the case is represented in `eval-candidates.toml`;
7. `frozen-v1.toml` and `risk-signal-frozen-v1.toml` are updated together.

## Rejected For Current v1

The following case shapes are intentionally excluded from v1 until their own
signal track exists:

- platform performance regressions without a performance/resource signal;
- macro compile-time hygiene;
- build-only compatibility without executable build-matrix evidence;
- revert-only cases when the original risky diff is not the eval target;
- duplicate aliases of already frozen PRs;
- rollup PRs that mix unrelated risk sources into one judgement;
- disputed fixes where follow-up evidence shows the chosen direction was
  reverted or unsafe.

## Product Metrics

The v1 comparison must report:

- `precision_at_top3`
- `actionable_pr_rate`
- `negative_control_silence_rate`
- `missed_high_risk_count`
- `weak_only_top_risk`

The 40-case set is complete as a product-effect validation set. It still does
not prove v0.2.0 product value until fact-only versus semantic-filtered output
is run and misses/noise are reported honestly.
