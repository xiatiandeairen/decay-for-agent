# hyperium-hyper-4069

- repo: `hyperium/hyper`
- pr: `4069`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/4069
- title: fix(http1): send error when dispatcher is dropped mid-body
- merged_at: 2026-05-12T18:56:30Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

HTTP/1 dispatcher drop mid-body silent truncation.

## Changed Files

- `src/proto/h1/dispatch.rs` (+41/-8)
- `tests/client.rs` (+55/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
