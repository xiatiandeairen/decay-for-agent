# Frozen v1 Fill B Review Summary

Formal review covered 8 candidates from pool C to fill the final 5-case gap.

## Result

| Bucket | Count | Cases |
| --- | ---: | --- |
| Newly freeze-ready | 5 | `BurntSushi-ripgrep-3174`, `BurntSushi-ripgrep-801`, `hyperium-hyper-3952`, `hyperium-hyper-3242`, `tokio-rs-tokio-8076` |
| Needs more context | 2 | `BurntSushi-ripgrep-2000`, `hyperium-hyper-1159` |
| Rejected | 1 | `hyperium-hyper-4019` |

## Aggregation Rule

The final five were selected only when all three roles agreed:

- strong ground truth;
- good fit to the current transferable signal catalog;
- no skeptic blocker.

`BurntSushi-ripgrep-2000` was not used even though the bug is real, because the PR is a platform/dependency recovery case with weak executable evidence. `hyperium-hyper-4019` was rejected because the stored record is effectively title-only.

## Impact

Frozen v1 can move from 35 to 40 cases and `status = "complete"`.
