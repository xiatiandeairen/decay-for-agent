# hyperium-hyper-3952

- repo: `hyperium/hyper`
- pr: `3952`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3952
- title: fix(http1): poll_loop writes when ready
- merged_at: 2025-11-10T14:51:16Z
- review_decision: APPROVED
- labels: 

## Selection Reason

HTTP/1 poll loop readiness stall.

## Changed Files

- `Cargo.toml` (+6/-0)
- `src/proto/h1/dispatch.rs` (+15/-2)
- `tests/ready_stream.rs` (+249/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
