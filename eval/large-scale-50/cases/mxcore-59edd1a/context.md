# mxcore-59edd1a context

Frozen inputs:

- Commit: `59edd1a`
- Parent: `412e52c`
- Subject: `feat(core-transfer): harden transfer execution`
- Stat: 7 files, 718 insertions, 160 deletions.
- Critical boundary: transfer execution across filesystem/network IO.
- Risk prompt points:
  - Large changes to `crates/core-transfer/src/core.rs`.
  - Adapter, config, error, types, and tests are changed together.
  - The phrase "harden" is a direct value signal, but ground truth must still
    identify concrete risks from diff/context.

Ground truth analysis must not use `decay.stdout`.

