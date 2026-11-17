# Follow-Up A Review Summary

Reviewed 12 `needs_more_context` cases after fetching linked issues and related PRs.

## Result

| Case | Final status | Why |
| --- | --- | --- |
| `ripgrep-3154` | `freeze_ready` | Concrete `--files-with-matches` + PCRE2 multiline look-around repro and bounded output-correctness scope. |
| `ripgrep-2885` | `freeze_ready` | Minimal false-negative repro and maintainer diagnosis of literal extraction. |
| `hyperium-hyper-518` | `freeze_ready` | Clear external API/lifecycle contract: setting SSL verifier discarded connector state/pool config. |
| `hyperium-hyper-3812` | `freeze_ready` | Concrete partial 1xx HTTP/1 panic issue plus regression-test path. |
| `hyperium-hyper-3290` | `needs_more_context` | Strong double-poll panic, but no stable repro/test and review questioned root-cause coverage. |
| `hyper-3952` | `rejected` | Strong evidence, but original fix was reverted for CPU hot-polling; not a clean frozen positive. |
| `ripgrep-2692` | `rejected` | Primarily memory/performance evidence; better fit for a future resource-lifecycle track. |
| `hyperium-hyper-3394` | `rejected` | Empty linked issue and feature/design-shaped PR. |
| `hyperium-hyper-310` | `rejected` | Underlying bug is real, but PR direction was disputed and later reverted. |
| `hyperium-hyper-3967` | `rejected` | Type-representation/API migration context, not validated runtime regression. |
| `tokio-rs-tokio-7638` | `rejected` | Macro hygiene is valuable but outside current function-level decay scope. |
| `tokio-rs-tokio-8057` | `rejected` | Revert-only target; original PR `#7757` should be collected instead. |

## Lessons

- Follow-up context can both promote and reject cases. The point is not to maximize frozen count.
- A real bug is not automatically a good frozen eval case. The diff must be the right target and the signal must fit current M3 scope.
- Revert-only, macro compile-time, type-soundness, and memory/performance cases need separate tracks or explicit future scope.
- Disputed or reverted fixes should not be frozen as positive examples, even when the underlying issue is real.
