# Ground Truth

## External Facts

- PR title/body says this is a small body refactor.
- Diff moves `Incoming` channel/h2 code out of `src/body/incoming.rs` into submodules and lightly touches HTTP/1 dispatch.
- No linked issue, no review concern, no CI failure, no follow-up fix in captured metadata.

## True Risks

- severity: `low`
- expected reviewer concern: behavior preservation across body channel split and HTTP/1 dispatch touch.
- risk mechanism: large move/split can accidentally change backpressure, abort, or frame forwarding behavior even when intended as refactor.

## Non-Risks

- Existing panic/expect/test assertions moved with the refactor should not be treated as newly introduced production risk.

## Expected Evidence

- useful: file split/move scope, body channel/h2 behavior area, tests moved with implementation.
- misleading: treating moved panic-like lines as new runtime failures.
