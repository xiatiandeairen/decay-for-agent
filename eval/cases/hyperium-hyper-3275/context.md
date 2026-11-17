# hyperium-hyper-3275

- repo: `hyperium/hyper`
- pr: `3275`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3275
- title: fix(client): early server response shouldn't propagate NO_ERROR
- merged_at: 2023-07-26T19:47:07Z
- review_decision: APPROVED
- labels: 

## Selection Reason

HTTP/2 early response error propagation.

## Changed Files

- `src/body/incoming.rs` (+10/-1)
- `src/proto/mod.rs` (+1/-1)
- `tests/client.rs` (+57/-1)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
