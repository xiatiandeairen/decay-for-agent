# hyper-4042

- repo: `hyperium/hyper`
- pr: `4042`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/4042
- title: fix(http2): cancel pipe_task and send RST_STREAM on response future drop
- merged_at: 2026-03-31T13:40:12Z
- review_decision: APPROVED
- labels: 

## Selection Reason

Dropped HTTP/2 response future cancellation.

## Changed Files

- `src/client/dispatch.rs` (+3/-0)
- `src/proto/h2/client.rs` (+37/-0)
- `src/proto/h2/mod.rs` (+5/-0)
- `tests/client.rs` (+41/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
