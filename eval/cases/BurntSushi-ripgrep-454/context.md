# BurntSushi-ripgrep-454

- repo: `BurntSushi/ripgrep`
- pr: `454`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/454
- title: Fix reiteration of the first found match with --only-mathing flag
- merged_at: 2017-04-21T12:11:56Z
- review_decision: APPROVED
- labels: 

## Selection Reason

Only-matching match iteration repeated first match.

## Changed Files

- `src/printer.rs` (+9/-18)
- `tests/tests.rs` (+45/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
