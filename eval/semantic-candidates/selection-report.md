# Semantic Risk Candidate Selection - 2026-05-13

## Goal

Build a strict, high-confidence seed set for M3 semantic evidence work. The set should contain real risk records, not convenient examples. A record is accepted only when the PR carries a concrete failure mode or merge risk that a reviewer would reasonably want surfaced before or during merge.

## Collection Run

- Source repositories: `hyperium/hyper`, `BurntSushi/ripgrep`, `tokio-rs/tokio`, `rust-lang/cargo`, `clap-rs/clap`, `serde-rs/json`.
- Raw candidate pool: 374 merged PRs.
- Scored candidates: 374.
- Enriched candidates: 34 with score >= 15.
- GitHub search hit rate limit during collection, but enough high-score candidates were collected for the first strict seed pass.

## Strict Acceptance Gate

Accepted records must satisfy all of:

- Concrete risk type: panic, regression, race, protocol/parse correctness, compatibility break, shutdown/lifecycle bug, or externally visible behavior bug.
- Code-bearing diff: at least one production code file, not test-only or docs-only.
- Context evidence: PR title/body, linked issue wording, tests, comments, reviews, labels, or revert wording explain why the change matters.
- Reviewable by decay later: the risk should plausibly leave code-level evidence in the diff or surrounding context.

Rejected or deferred records include:

- Dependency automation and release-note noise.
- WIP/prototype feature PRs without a clear failure mode.
- Pure test infra fixes or test-only regression records.
- Broad product features where the risk is plausible but not grounded enough.
- Environment/build fixes unless they expose production code risk.

## Selected Seed Records

1. `tokio-rs-tokio-7991` - <https://github.com/tokio-rs/tokio/pull/7991>
   - Risk: panic caused by an incorrect assertion path in `Chan::recv_many`.
   - Evidence: failure explained in PR body, linked issue, tests added, 14 reviews, 2 comments.
   - Files: `tokio/src/sync/mpsc/chan.rs`, `tokio/tests/sync_mpsc.rs`.

2. `tokio-rs-tokio-8001` - <https://github.com/tokio-rs/tokio/pull/8001>
   - Risk: UDP `SO_ERROR` readiness was not surfaced correctly on Linux.
   - Evidence: linked issue, behavioral explanation, test file changed, 7 reviews, 13 comments.
   - Files: `tokio/src/net/udp.rs`, `tokio/tests/udp.rs`.

3. `hyperium-hyper-647` - <https://github.com/hyperium/hyper/pull/647>
   - Risk: HTTP/1 invalid content length handling could lose the stream.
   - Evidence: explicit HTTP parser correctness fix in core protocol file.
   - Files: `src/http/h1.rs`.

4. `hyperium-hyper-3365` - <https://github.com/hyperium/hyper/pull/3365>
   - Risk: client pool idle timeout set to zero could panic.
   - Evidence: explicit panic fix in connection pool logic.
   - Files: `src/client/pool.rs`.

5. `clap-rs-clap-1439` - <https://github.com/clap-rs/clap/pull/1439>
   - Risk: dependency upgrade changed public API expectations enough to require a major release.
   - Evidence: revert PR explains breaking API impact and security trade-off.
   - Files: `Cargo.toml`, `src/args/arg.rs`, `src/args/group.rs`, `src/lib.rs`.

6. `hyperium-hyper-228` - <https://github.com/hyperium/hyper/pull/228>
   - Risk: invalid HTTP chunk size parsing.
   - Evidence: protocol parser correctness fix across client/server request handling.
   - Files: `src/client/response.rs`, `src/http.rs`, `src/server/request.rs`.

7. `hyperium-hyper-3261` - <https://github.com/hyperium/hyper/pull/3261>
   - Risk: HTTP/1 graceful shutdown behavior was incorrect.
   - Evidence: shutdown fix with server tests and discussion.
   - Files: `src/proto/h1/conn.rs`, `src/proto/h1/dispatch.rs`, `tests/server.rs`.

8. `hyperium-hyper-3616` - <https://github.com/hyperium/hyper/pull/3616>
   - Risk: `graceful_shutdown` could panic after HTTP/1 upgrade.
   - Evidence: explicit panic avoidance and test file change.
   - Files: `src/server/conn/http1.rs`, `tests/server.rs`.

9. `tokio-rs-tokio-7922` - <https://github.com/tokio-rs/tokio/pull/7922>
   - Risk: race condition during blocking pool shutdown.
   - Evidence: explicit race condition fix, loom workflow/test context, reviews and comments.
   - Files: `tokio/src/runtime/blocking/pool.rs`, `tokio/src/runtime/tests/loom_blocking.rs`.

10. `BurntSushi-ripgrep-827` - <https://github.com/BurntSushi/ripgrep/pull/827>
    - Risk: performance regression on Windows in ignore walking.
    - Evidence: explicit performance regression title in production traversal code.
    - Files: `ignore/src/walk.rs`.

11. `BurntSushi-ripgrep-3182` - <https://github.com/BurntSushi/ripgrep/pull/3182>
    - Risk: panic in replacement handling for a look-around corner case.
    - Evidence: explicit panic fix with regression test.
    - Files: `crates/printer/src/util.rs`, `tests/regression.rs`.

12. `serde-rs-json-757` - <https://github.com/serde-rs/json/pull/757>
    - Risk: panic while deserializing `RawValue` from invalid UTF-8 bytes.
    - Evidence: explicit invalid input panic fix with test file change.
    - Files: `src/read.rs`, `tests/test.rs`.

## Deferred Examples

- `serde-rs-json-1161`: performance/vectorization PR. Useful later for performance-risk evidence, but current context is not as clear as panic/regression records.
- `serde-rs-json-416`: broad arbitrary-precision feature. High review volume, but too feature-shaped for the first risk seed.
- `tokio-rs-tokio-8068`: regression-test-only PR. Useful as supporting evidence for another case, not a primary diff risk case.
- `BurntSushi-ripgrep-642`: test skip on APFS. Real project pain, but mostly test-environment behavior.
- `hyperium-hyper-384`: old rustup/build compatibility fix. Evidence is weak for production diff risk.
- `hyperium-hyper-2975`: broad cargo feature removal. Potentially important, but needs stronger external breakage context before acceptance.

## Immediate Follow-up

The first selected set is strong enough to start M3.1 semantic prompts, but not enough to claim product value. Next pass should add follow-up fix detection: for each accepted PR, inspect nearby issues/PRs and commits after merge to see whether the original PR itself introduced later fixes or whether it fixed a real reported user problem.
