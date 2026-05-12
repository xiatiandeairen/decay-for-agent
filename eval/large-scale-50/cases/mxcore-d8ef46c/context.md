# mxcore-d8ef46c context

Frozen inputs:

- Commit: `d8ef46c`
- Parent: `cabc489`
- Subject: `feat(core-storage): R100 parity — flock + WAL hook + slow-query + corruption latch`
- Stat: 15 files, 1595 insertions, 294 deletions.
- Critical boundary: storage locking, WAL, corruption detection, diagnostics, FFI error surface.
- Risk prompt points:
  - `crates/core-storage/src/pool.rs` and `diag.rs` carry most behavior.
  - Swift headers and core-ffi modules change at the same time.
  - The diff is large enough to create evidence overload risk; analysis must
    distinguish actionable signals from generic size warnings.

Ground truth analysis must not use `decay.stdout`.

