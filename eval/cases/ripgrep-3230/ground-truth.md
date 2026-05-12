# Ground Truth

## External Facts

- PR title/body: clippy `map_clone` lint fix.
- Diff changes small mechanical clone/map calls across four files including tests.
- Maintainer comment says future lint streams should be grouped differently, not that this PR has behavior risk.

## True Risks

- severity: `control`
- expected reviewer concern: mechanical lint correctness and scope hygiene.
- risk mechanism: low behavior risk; potential concern is review noise from many lint PRs.

## Non-Risks

- Multi-file mechanical lint should not be treated as behavioral mixed-concern risk.

## Expected Evidence

- useful: ideally no risk evidence, or low-severity mechanical refactor note.
- misleading: blast/mixed-concerns as if this were a broad feature change.
