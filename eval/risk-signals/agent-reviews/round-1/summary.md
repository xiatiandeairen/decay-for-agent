# Agent Review Round 1

## Scope

Eight candidates were reviewed by three independent subagents:

- Ground Truth Agent: real-world evidence strength.
- Risk Signal Agent: transferability and signal fit.
- Skeptic Agent: noise, overfit, and missing-context risk.

Aggregation used `eval/risk-signals/agent-review-rubric.md`. This is not a majority vote. Skeptic objections can block freezing.

## Result

| Case | Decision | Reason |
|---|---|---|
| `tokio-rs-tokio-7991` | `freeze_ready` | All gates passed; concrete panic edge path, production code, regression test, backport signal. |
| `tokio-rs-tokio-8001` | `needs_more_context` | Strong local risk, but OS/readiness semantics are too nuanced without issue/platform contract context. |
| `hyper-3977` | `needs_more_context` | Revert is strong evidence, but original fix and CPU report mechanism are not stored locally. |
| `hyper-4042` | `freeze_ready` | Clear HTTP/2 response-future drop lifecycle/state risk; error propagation signal excluded. |
| `hyper-4038` | `needs_more_context` | Null FFI boundary is real, but file mismatch, missing issue/repro, and no tests block freezing. |
| `hyper-3185` | `needs_more_context` | Real timeout lifecycle risk, but semantics are contested; cannot label as generic DoS/bug. |
| `BurntSushi-ripgrep-3182` | `freeze_ready` | Concrete panic boundary with production code and regression test. |
| `serde-rs-json-265` | `needs_more_context` | Strong follow-up regression evidence, but original #256 diff is needed and correctness/performance claims must be separated. |

## Freeze-Ready

- `tokio-rs-tokio-7991`
- `hyper-4042`
- `BurntSushi-ripgrep-3182`

## Needs More Context

- `tokio-rs-tokio-8001`
- `hyper-3977`
- `hyper-4038`
- `hyper-3185`
- `serde-rs-json-265`

## Lessons

- Multi-agent review is stricter than the initial candidate screen: only 3/8 passed freeze gates.
- The strongest blocker is not lack of risk, but missing source-chain precision.
- Reverts and follow-up fixes are valuable, but need original PR/issue comparison before freezing.
- For semantic eval, claims must be narrow. Several cases are good only if the expected claim avoids overgeneralization.
