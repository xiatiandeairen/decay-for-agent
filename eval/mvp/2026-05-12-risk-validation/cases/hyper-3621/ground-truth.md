# Ground truth: hyper-3621

Source isolation: this analysis used only `metadata.toml`, `context.md`, and `diff.patch`.

## Risks

### hyper-3621-r1-host-userinfo-leak

- Severity: medium
- Confidence: medium
- Evidence source: review + code semantics
- Affected files/functions: `examples/client.rs::fetch_url`, `examples/client_json.rs::fetch_json`, `examples/single_threaded.rs::{http1_client,http2_client}`
- Fact: old code used `url.authority()` for `Host`; new code uses `format!("{}:{}", host, port)`.
- Inference: if authority includes `userinfo@host`, the old examples could include user info in `Host`.
- Why it matters: examples can be copied into production; wrong `Host` may leak credential fragments or break routing.
- Action before merge: add a userinfo URI test, document Host construction, and verify default port behavior.

### hyper-3621-r2-unrelated-h2-client-api-removal

- Severity: medium
- Confidence: high
- Evidence source: code semantics
- Affected files/functions: `src/client/conn/http2.rs::Builder::max_header_list_size`, `src/proto/h2/client.rs::{Config,new_builder}`
- Fact: the diff removes a public HTTP/2 client builder method, config field, default, and builder wiring.
- Inference: this looks unrelated to the Host header fix and may be accidental API/behavior regression.
- Why it matters: users lose the ability to limit received header list size; this has compatibility and resource-control implications.
- Action before merge: split into a separate PR, keep the API, or provide semver rationale and tests.

### hyper-3621-r3-server-h2-header-limit-raised

- Severity: medium
- Confidence: high
- Evidence source: code semantics
- Affected files/functions: `src/proto/h2/server.rs::DEFAULT_SETTINGS_MAX_HEADER_LIST_SIZE`, `src/server/conn/http2.rs::Builder::max_header_list_size`
- Fact: HTTP/2 server default header list size changes from `16KB` to `16MB`.
- Inference: raising a default resource limit by 1024x changes the server exposure surface.
- Why it matters: this can affect memory pressure and denial-of-service posture.
- Action before merge: require independent rationale, boundary tests, and release note.

### hyper-3621-r4-http1-upgrade-inner-unwrap

- Severity: low
- Confidence: medium
- Evidence source: code semantics
- Affected files/functions: `src/server/conn/http1.rs::Connection::poll`
- Fact: old code handled `inner == None`; new code calls `self.inner.as_mut().unwrap()`.
- Inference: if polled after upgrade/finalization, behavior could change from ready to panic, although future-after-ready semantics weaken this risk.
- Why it matters: this is an unrelated panic-prone behavior change in connection state handling.
- Action before merge: preserve old branch or add a test/proof that the state is unreachable.

## Candidate quality

`medium_confidence`.

Reason: the PR has real protocol/security semantics, but the frozen context says closed/not merged and has no linked issue or follow-up fix. It is useful for process testing, not a strong high-value ground truth case.

