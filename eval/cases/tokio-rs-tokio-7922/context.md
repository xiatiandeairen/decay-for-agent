# tokio-rs-tokio-7922

- repo: `tokio-rs/tokio`
- pr: `7922`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/tokio-rs/tokio/pull/7922
- title: runtime: fix race condition during the blocking pool shutdown
- merged_at: 2026-02-23T21:50:28Z
- review_decision: APPROVED
- labels: A-tokio, M-runtime, R-loom-blocking

## Selection Reason

Blocking pool shutdown race.

## Changed Files

- `.github/labeler.yml` (+3/-0)
- `.github/workflows/loom.yml` (+16/-0)
- `tokio/src/runtime/blocking/pool.rs` (+18/-8)
- `tokio/src/runtime/tests/loom_blocking.rs` (+39/-1)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
