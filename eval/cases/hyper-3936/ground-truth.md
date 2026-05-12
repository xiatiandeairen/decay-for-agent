# Ground Truth

## External Facts

- PR title/body: add `http10_disable_keep_alive()` for server and client.
- Body says it fixes #3588 and security advisory GHSA-85g5-4q7m-6cgx.
- Comments discuss whether behavior belongs inside hyper or can be handled outside, then acknowledge connection pooling/poisoning nuance.
- Diff touches client/server builders and HTTP/1 parser/role/io code.

## True Risks

- severity: `high`
- expected reviewer concern: HTTP/1.0 keep-alive behavior, public config API, connection close semantics, and security compatibility.
- risk mechanism: wrong connection reuse/close handling can cause protocol/security problems and resource reuse bugs.

## Non-Risks

- Generic blast radius is secondary to protocol/API semantics.

## Expected Evidence

- useful: HTTP/1.0 keep_alive semantic change, public builder API addition, core parser/role changes, security advisory linkage.
- misleading: only reporting complexity without naming protocol behavior.
