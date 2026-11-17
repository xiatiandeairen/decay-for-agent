# hyper-3988

- repo: `hyperium/hyper`
- pr: `3988`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3988
- title: fix(http1): fix rare missed write wakeup on connections v2
- merged_at: 2026-04-22T15:40:30Z
- review_decision: APPROVED
- labels: 

## Selection Reason

HTTP/1 write/flush readiness coordination v2.

## Changed Files

- `Cargo.toml` (+12/-0)
- `src/proto/h1/dispatch.rs` (+34/-6)
- `tests/h1_server/fixture.rs` (+136/-0)
- `tests/h1_server/mod.rs` (+97/-0)
- `tests/ready_on_poll_stream.rs` (+139/-0)
- `tests/unbuffered_stream.rs` (+126/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
