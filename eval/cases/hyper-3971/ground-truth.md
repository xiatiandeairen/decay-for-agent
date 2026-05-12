# Ground Truth

## External Facts

- PR title/body: add public API for HeaderCaseMap/header casing.
- Body explicitly questions validation: parse as HeaderName vs comparing equivalence.
- Comments say design space is interesting enough to benefit from a design proposal first.
- Diff touches `src/ext/mod.rs` public extension API and HTTP/1 role encoding/parsing.

## True Risks

- severity: `high`
- expected reviewer concern: public API design, header casing validation, and protocol compatibility.
- risk mechanism: exposing header casing API can freeze API shape and alter HTTP/1 header serialization/parsing behavior.

## Non-Risks

- History-only context cannot prove the public API design risk.

## Expected Evidence

- useful: public API addition/change, protocol/header encoding path, validation concern.
- misleading: panic-like or history signals without connecting to API/protocol semantics.
