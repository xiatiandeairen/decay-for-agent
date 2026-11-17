# Round 5 Review Summary

Reviewed 8 candidates with three independent roles:

- Ground Truth Agent: checks whether the risk/problem is independently reproducible from available context.
- Risk Signal Agent: checks whether the candidate maps to transferable risk signals.
- Skeptic Agent: blocks cases where evidence may be too thin, too project-specific, revert-shaped, or misleading for decay evaluation.

## Result

| Case | Final status | Why |
| --- | --- | --- |
| `hyper-4011` | `freeze_ready` | HTTP/1 trailer field casing is a clear external protocol contract; integration tests were added. |
| `ripgrep-3189` | `freeze_ready` | Absolute-path global gitignore behavior is externally visible and backed by regression/changelog evidence. |
| `hyper-3952` | `needs_more_context` | Strong waker/fairness risk, but must be compared with follow-up revert `hyper-3977`. |
| `ripgrep-3154` | `needs_more_context` | Correctness risk is plausible, but the PR describes a narrow hack and needs issue/test grounding. |
| `ripgrep-2885` | `needs_more_context` | False-negative risk is high value, but current evidence is optimizer-internal without enough invariant/test context. |
| `hyperium-hyper-3967` | `needs_more_context` | Large HTTP/2 CONNECT upgrade refactor has strong risk surface but insufficient issue/review/test evidence. |
| `tokio-rs-tokio-8057` | `needs_more_context` | Revert is strong evidence, but decay eval should target the original regression diff too. |
| `hyperium-hyper-3812` | `needs_more_context` | Partial-header panic is likely high value, but issue/test hunk context is missing. |

## Lessons

This round shows why strict aggregation is necessary. Ground-truth and signal-fit agents were willing to freeze more cases, but the skeptic correctly blocked cases that could teach the wrong lesson:

- Revert-only cases can invert the decay direction; the original risky diff must be included.
- Large protocol refactors can look risky structurally while lacking proof of a concrete user-visible failure.
- Optimizer and parser fixes can be high value, but need issue/test hunks to avoid overfitting to a tiny local condition.
- File metadata from the earlier collector was not always precise enough; round-5 corrected production/test paths for reviewed cases.

## Follow-Up Queue

- `hyper-3952`: compare with `hyper-3977`.
- `ripgrep-3154`: fetch issues `#2528` and `#3139`, plus test hunk.
- `ripgrep-2885`: fetch issue `#2884`, plus regression assertion.
- `hyperium-hyper-3967`: fetch issue `#3966` and app/review validation.
- `tokio-rs-tokio-8057`: compare original PR `#7757`, issue `#8056`, and revert.
- `hyperium-hyper-3812`: fetch issue `#3811` and test hunk.
