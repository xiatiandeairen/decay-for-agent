# Follow-Up B Review Summary

Reviewed the remaining 9 `needs_more_context` cases after fetching missing linked issues and related PRs.

## Result

| Case | Final status | Why |
| --- | --- | --- |
| `hyperium-hyper-3365` | `freeze_ready` | Zero idle timeout is a boundary config value that caused a concrete tokio interval panic. |
| `hyperium-hyper-3261` | `freeze_ready` | Graceful shutdown before first request could hang; scoped as shutdown lifecycle semantics. |
| `serde-rs-json-757` | `freeze_ready` | Invalid UTF-8 RawValue panic has minimal input, stacktrace, patch, and tests. |
| `hyper-4056` | `freeze_ready` | 32-bit u64-to-usize truncation in HTTP/1 body decoding is a clear integer boundary risk. |
| `hyper-4061` | `freeze_ready` | HTTP/2 flow-control capacity reservation before body data caused a concrete streaming deadlock path. |
| `hyper-3988` | `freeze_ready` | v2 write/flush readiness fix with chain context after #3952 revert and #3976 CPU report. |
| `BurntSushi-ripgrep-827` | `rejected` | Real Windows performance regression, but current M3 has no performance/platform signal. |
| `hyperium-hyper-3290` | `needs_more_context` | Real panic stack exists, but no stable repro/test and root-cause coverage remains questioned. |
| `serde-rs-json-265` | `needs_more_context` | Regression claim is plausible, but #256 diff and performance evidence are not captured. |

## Lessons

- Remaining NMC cases split cleanly: six can freeze, one belongs to a future performance track, two still lack stable repro/diff evidence.
- Some frozen cases need scoped wording. `hyperium-hyper-3261` is lifecycle semantics, not a broad claim that all prior shutdown behavior was wrong.
- `hyper-3988` must carry chain context; without the #3952/#3977/#3976 chain, it is easy to misread the value signal.
- Current M3 intentionally excludes platform performance regressions until a dedicated signal exists.
