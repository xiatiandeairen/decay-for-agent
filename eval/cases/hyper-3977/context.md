# hyper-3977

- repo: `hyperium/hyper`
- pr: `3977`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3977
- title: Revert "fix(http1): fix rare missed write wakeup on connections (#3952)"
- merged_at: 2025-11-13T21:11:36Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

Revert of rare missed write wakeup fix.

## Changed Files

- `Cargo.toml` (+0/-5)
- `src/proto/h1/dispatch.rs` (+2/-15)
- `tests/ready_stream.rs` (+0/-234)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
