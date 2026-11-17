# tokio-rs-tokio-7991

- repo: `tokio-rs/tokio`
- pr: `7991`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/tokio-rs/tokio/pull/7991
- title: sync: fix panic in `Chan::recv_many` when called with non-empty vector on closed channel
- merged_at: 2026-04-02T11:35:26Z
- review_decision: APPROVED
- labels: A-tokio, M-sync, R-loom-sync

## Selection Reason

Closed-channel recv_many panic boundary.

## Changed Files

- `tokio/src/sync/mpsc/chan.rs` (+5/-9)
- `tokio/tests/sync_mpsc.rs` (+24/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
