# mxcore-eeefff5 context

Frozen inputs:

- Commit: `eeefff5`
- Parent: `6b50f6c`
- Subject: `feat(core-http): retry transient request failures`
- Stat: 4 files, 181 insertions, 17 deletions.
- Critical boundary: HTTP retry and failure semantics.
- Risk prompt points:
  - `crates/core-http/src/client.rs` changes request execution and retry behavior.
  - `crates/core-http/tests/error_codes.rs` is updated.
  - Retry logic can change duplicate request behavior and observed errors.

Ground truth analysis must not use `decay.stdout`.

