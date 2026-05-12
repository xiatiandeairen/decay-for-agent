# hyper-3865

- repo: `hyperium/hyper`
- pr: `3865`
- kind: `refactor`
- status: `candidate`
- url: https://github.com/hyperium/hyper/pull/3865
- title: refactor: use cfg-aliases for feature flags
- merged_at: None
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

Feature flag refactor; should test build/config classification.

## Changed Files

- `build.rs` (+77/-0)
- `examples/hello-http2.rs` (+4/-4)
- `src/body/incoming.rs` (+52/-85)
- `src/body/length.rs` (+7/-13)
- `src/body/mod.rs` (+3/-9)
- `src/cfg.rs` (+3/-6)
- `src/client/conn/http1.rs` (+4/-4)
- `src/client/conn/http2.rs` (+1/-1)
- `src/client/conn/mod.rs` (+2/-2)
- `src/client/dispatch.rs` (+27/-27)
- `src/client/mod.rs` (+1/-1)
- `src/client/tests.rs` (+2/-2)
- `src/common/date.rs` (+11/-11)
- `src/common/io/mod.rs` (+2/-2)
- `src/common/io/rewind.rs` (+1/-4)
- `src/common/mod.rs` (+5/-8)
- `src/common/time.rs` (+5/-8)
- `src/error.rs` (+97/-172)
- `src/ext/h1_reason_phrase.rs` (+2/-2)
- `src/ext/mod.rs` (+27/-30)
- `src/ffi/client.rs` (+5/-5)
- `src/ffi/error.rs` (+1/-1)
- `src/ffi/http_types.rs` (+4/-4)
- `src/ffi/mod.rs` (+1/-1)
- `src/headers.rs` (+15/-15)
- `src/lib.rs` (+8/-8)
- `src/mock.rs` (+20/-20)
- `src/proto/h1/conn.rs` (+39/-39)
- `src/proto/h1/decode.rs` (+3/-3)
- `src/proto/h1/dispatch.rs` (+2/-2)
- `src/proto/h1/encode.rs` (+8/-8)
- `src/proto/h1/io.rs` (+13/-13)
- `src/proto/h1/mod.rs` (+5/-5)
- `src/proto/h1/role.rs` (+84/-84)
- `src/proto/h2/ping.rs` (+1/-1)
- `src/proto/mod.rs` (+11/-11)
- `src/rt/bounds.rs` (+6/-6)
- `src/rt/io.rs` (+4/-4)
- `src/server/conn/http2.rs` (+2/-2)
- `src/server/conn/mod.rs` (+2/-2)
- `src/trace.rs` (+12/-12)
- `src/upgrade.rs` (+8/-23)
- `tests/server.rs` (+6/-6)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
