# Comparison: mxcore-dac56d9

## Ground truth coverage

| Ground truth risk | Match | Value | Matched decay evidence | Reason |
| --- | --- | --- | --- | --- |
| `r1-custom-cert-verifier-security-boundary` | partial | high | `new_high_risk`, `dependency_changed`, `sensitive_test_gap`, `churn_hotspot` | Decay strongly indicates this needs review and tests, but does not name TLS verifier security-boundary semantics. |
| `r2-use-os-trust-actually-webpki-roots` | missed | none | none | Decay cannot detect semantic mismatch between product wording and root store implementation. |
| `r3-pinning-semantics-ambiguous` | partial | medium | `new_high_risk`, `sensitive_test_gap` | Decay points to complex verifier functions and missing tests, but does not distinguish trust-anchor vs leaf/SPKI pinning. |
| `r4-unsupported-server-name-fallback` | missed | none | none | Decay did not identify fail-open/fail-closed behavior around unsupported server names. |

## Decay evidence quality

| Decay evidence group | Maps to ground truth | Actionable | Noise | Reason |
| --- | --- | --- | --- | --- |
| Complexity `new_high_risk` | yes | yes | no | Correctly focuses attention on verifier functions where semantic risks live. |
| Stability `panic_like_added` | weak | medium | possible | It flags `.expect(...)` in client construction. This may be less important than verifier semantics, but still asks for runtime failure review. |
| Dependency changes | partial | yes | no | Dependency changes are relevant because TLS implementation changes depend on new crypto/TLS crates. |
| Test gaps | yes | yes | no | Missing tests directly maps to the most important action before merge. |
| History hotspots | weak | medium | possible | Useful as context but not directly tied to the specific TLS semantic risks. |

## Process finding

This is the strongest MVP case. Decay produces valuable evidence for "review this before merge" and "tests are missing", but misses high-value semantic details. The current product value is risk triage, not root-cause semantic diagnosis.

