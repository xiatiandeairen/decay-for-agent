# ripgrep-2885

- repo: `BurntSushi/ripgrep`
- pr: `2885`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/2885
- title: regex: fix inner literal extraction that resulted in false negatives
- merged_at: 2024-09-09T02:00:47Z
- review_decision: 
- labels: 

## Selection Reason

Regex literal extraction false negative.

## Changed Files

- `crates/regex/src/literal.rs` (+21/-8)
- `tests/util.rs` (+2/-20)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
