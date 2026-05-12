# mxcore-1bab54c context

Frozen inputs:

- Commit: `1bab54c`
- Parent: `8b460e1`
- Subject: `fix: cancel-safe read_frame + split FrameErr::Decode + buffer shrink`
- Stat: 1 file, 121 insertions, 41 deletions.
- Critical boundary: async frame read cancellation safety and buffer memory behavior.
- Risk prompt points:
  - `crates/platform/core-transport/src/io.rs` changes the read-frame path.
  - Commit message directly names cancel safety, decode error semantics, and
    buffer shrink behavior.

Ground truth analysis must not use `decay.stdout`.

