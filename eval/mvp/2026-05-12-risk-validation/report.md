# MVP validation report

Date: 2026-05-12

## Important summary

This MVP proves the validation workflow can run, but it also proves the
selection gate must be stricter.

Only one case in this run should count as high-value product validation:

- `mxcore-dac56d9`: real TLS pinning/security-boundary change.

The open-source case `hyper-3621` should be filtered out of the high-value set.
It remains useful as a process failure case because it looked risky from the
title and diff, but the frozen context did not include a linked issue, merge,
follow-up fix, revert, or hard review signal.

The current decay value signal is:

> decay is useful for pre-merge risk triage. It can point reviewers to complex
> security/protocol code, panic-like runtime paths, missing tests, dependency
> changes, mixed concerns, and hot files.

The current limitation is:

> decay does not yet understand enough project/domain semantics to catch many
> exact high-value risks.

## Execution trace

1. Selected three cases before using decay as a selector:
   - `hyper-3621`: open-source bugfix candidate with protocol/security semantics.
   - `mxcore-dac56d9`: local TLS pinning feature commit.
   - `mxcore-34af95f`: docs-only negative control.
2. Froze per-case inputs:
   - `metadata.toml`
   - `context.md`
   - `diff.patch`
   - `decay.stdout`
3. Ran independent subagent analysis:
   - Ground truth agent read only metadata/context/diff.
   - Decay evidence agent read only `decay.stdout`.
4. Compared ground truth risks against decay evidence.

## High-value selection policy

Going forward, non-high-value cases are filtered before the validation stage.
The detailed gate lives in `high-value-selection.md`.

A case must have both:

- Objective value signal: issue, review-required behavior change, follow-up fix,
  revert, failing test/CI tied to behavior, explicit production-risk commit
  message, or a known critical boundary.
- Concrete risk prompt point: a function, API, constant/default, dependency,
  test gap, hot file, or behavior change a reviewer could act on.

Cases without both are not counted as product-value proof.

## Case results

| Case | Candidate quality | Ground truth risks | Decay evidence | Result |
| --- | --- | ---: | ---: | --- |
| `hyper-3621` | medium confidence / filtered from high-value set | 4 | 3 | Useful process case, not product-value proof. Decay caught panic/mixed-concern risk but missed Host/userinfo and H2 limit/API semantics. |
| `mxcore-dac56d9` | high value | 4 | 10 | Strong value signal for triage and missing tests; semantic recall is incomplete. |
| `mxcore-34af95f` | negative control | 0 | 0 | Correct silence. |

## Metrics

- Cases: 3
- High-value validation cases: 1
- Medium-confidence cases: 1
- Filtered non-high-value cases: 1
- Negative controls: 1
- High/medium ground truth risks in high-value validation set: 3
- Fully/exactly covered risks in high-value validation set: 0
- Partially/weakly covered risks in high-value validation set: 2
- Missed high/medium risks in high-value validation set: 2
- Negative-control silence: 1/1
- Evidence overload cases: 0 high, 1 medium, 2 low/none

## What decay is proving in this MVP

Decay is useful as a pre-merge risk triage layer when the diff introduces:

- complex new security/protocol functions,
- panic-like runtime failure paths,
- sensitive production changes without tests,
- dependency changes,
- cross-domain or mixed-concern changes,
- changes in recently hot files.

The strongest signal is not "decay knows the exact bug". The stronger value is:

> This diff needs focused review and tests before merge, and here are the files/functions where the review should start.

## What decay is not yet proving

Decay did not identify several high-value semantic risks:

- Host header userinfo semantics in `hyper-3621`.
- Public API removal in `hyper-3621`.
- HTTP/2 default header limit changed from 16KB to 16MB in `hyper-3621`.
- "OS trust" implemented via bundled webpki roots in `mxcore-dac56d9`.
- trust-anchor pinning vs leaf/SPKI pinning ambiguity in `mxcore-dac56d9`.
- unsupported server-name fail-closed semantics in `mxcore-dac56d9`.

This means current decay evidence is structurally useful but not yet a semantic reviewer.

## Process problems found

1. Candidate selection is still too weak for open-source cases.
   `hyper-3621` looked high-value from title and discussion, but frozen data had no merge, linked issue, revert, or follow-up fix. It should be kept as a medium-confidence case, not counted as high-value proof.

2. "Bugfix PR" is not enough.
   A bugfix candidate can contain unrelated API/resource-limit changes. The validation pipeline must classify each risk inside the diff, not treat the PR as one label.

3. Exact semantic recall is the hard part.
   Decay evidence points to important review areas, but many ground truth risks require domain facts: protocol semantics, public API compatibility, trust-store behavior, and pinning model.

4. Negative control works in this small run.
   The docs-only mxcore case produced zero evidence. This is a real improvement over earlier false-positive behavior.

## Next MVP hardening steps

1. Improve candidate mining before adding more cases:
   - Require at least one hard signal for high-value open-source cases: linked issue, review requested change, follow-up fix, revert, failing CI tied to behavior, or merge with later same-area fix.
   - Keep title-only cases in `medium_confidence`.

2. Add semantic evidence sources:
   - Public API surface diff.
   - Constant/default value delta detector.
   - Security-sensitive dependency and feature detector.
   - Test expectation detector for sensitive domains.

3. Add missed-risk registry:
   - Record every high/medium ground truth risk that decay missed.
   - Use the registry to drive the next evidence-layer backlog.

4. Keep subagent isolation:
   - Ground truth: no decay output.
   - Decay evidence: no diff/context beyond raw decay output.
   - Comparison: only after both outputs are complete.
