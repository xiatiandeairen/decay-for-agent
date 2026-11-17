# BurntSushi-ripgrep-762

- repo: `BurntSushi/ripgrep`
- pr: `762`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/762
- title: Make handling of patterns containing a `/` match actual git behaviour
- merged_at: 2018-01-29T19:11:00Z
- review_decision: APPROVED
- labels: 

## Selection Reason

Gitignore slash-pattern external contract mismatch.

## Changed Files

- `ignore/src/gitignore.rs` (+11/-10)
- `ignore/src/overrides.rs` (+2/-1)
- `ignore/tests/gitignore_matched_path_or_any_parents_tests.rs` (+8/-8)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
