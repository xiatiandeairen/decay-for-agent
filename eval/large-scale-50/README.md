# Large-scale 50 high-value diff validation

Date started: 2026-05-12

Goal: validate decay diff evidence on 50 naturally sampled high-value diffs,
and preserve enough context to turn every miss/noise/overload into a reusable
bad-case suite.

## Non-negotiable rule

Do not shape the corpus by target buckets. The corpus should reflect real issue
distribution. A case enters the validation set only if it passes the high-value
gate.

## Flow

1. Mine candidates from objective sources.
2. Apply the high-value gate.
3. Freeze context for accepted cases.
4. Run independent ground truth and decay-evidence analysis.
5. Compare results.
6. Preserve every failure as a bad case.

## Case states

- `candidate`: discovered, not yet evaluated.
- `accepted`: passed high-value gate, ready to freeze.
- `frozen`: input context saved; no further source mutation.
- `analyzed`: ground truth and decay evidence exist.
- `compared`: comparison exists.
- `bad_case_recorded`: one or more decay failures were archived.
- `filtered`: did not pass high-value gate.

## Directory layout

```text
eval/large-scale-50/
  README.md
  gate.md
  plan.md
  candidates/
    mxcore.toml
    open-source.toml
  cases/
    <case-id>/
      metadata.toml
      context.md
      diff.patch
      raw/
      ground-truth.md
      decay.stdout
      decay-evidence.md
      comparison.md
      bad-case.md
  bad-cases/
    <case-id>-<risk-id>.md
  reports/
    progress.md
    final.md
```

