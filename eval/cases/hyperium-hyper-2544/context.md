# hyperium-hyper-2544

- repo: `hyperium/hyper`
- pr: `2544`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/2544
- title: Return Parse::Internal error if there's an illegal header name or value
- merged_at: 2021-06-04T22:17:38Z
- review_decision: APPROVED
- labels: 

## Selection Reason

Malformed header defensive parse boundary.

## Changed Files

- `src/error.rs` (+5/-0)
- `src/proto/h1/role.rs` (+15/-27)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
