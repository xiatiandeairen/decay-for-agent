# tokio-rs-tokio-8001

- repo: `tokio-rs/tokio`
- pr: `8001`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/tokio-rs/tokio/pull/8001
- title: Surface errors from `SO_ERROR` on `recv` for UDP sockets on Linux
- merged_at: 2026-04-04T19:46:10Z
- review_decision: APPROVED
- labels: A-tokio, M-net

## Selection Reason

UDP error-readiness propagation.

## Changed Files

- `tokio/src/net/udp.rs` (+22/-10)
- `tokio/tests/udp.rs` (+33/-1)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
