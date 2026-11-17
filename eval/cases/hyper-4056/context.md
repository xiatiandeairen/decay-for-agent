# hyper-4056

- repo: `hyperium/hyper`
- pr: `4056`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/4056
- title: fix(http1): fix reading large bodies on 32-bit systems
- merged_at: 2026-04-27T12:31:58Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

HTTP/1 32-bit large-body integer boundary.

## Changed Files

- `src/proto/h1/decode.rs` (+2/-7)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
