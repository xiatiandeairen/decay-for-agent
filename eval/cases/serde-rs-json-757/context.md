# serde-rs-json-757

- repo: `serde-rs/json`
- pr: `757`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/serde-rs/json/pull/757
- title: Fix panic deserializing RawValue from invalid utf-8 bytes
- merged_at: 2021-02-28T04:54:05Z
- review_decision: 
- labels: 

## Selection Reason

RawValue invalid UTF-8 panic.

## Changed Files

- `src/read.rs` (+4/-1)
- `tests/test.rs` (+19/-0)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
