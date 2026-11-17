# hyper-4043

- repo: `hyperium/hyper`
- pr: `4043`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/4043
- title: fix(http1): allow keep-alive for chunked requests with trailers
- merged_at: 2026-03-27T16:10:45Z
- review_decision: APPROVED
- labels: 

## Selection Reason

HTTP/1 keep-alive with chunked trailers.

## Changed Files

- `src/proto/h1/conn.rs` (+2/-1)
- `tests/server.rs` (+52/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
