# Round 6 Review Summary

Reviewed the final 8 `candidate` records with the same three-role process.

## Result

| Case | Final status | Why |
| --- | --- | --- |
| `ripgrep-3185` | `rejected` | Performance-shaped stdin buffering case without benchmark/repro evidence; current decay cannot verify throughput value. |
| `hyperium-hyper-820` | `rejected` | Build feature compatibility case with only a linked issue and no executable build-matrix evidence. |
| `ripgrep-2692` | `needs_more_context` | Reference-cycle ownership risk is plausible, but needs issue/profiling/repro proof. |
| `hyperium-hyper-3394` | `needs_more_context` | Missing Timer warning/panic is plausible, but may be a feature-shaped contract change. |
| `hyperium-hyper-310` | `needs_more_context` | Drain-on-drop is high-value, but review raised unresolved large-body resource risk. |
| `hyperium-hyper-518` | `needs_more_context` | Connector preservation is strong, but the regression-test assertion is not captured. |
| `tokio-rs-tokio-7638` | `needs_more_context` | Macro hygiene evidence is strong, but it may not fit the current function-level decay scanner. |
| `hyperium-hyper-3290` | `needs_more_context` | Double-poll panic is a strong async risk, but lacks reproducible/test evidence. |

## Lessons

- The candidate screen still admits cases that are real engineering issues but weak eval targets for current decay.
- Performance and build-matrix cases need executable measurements or commands, not only PR descriptions.
- Macro hygiene may be valuable for future semantic evidence, but should not be silently mixed into function-level decay evaluation.
- Async/concurrency panic cases need repro, issue, test, or follow-up evidence before being frozen.
- Old PRs can be useful, but only if issue and final patch context are recoverable.

## Follow-Up Queue

- `ripgrep-2692`: issue `#2690`, profiling/repro, reference-cycle proof.
- `hyperium-hyper-3394`: issue `#3393`, trigger config, test/review evidence.
- `hyperium-hyper-310`: issues `#197`/`#309`, final drain safety handling, test hunks.
- `hyperium-hyper-518`: issue `#495`, regression-test assertion.
- `tokio-rs-tokio-7638`: issue `#7637`, macro test hunks, explicit eval-scope decision.
- `hyperium-hyper-3290`: issue `#3289`, scheduling repro/follow-up confirmation.
