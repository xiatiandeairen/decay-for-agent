# High-value case selection gate

Purpose: prevent validation from being polluted by cases that merely look risky.

## Core rule

A case enters the high-value validation set only if it has both:

1. Objective value signal.
2. Concrete risk prompt point visible before merge.

If either side is missing, the case is filtered out or kept only as a process note.

## Hard inclusion signals

At least one is required:

- Linked issue describes real user-visible bug, regression, security concern, compatibility break, crash, data loss, or protocol violation.
- Review comment requests behavioral fix, API correction, missing test, security clarification, or split because risk is real.
- Follow-up fix, revert, hotfix, or same-area patch lands soon after the original PR/commit.
- Failing CI or test failure is tied to the changed behavior, not just formatting or environment.
- Commit message explicitly names a production problem, compatibility issue, safety issue, data migration issue, or regression.
- The diff changes a known critical boundary: auth, TLS/cert verification, crypto, storage format, wire protocol, public API, FFI, concurrency/runtime, data deletion, billing/payment, or migration.

## Concrete risk prompt point

The case must have at least one reviewable risk point:

- A specific function, file, public API, constant/default, config, dependency, or test gap.
- A clear pre-merge question a reviewer could act on.
- A clear expected evidence class decay should have a chance to report.

Examples:

- Public API removed or behavior default changed.
- Security-sensitive dependency or feature added.
- New complex verifier/parser/state-machine code.
- Sensitive production change without tests.
- Hot file changed again after recent fix/churn.
- Repeated follow-up fixes in the same area after a feature PR.

## Exclusion rules

Filter out as non-high-value:

- Title-only bugfix with no issue, review, merge, follow-up, or clear critical boundary.
- Large diff whose only signal is size.
- PR where decay has interesting output but no independent objective value signal.
- Refactor/cleanup without behavior risk, unless follow-up fixes prove it caused issues.
- Docs/test/dependency-only changes unless the validation target explicitly includes those domains.

## Case buckets

- `high_value`: qualifies for product-value validation.
- `medium_confidence`: useful for process debugging, not counted as high-value proof.
- `negative_control`: used only to measure silence/noise.
- `discarded`: not used.

## MVP application

- `mxcore-dac56d9`: `high_value`.
  - Critical boundary: TLS certificate verification and pinning.
  - Risk prompt points: custom verifier, root store semantics, missing tests, dependencies.
- `mxcore-34af95f`: `negative_control`.
  - Docs-only, used to test silence.
- `hyper-3621`: `medium_confidence`, filtered out of high-value validation.
  - It has protocol/security-looking title and interesting diff risks, but frozen data lacks hard value signal such as merge, linked issue, follow-up fix, or review-requested behavior change.

