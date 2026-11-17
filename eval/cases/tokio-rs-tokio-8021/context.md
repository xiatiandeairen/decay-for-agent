# tokio-rs-tokio-8021

- repo: `tokio-rs/tokio`
- pr: `8021`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/tokio-rs/tokio/pull/8021
- title: fix(semaphore): Fix semaphore reopens after forget
- merged_at: 2026-04-08T07:59:54Z
- review_decision: APPROVED
- labels: A-tokio, M-sync, R-loom-sync

## Selection Reason

Closed semaphore reopened after forget_permits.

## Changed Files

- `tokio/src/sync/batch_semaphore.rs` (+1/-1)
- `tokio/tests/sync_semaphore.rs` (+54/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
