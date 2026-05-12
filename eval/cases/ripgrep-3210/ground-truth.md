# Ground Truth

## External Facts

- PR title/body: add POSIX character class support to globset; fixes #2962.
- Body lists supported classes and states ASCII-only definitions to avoid locale-dependent behavior.
- Diff adds parser/token conversion logic and extensive tests in `crates/globset`.

## True Risks

- severity: `high`
- expected reviewer concern: glob parser correctness, regex conversion, invalid class handling, ASCII-only semantics, compatibility with existing classes/ranges.
- risk mechanism: parser changes can create false matches, missed matches, invalid rollback behavior, or compatibility regressions.

## Non-Risks

- Complexity itself is not the product risk; parser behavior is.

## Expected Evidence

- useful: new high-risk parser functions, worsened parser/token conversion functions, tests covering valid/invalid POSIX classes.
- misleading: none obvious if findings stay on glob parser functions.
