# Agent Review Round 4

## Scope

Round 4 reviewed 8 more remaining `candidate` cases.

Reviewed:

- `hyperium-hyper-647`
- `hyperium-hyper-3365`
- `clap-rs-clap-1439`
- `hyperium-hyper-228`
- `hyperium-hyper-3261`
- `hyperium-hyper-3616`
- `BurntSushi-ripgrep-827`
- `hyper-4061`

## Result

| Case | Decision | Reason |
|---|---|---|
| `hyperium-hyper-647` | `freeze_ready` | Concrete invalid Content-Length stream-loss/panic path; scoped protocol-state claim. |
| `hyperium-hyper-3365` | `needs_more_context` | Plausible zero-duration panic, but lacks issue/test/diff-level invariant. |
| `clap-rs-clap-1439` | `reject` | Dependency/API compatibility revert is too release-policy-specific for this frozen eval without downstream breakage evidence. |
| `hyperium-hyper-228` | `freeze_ready` | Strong malformed chunk-size parser case with examples and regression-test claims. |
| `hyperium-hyper-3261` | `needs_more_context` | Shutdown hang is real-looking, but compatibility/breaking semantics need issue context. |
| `hyperium-hyper-3616` | `freeze_ready` | Upgrade + graceful_shutdown panic path with production code and test evidence. |
| `BurntSushi-ripgrep-827` | `needs_more_context` | Platform performance regression needs measurement or issue body; current catalog also lacks a clean performance/platform signal. |
| `hyper-4061` | `needs_more_context` | Sparse PR body; needs issue #4003 / PR #4051 context to prove capacity reservation risk. |

## Lessons

- `external_contract` is too broad for dependency/API reverts unless downstream breakage or executable compatibility evidence is present.
- Performance/platform regressions are real, but current 8-signal catalog does not model them cleanly.
- Skeptic gate prevented freezing cases whose PR title/body looked strong but lacked independent source-chain support.
