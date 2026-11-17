# Frozen v1 Screen A Summary

This screen reviewed 20 newly enriched candidates for the remaining `frozen-v1` gap.

## Result

| Bucket | Count | Cases |
| --- | ---: | --- |
| Promote to formal review | 8 | `BurntSushi-ripgrep-454`, `BurntSushi-ripgrep-620`, `BurntSushi-ripgrep-762`, `hyperium-hyper-2544`, `hyperium-hyper-3275`, `hyperium-hyper-3782`, `serde-rs-json-521`, `tokio-rs-tokio-8021` |
| Needs more context | 4 | `tokio-rs-tokio-7910`, `clap-rs-clap-4049`, `serde-rs-json-187`, `BurntSushi-ripgrep-652` |
| Hold / weak context | 1 | `BurntSushi-ripgrep-554` |
| Reject | 7 | `clap-rs-clap-5836`, `hyperium-hyper-3430`, `hyperium-hyper-3538`, `hyperium-hyper-3557`, `serde-rs-json-303`, `serde-rs-json-384`, `serde-rs-json-770` |

## Promote To Formal Review

These cases have enough signal to justify a final frozen-review pass. They are not counted in `frozen-v1.toml` yet.

- `BurntSushi-ripgrep-454`: wrong `--only-matching` reiteration; issue repro, output difference, regression tests.
- `BurntSushi-ripgrep-620`: max-count early return dropped trailing context; issue repro, maintainer confirmation, regression tests.
- `BurntSushi-ripgrep-762`: gitignore slash-pattern behavior diverged from git; issue and review show external contract risk.
- `hyperium-hyper-2544`: malformed HTTP header path could panic; strong malformed-input/error-boundary case.
- `hyperium-hyper-3275`: HTTP/2 early response propagated `NO_ERROR`; issue, tests, review, and backport evidence are strong.
- `hyperium-hyper-3782`: wrong h2 builder parameter made a DoS-related limit ineffective; source PR chain is traceable.
- `serde-rs-json-521`: adjacently tagged `f32` roundtrip regression; clear issue, regression test, and release fix.
- `tokio-rs-tokio-8021`: semaphore closed bit could be reopened after `forget_permits`; issue, duplicate fix, and tests support the ground truth.

## Needs More Context

- `tokio-rs-tokio-7910`: issue `#6439` now confirms a bug-labeled idle metric double-increment, but final review must decide whether unstable metrics are high-value enough.
- `clap-rs-clap-4049`: issue and tests are strong, but PR admits an undesired `Command` side effect; needs resolution before freezing.
- `serde-rs-json-187`: panic-to-error change is plausible, but issue/review evidence is thin.
- `BurntSushi-ripgrep-652`: issue `#649` has now been collected; still needs test or concrete pre-fix failure confirmation.
- `BurntSushi-ripgrep-554`: plausible gitignore parent matching issue, but independent repro evidence is missing.

## Rejections

- Build-only, fuzz-only, feature-only, API-completeness, pure tuning, or no-current-signal-fit cases were rejected instead of used to fill the count.
- The important rejection lesson is that `signal_fit` alone is insufficient. A case can look semantically interesting and still be a poor product-effect validation sample if ground truth is weak or the value is outside the current scanner/evidence scope.

## Next Step

Run formal frozen review on the 8 promoted cases. If at least 6 pass, update `eval-candidates.toml`, `frozen-v1.toml`, and `eval/corpus/risk-signal-frozen-v1.toml`. Then continue another collection screen for the remaining gap rather than lowering the admission bar.
