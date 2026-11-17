# hyperium-hyper-3261

- repo: `hyperium/hyper`
- pr: `3261`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3261
- title: fix(http1): http1 server graceful shutdown fix
- merged_at: 2023-07-06T19:23:29Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

HTTP/1 graceful shutdown lifecycle semantics.

## Changed Files

- `src/proto/h1/conn.rs` (+7/-0)
- `src/proto/h1/dispatch.rs` (+5/-1)
- `tests/server.rs` (+32/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
