# hyper-4061

- repo: `hyperium/hyper`
- pr: `4061`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/4061
- title: fix(http2): do not reserve capacity before body data is available
- merged_at: 2026-05-04T20:49:24Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

HTTP/2 flow-control capacity reservation deadlock.

## Changed Files

- `src/proto/h2/mod.rs` (+70/-24)
- `tests/client.rs` (+167/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
