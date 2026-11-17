# hyper-3185

- repo: `hyperium/hyper`
- pr: `3185`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3185
- title: fix(server): start header read timeout immediately
- merged_at: 2024-06-03T19:18:14Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

HTTP/1 header read timeout lifecycle.

## Changed Files

- `src/proto/h1/conn.rs` (+49/-13)
- `src/proto/h1/io.rs` (+1/-37)
- `src/proto/h1/mod.rs` (+0/-15)
- `src/proto/h1/role.rs` (+0/-94)
- `tests/server.rs` (+21/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
