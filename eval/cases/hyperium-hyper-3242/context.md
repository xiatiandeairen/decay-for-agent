# hyperium-hyper-3242

- repo: `hyperium/hyper`
- pr: `3242`
- kind: `risk_signal`
- status: `freeze_ready`
- url: https://github.com/hyperium/hyper/pull/3242
- title: fix(client): Do not strip `path` and `scheme` components from URIs for HTTP/2 Extended CONNECT requests
- merged_at: 2023-08-03T17:33:57Z
- review_decision: APPROVED
- labels: 

## Selection Reason

HTTP/2 Extended CONNECT missing required pseudo-headers.

## Changed Files

- `src/client/client.rs` (+8/-0)
- `tests/integration.rs` (+42/-0)
- `tests/support/mod.rs` (+55/-33)

## High-Value Gate

- [ ] linked issue / bug report
- [ ] review risk discussion
- [ ] CI/test failure
- [ ] follow-up fix
- [ ] public API / protocol / storage / security / config / dependency sensitive change
- [ ] negative control

## Notes

TODO: fill before writing ground truth.
