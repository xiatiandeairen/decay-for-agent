# hyper-3936

- repo: `hyperium/hyper`
- pr: `3936`
- kind: `bugfix`
- status: `candidate`
- url: https://github.com/hyperium/hyper/pull/3936
- title: fix(http1): add http10_disable_keep_alive() for server and client
- merged_at: None
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

HTTP/1 keep-alive behavior fix; should test protocol-sensitive evidence.

## Changed Files

- `src/client/client.rs` (+15/-0)
- `src/client/conn.rs` (+18/-0)
- `src/client/conn/http1.rs` (+3/-0)
- `src/proto/h1/conn.rs` (+14/-1)
- `src/proto/h1/io.rs` (+2/-0)
- `src/proto/h1/mod.rs` (+1/-0)
- `src/proto/h1/role.rs` (+34/-4)
- `src/server/conn.rs` (+25/-0)
- `src/server/conn/http1.rs` (+18/-0)
- `src/server/server.rs` (+18/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
