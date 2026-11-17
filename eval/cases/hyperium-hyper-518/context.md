# hyperium-hyper-518

- repo: `hyperium/hyper`
- pr: `518`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/518
- title: fix(client): keep the underlying connector when setting an SSL verifier
- merged_at: 2015-05-10T05:46:43Z
- review_decision: REVIEW_REQUIRED
- labels: 

## Selection Reason

SSL verifier connector lifecycle contract.

## Changed Files

- `benches/client.rs` (+4/-1)
- `src/client/mod.rs` (+38/-5)
- `src/client/pool.rs` (+21/-2)
- `src/mock.rs` (+38/-1)
- `src/net.rs` (+15/-1)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
