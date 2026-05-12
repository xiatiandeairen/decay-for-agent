# Decay evidence: mxcore-dac56d9

Source isolation: this analysis used only `decay.stdout`.

## Evidence summary

- Complexity: 2 new high-risk functions
- Stability: 1 panic-like addition
- Quality: 3 dependency changes, 1 test gap, 1 sensitive test gap
- History: 2 churn hotspots
- Evidence overload: medium
- Signal state: non-null
- Top signals: `new_high_risk`, `panic_like_added`, `sensitive_test_gap`

## Evidence list

| ID | Category | Kind | Severity | Location | Actionability |
| --- | --- | --- | --- | --- | --- |
| 1 | complexity | new_high_risk | high | `crates/core-http/src/tls.rs:verify_pinned_chain` | Review control flow and split/test key paths. |
| 2 | complexity | new_high_risk | high | `crates/core-http/src/tls.rs:verify_server_cert` | Review verifier branch complexity and add path tests. |
| 3 | stability | panic_like_added | high | `crates/core-http/src/tls.rs`, line 13 | Check whether the failure path is runtime reachable. |
| 4 | quality | dependency_changed | medium | `Cargo.lock` | Review dependency impact. |
| 5 | quality | dependency_changed | medium | `Cargo.toml` | Review dependency rationale. |
| 6 | quality | dependency_changed | medium | `crates/core-http/Cargo.toml` | Review crate-level dependency impact. |
| 7 | quality | test_gap | medium | `crates/core-http/src/client.rs` | Confirm tests cover production behavior change. |
| 8 | quality | sensitive_test_gap | high | `crates/core-http/src/client.rs` | Add tests or justify no tests for sensitive production change. |
| 9 | history | churn_hotspot | medium | `crates/core-http/src/client.rs` | Raise review scrutiny because the file is recently hot. |
| 10 | history | churn_hotspot | medium | `crates/core-http/src/lib.rs` | Check module boundary impact in a recently touched file. |

