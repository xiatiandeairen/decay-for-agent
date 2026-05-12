# mxcore-dac56d9 context

Frozen inputs:

- Commit: `dac56d9`
- Parent: `eeefff5f04acb8969590b47682be15cd0e950d47`
- Subject: `feat(core-http): add TLS pinning`
- Stat: 6 files, 134 insertions, 4 deletions.
- Critical boundary: TLS certificate verification and pinning.
- Risk prompt points:
  - `crates/core-http/src/tls.rs` adds a custom rustls verifier.
  - HTTP client construction switches from default `reqwest::Client::new()` to preconfigured TLS.
  - New dependencies include `rustls` dangerous configuration and `webpki-roots`.
  - No tests are changed in this commit.

Ground truth analysis must not use `decay.stdout`.

