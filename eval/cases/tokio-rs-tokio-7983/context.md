# tokio-rs-tokio-7983

- repo: `tokio-rs/tokio`
- pr: `7983`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/tokio-rs/tokio/pull/7983
- title: rt: do not leak fd when cancelling io_uring open operation
- merged_at: 2026-04-08T05:37:19Z
- review_decision: APPROVED
- labels: C-bug, A-tokio, M-fs, T-io-uring

## Selection Reason

io_uring open cancellation leaked file descriptors.

## Changed Files

- `tokio/src/runtime/io/driver/uring.rs` (+31/-4)
- `tokio/tests/fs_uring_cancel_open.rs` (+71/-0)
- `tokio/tests/fs_uring_completed_then_dropped_before_repoll.rs` (+130/-0)
- `tokio/tests/fs_uring_runtime_shutdown.rs` (+78/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
