# hyper-4011

- repo: `hyperium/hyper`
- pr: `4011`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/4011
- title: fix(http1): use case-insensitive matching for trailer fields
- merged_at: 2026-02-06T17:42:00Z
- review_decision: APPROVED
- labels: 

## Selection Reason

HTTP/1 trailer field case-insensitive contract.

## Changed Files

- `src/proto/h1/encode.rs` (+42/-30)
- `src/proto/h1/role.rs` (+27/-9)
- `tests/client.rs` (+74/-0)
- `tests/server.rs` (+45/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
