# Comparison: hyper-3621

## Ground truth coverage

| Ground truth risk | Match | Value | Matched decay evidence | Reason |
| --- | --- | --- | --- | --- |
| `r1-host-userinfo-leak` | missed | none | none | Decay did not identify Host header/userinfo semantics or example behavior. |
| `r2-unrelated-h2-client-api-removal` | partial | medium | `blast_radius`, `mixed_concerns` | Decay saw cross-domain/mixed concerns, but did not identify public API removal. |
| `r3-server-h2-header-limit-raised` | weak | low | `blast_radius`, `mixed_concerns` | Cross-domain evidence hints at review risk but misses resource-limit semantics. |
| `r4-http1-upgrade-inner-unwrap` | exact | medium | `panic_like_added` | Decay directly flagged the newly panic-like line. |

## Decay evidence quality

| Decay evidence | Maps to ground truth | Actionable | Noise | Reason |
| --- | --- | --- | --- | --- |
| `panic_like_added` | yes | yes | no | It maps to the `inner.unwrap()` behavior change. |
| `blast_radius` | partial | yes | no | It correctly identifies that unrelated production domains are mixed. |
| `mixed_concerns` | partial | yes | no | It helps reviewer ask for split/rationale, but lacks specific semantic diagnosis. |

## Process finding

This case should not be counted as a strong high-value validation case. It is a useful pilot because it exposes candidate-selection weakness: PR title and review comments indicate real risk, but there is no merge, linked issue, or follow-up fix in the frozen evidence.

