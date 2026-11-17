# BurntSushi-ripgrep-3174

- repo: `BurntSushi/ripgrep`
- pr: `3174`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/3174
- title: ignore: fix problem with searching whitelisted hidden files
- merged_at: 2025-10-09T01:16:59Z
- review_decision: 
- labels: 

## Selection Reason

Ancestor ignore whitelist path normalization false negative.

## Changed Files

- `CHANGELOG.md` (+2/-0)
- `crates/ignore/src/dir.rs` (+14/-10)
- `tests/regression.rs` (+21/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
