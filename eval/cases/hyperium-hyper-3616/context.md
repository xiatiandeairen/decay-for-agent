# hyperium-hyper-3616

- repo: `hyperium/hyper`
- pr: `3616`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3616
- title: fix: avoid panic when calling `graceful_shutdown` if H1 connection was upgraded
- merged_at: 2024-03-31T15:30:31Z
- review_decision: APPROVED
- labels: 

## Selection Reason

Graceful shutdown after upgrade panic.

## Changed Files

- `src/server/conn/http1.rs` (+5/-1)
- `tests/server.rs` (+61/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
