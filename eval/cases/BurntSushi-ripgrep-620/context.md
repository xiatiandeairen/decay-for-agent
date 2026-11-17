# BurntSushi-ripgrep-620

- repo: `BurntSushi/ripgrep`
- pr: `620`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/620
- title: Fix printing context after an early return from a search
- merged_at: 2017-10-08T12:10:36Z
- review_decision: APPROVED
- labels: 

## Selection Reason

Early return skipped pending after-context output.

## Changed Files

- `src/search_stream.rs` (+43/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
