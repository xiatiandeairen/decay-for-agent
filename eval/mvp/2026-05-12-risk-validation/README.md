# Risk validation MVP run

Date: 2026-05-12

Purpose: run a small, auditable validation loop for decay diff risk evidence.

This run intentionally keeps only three cases:

- `hyper-3621`: open-source protocol/security bugfix candidate.
- `mxcore-dac56d9`: local real project TLS pinning feature commit.
- `mxcore-34af95f`: local docs-only negative control.

Important process rule: candidate selection and ground truth analysis must not use
`decay.stdout`. Decay evidence analysis uses only `decay.stdout` and may not read
ground truth outputs.

