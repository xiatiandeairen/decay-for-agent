# ripgrep-3195

- repo: `BurntSushi/ripgrep`
- pr: `3195`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/BurntSushi/ripgrep/pull/3195
- title: searcher: fix regression with `--line-buffered` flag
- merged_at: 2025-10-19T15:06:40Z
- review_decision: 
- labels: 

## Selection Reason

Line-buffered searcher buffering regression.

## Changed Files

- `CHANGELOG.md` (+7/-4)
- `crates/searcher/src/line_buffer.rs` (+8/-13)
- `crates/searcher/src/searcher/glue.rs` (+2/-2)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
