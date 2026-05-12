# ripgrep-3230

- repo: `BurntSushi/ripgrep`
- pr: `3230`
- kind: `refactor`
- status: `candidate`
- url: https://github.com/BurntSushi/ripgrep/pull/3230
- title: clippy: fix `map_clone` lint
- merged_at: None
- review_decision: 
- labels: 

## Selection Reason

Clippy-driven change; useful for checking low-risk refactor noise.

## Changed Files

- `crates/core/search.rs` (+4/-4)
- `crates/globset/src/glob.rs` (+1/-1)
- `crates/matcher/tests/util.rs` (+1/-1)
- `crates/pcre2/src/matcher.rs` (+1/-1)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
