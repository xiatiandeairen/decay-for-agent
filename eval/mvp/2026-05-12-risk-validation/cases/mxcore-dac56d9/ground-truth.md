# Ground truth: mxcore-dac56d9

Source isolation: this analysis used only `metadata.toml`, `context.md`, and `diff.patch`.

## Risks

### mxcore-dac56d9-r1-custom-cert-verifier-security-boundary

- Severity: high
- Confidence: high
- Evidence source: code semantics
- Affected files/functions: `crates/core-http/src/tls.rs::{MxServerCertVerifier,verify_server_cert,verify_pinned_chain}`, `crates/core-http/src/client.rs::HttpClient::new`
- Fact: the HTTP client changes from `reqwest::Client::new()` to a custom rustls verifier; `dangerous_configuration` is added.
- Inference: this changes the TLS certificate verification boundary.
- Why it matters: verifier mistakes can cause failed pinning, false rejection, or incorrect trust.
- Action before merge: require tests for pinned accept, wrong pin reject, reject host, trust fallback, expired cert, and wrong-domain cert.

### mxcore-dac56d9-r2-use-os-trust-actually-webpki-roots

- Severity: medium
- Confidence: high
- Evidence source: code semantics
- Affected files/functions: `crates/core-http/src/tls.rs::{os_root_store,MxServerCertVerifier::new,HostVerdict::UseOsTrust}`
- Fact: `UseOsTrust` uses `webpki_roots::TLS_SERVER_ROOTS`, not the system trust store.
- Inference: implementation semantics differ from the name and possible product expectation.
- Why it matters: private CA, enterprise roots, and OS trust updates may behave differently than expected.
- Action before merge: rename/document as webpki roots or implement true OS trust; add private CA behavior coverage.

### mxcore-dac56d9-r3-pinning-semantics-ambiguous

- Severity: medium
- Confidence: medium
- Evidence source: code semantics
- Affected files/functions: `crates/core-http/src/tls.rs::verify_pinned_chain`
- Fact: `Pinned(anchors)` is loaded into `RootCertStore` as trust anchors.
- Inference: this is trust-anchor pinning, not necessarily leaf or SPKI pinning.
- Why it matters: misunderstood pinning semantics can make users believe a narrower trust policy is enforced.
- Action before merge: define `PinConfig` semantics and test root/intermediate/leaf input behavior.

### mxcore-dac56d9-r4-unsupported-server-name-fallback

- Severity: low
- Confidence: medium
- Evidence source: code semantics
- Affected files/functions: `crates/core-http/src/tls.rs::server_name_to_host`
- Fact: unsupported `ServerName` maps to an empty string.
- Inference: empty host may flow into default host verdict rather than fail closed.
- Why it matters: unknown certificate verifier inputs should generally reject explicitly.
- Action before merge: reject unsupported `ServerName` directly and test it.

## Candidate quality

`high_value`.

Reason: this is a real security-boundary change in a local project commit. Even without external issue data, the diff itself contains enough security-critical semantics for high-value validation.

