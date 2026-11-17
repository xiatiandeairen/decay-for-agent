# tokio-rs-tokio-8076

- repo: `tokio-rs/tokio`
- pr: `8076`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/tokio-rs/tokio/pull/8076
- title: Require that an `RwLock` has `max_readers != 0`
- merged_at: 2026-05-07T07:31:12Z
- review_decision: APPROVED
- labels: A-tokio, M-sync, R-loom-sync

## Selection Reason

RwLock zero max_readers mutual exclusion violation.

## Changed Files

- `tokio/src/sync/rwlock.rs` (+11/-1)
- `tokio/tests/sync_rwlock.rs` (+12/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
