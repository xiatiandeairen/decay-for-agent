# hyperium-hyper-3812

- repo: `hyperium/hyper`
- pr: `3812`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3812
- title: fix(http1): fix intermitent panic parsing partial headers
- merged_at: 2024-12-16T15:36:41Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

Partial HTTP/1 header parsing panic.

## Changed Files

- `src/proto/h1/io.rs` (+4/-0)
- `tests/client.rs` (+57/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
