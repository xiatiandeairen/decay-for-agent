# hyper-3998

- repo: `hyperium/hyper`
- pr: `3998`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3998
- title: fix(client): reading trailers shouldn't propagate `NO_ERROR` from early response
- merged_at: 2026-04-22T21:40:56Z
- review_decision: APPROVED
- labels: 

## Selection Reason

Trailer NO_ERROR propagation.

## Changed Files

- `src/body/incoming.rs` (+18/-9)
- `tests/client.rs` (+89/-8)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
