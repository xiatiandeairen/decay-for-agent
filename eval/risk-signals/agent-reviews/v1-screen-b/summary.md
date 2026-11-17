# Frozen v1 Screen B Preliminary Summary

This is a preliminary screen for the second 20-case collection batch. It is based on raw PR context plus selected follow-up refs, not a final three-role frozen review.

## Result

| Bucket | Count | Cases |
| --- | ---: | --- |
| Strong follow-up candidates | 5 | `tokio-rs-tokio-7983`, `hyperium-hyper-4043`, `hyperium-hyper-4069`, `BurntSushi-ripgrep-3189`, `BurntSushi-ripgrep-1065` |
| Needs strict review | 4 | `hyperium-hyper-3813`, `hyperium-hyper-4019`, `tokio-rs-tokio-8010`, `hyperium-hyper-2746` |
| Likely reject / out of scope | 11 | `tokio-rs-tokio-8068`, `hyperium-hyper-2792`, `hyperium-hyper-4002`, `BurntSushi-ripgrep-3181`, `clap-rs-clap-4668`, `clap-rs-clap-6068`, `clap-rs-clap-6205`, `clap-rs-clap-6368`, `serde-rs-json-257`, `serde-rs-json-1030`, `serde-rs-json-1205` |

## Strong Follow-Up Candidates

- `tokio-rs-tokio-7983`: cancelled io_uring open futures leaked file descriptors; issue labels it as bug, PR has tests and heavy review.
- `hyperium-hyper-4043`: chunked requests with trailers incorrectly closed keep-alive state; PR includes server test coverage.
- `hyperium-hyper-4069`: dispatcher dropped mid-body could silently truncate data; issue and continuation issue give user-visible failure context.
- `BurntSushi-ripgrep-3189`: global gitignore with rooted files behaved incorrectly; issue traces a real Helix report and PR touches ignore walking plus regression tests.
- `BurntSushi-ripgrep-1065`: mixed bugfix PR includes a panic case and a missing-match false negative, each with issue context. Final review must split the judgement by sub-issue to avoid treating a rollup as one clean diff.

## Needs Strict Review

- `hyperium-hyper-3813`: backport of partial-header panic fix. It looks valuable, but the original issue `#3811` still needs collection.
- `hyperium-hyper-4019`: title says non-UTF8 can panic in `to_str`, but the PR body is empty and no issue is captured.
- `tokio-rs-tokio-8010`: runtime handoff addresses starvation, but it is feature-shaped and broad. It needs a clear product-effect judgement before entering frozen v1.
- `hyperium-hyper-2746`: defensive panic avoidance around `Instant` has real reports, but the root cause may be outside hyper.

## Likely Reject / Out Of Scope

- Help/completion UI fixes, feature requests, compile-time/performance-only refactors, and config feature enablement are not good frozen-v1 product-effect cases unless they show a concrete stability or correctness failure.
- `tokio-rs-tokio-8068` is only a regression-test PR for a larger reverted change; it should not be frozen without the underlying fix context.

## Next Step

Use a formal three-role review on the 5 strong candidates plus the 4 strict-review candidates. Do not promote likely rejects just to fill the 40-case target.
