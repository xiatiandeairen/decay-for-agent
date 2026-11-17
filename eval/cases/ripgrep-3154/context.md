# ripgrep-3154

- repo: `BurntSushi/ripgrep`
- pr: `3154`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/3154
- title: printer: hack in a fix for `-l/--files-with-matches` when using `--pcre2 --multiline` with look-around
- merged_at: 2025-09-22T13:12:16Z
- review_decision: 
- labels: 

## Selection Reason

PCRE2 multiline files-with-matches output correctness.

## Changed Files

- `crates/printer/src/summary.rs` (+5/-1)
- `tests/regression.rs` (+41/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
