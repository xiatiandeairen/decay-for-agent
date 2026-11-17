# Frozen v1 Fill A Review Summary

Formal review covered 13 candidates from screen A/B.

## Result

| Bucket | Count | Cases |
| --- | ---: | --- |
| Newly freeze-ready | 9 | `BurntSushi-ripgrep-454`, `BurntSushi-ripgrep-620`, `BurntSushi-ripgrep-762`, `hyperium-hyper-2544`, `hyperium-hyper-3275`, `serde-rs-json-521`, `tokio-rs-tokio-8021`, `tokio-rs-tokio-7983`, `hyperium-hyper-4069` |
| Needs more context | 1 | `hyperium-hyper-3782` |
| Duplicate alias | 2 | `hyperium-hyper-4043` is already `hyper-4043`; `BurntSushi-ripgrep-3189` is already `ripgrep-3189` |
| Rejected | 1 | `BurntSushi-ripgrep-1065` |

## Aggregation Rule

The final decision uses the strict rule:

- `freeze_ready` requires high ground truth, good signal fit, and no skeptic blocker.
- A duplicate alias cannot count toward the 40-case target even if the case is valuable.
- Rollup PRs are rejected as single frozen cases because they mix multiple risk sources into one judgement.

## Impact

Frozen v1 can move from 26 to 35 cases after adding the 9 newly accepted cases. The remaining gap is 5 cases.

## Blocked But Useful

- `hyperium-hyper-3782`: plausible one-line config-wiring risk, but needs h2 behavior/test evidence before freeze.
- `BurntSushi-ripgrep-1065`: useful only if split into child cases for issue `#1052` panic and issue `#1064` missing match.
