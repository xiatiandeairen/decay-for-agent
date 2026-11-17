# ripgrep-3189

- repo: `BurntSushi/ripgrep`
- pr: `3189`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/3189
- title: ignore: fix global gitignore bug that arises with absolute paths
- merged_at: 2025-10-15T23:44:24Z
- review_decision: 
- labels: 

## Selection Reason

Global gitignore absolute-path behavior.

## Changed Files

- `CHANGELOG.md` (+2/-0)
- `crates/core/flags/hiargs.rs` (+7/-2)
- `crates/ignore/src/dir.rs` (+57/-4)
- `crates/ignore/src/gitignore.rs` (+4/-1)
- `crates/ignore/src/walk.rs` (+93/-4)
- `tests/regression.rs` (+50/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
