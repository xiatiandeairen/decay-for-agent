# Agent Review Round 2

## Scope

Round 2 re-reviewed the 5 `needs_more_context` cases from Round 1 after adding follow-up issue/PR context.

New context is stored under:

- `eval/risk-signals/follow-up-targets.toml`
- `eval/risk-signals/follow-up-raw/`

## Result

| Case | Round 1 | Round 2 | Reason |
|---|---|---|---|
| `tokio-rs-tokio-8001` | `needs_more_context` | `freeze_ready` | Issue #8000 gives Linux UDP SO_ERROR reproduction and narrows the claim to recv error-readiness. |
| `hyper-3977` | `needs_more_context` | `freeze_ready` | Original #3952 and issue #3976 connect the revert to production 100% CPU reports and HTTP/1 poll-loop coordination. |
| `hyper-4038` | `needs_more_context` | `freeze_ready` | Issue #4033 gives concrete C NULL-handle repro and fixes the production file path to `src/ffi/http_types.rs`. |
| `hyper-3185` | `needs_more_context` | `freeze_ready` | Issue #3178 confirms user-visible HTTP/1 header timeout start semantics, with scope caveats. |
| `serde-rs-json-265` | `needs_more_context` | `needs_more_context` | Issue #256 is only the feature request; the implementation diff that introduced the regression is still missing. |

## Updated Freeze-Ready Set

Round 1 freeze-ready:

- `tokio-rs-tokio-7991`
- `hyper-4042`
- `BurntSushi-ripgrep-3182`

Round 2 upgraded:

- `tokio-rs-tokio-8001`
- `hyper-3977`
- `hyper-4038`
- `hyper-3185`

Total after Round 2: 7 freeze-ready cases.

## Still Blocked

- `serde-rs-json-265`

Required before reconsideration:

- Capture the implementation diff for the change associated with issue #256.
- Split the correctness-regression claim from the performance-regression claim.
- Keep `non_happy_path` rejected until the original diff supports it.

## Lessons

- Multi-agent review plus targeted follow-up is effective: 4/5 blocked cases were upgraded after missing evidence was added.
- The review remained strict: `serde-rs-json-265` stayed blocked even though the PR itself looks high value.
- Follow-up evidence must narrow claims, not just increase confidence. Several freeze-ready cases are valid only with scoped wording.
