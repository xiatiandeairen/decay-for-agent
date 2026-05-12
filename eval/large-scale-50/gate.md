# High-value gate

A diff is accepted only if both sections pass.

## A. Objective value signal

At least one hard signal is required:

- Linked issue describes a real user-visible bug, regression, security concern,
  compatibility break, crash, data loss, protocol violation, migration problem,
  or production failure.
- Review comment requests a behavioral fix, API correction, missing test,
  security clarification, split, rollback, or other risk-reducing change.
- Follow-up fix, revert, hotfix, or same-area patch lands soon after the diff.
- CI or test failure is tied to the changed behavior.
- Commit/PR message explicitly names regression, crash, panic, compatibility,
  safety, security, data corruption, migration, or production problem.
- The diff changes a known critical boundary: auth, TLS/certificate
  verification, crypto, storage format, wire protocol, public API, FFI,
  concurrency/runtime, data deletion, migration, sandboxing, or filesystem IO.

## B. Concrete risk prompt point

At least one actionable risk point is required:

- Specific function, file, public API, config, constant/default, dependency,
  migration, test gap, hot file, or behavior branch.
- A reviewer could act before merge by adding tests, splitting the PR, reverting
  a piece, preserving compatibility, changing error handling, or documenting a
  behavior contract.
- decay has at least one plausible evidence path: complexity, stability,
  quality, history, API surface, default-value change, dependency/security, or
  test expectation.

## Filter conditions

Filter out:

- Title-only bugfix without issue/review/follow-up/critical boundary.
- Large diff whose only signal is size.
- PR selected because decay output looked interesting.
- Cleanup/refactor with no behavior risk and no follow-up evidence.
- Docs/test/dependency-only changes unless they are tied to a concrete runtime,
  API, migration, or security risk.

