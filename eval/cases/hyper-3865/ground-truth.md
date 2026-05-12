# Ground Truth

## External Facts

- PR title/body: refactor feature flags using cfg aliases.
- Comments explicitly discuss concern about adding a build dependency and build script overhead.
- Diff adds `build.rs` and touches many feature-gated modules across HTTP/1, HTTP/2, client/server, FFI, tests.

## True Risks

- severity: `medium`
- expected reviewer concern: cfg alias equivalence across feature combinations and whether build.rs/build dependency tradeoff is acceptable.
- risk mechanism: feature gate refactors can silently enable/disable code for some feature combinations; build scripts affect build parallelism and maintenance.

## Non-Risks

- Existing complex parser functions becoming touched due to cfg rewrites are not necessarily new functional complexity.

## Expected Evidence

- useful: build_or_ci/build.rs change, broad feature-gated blast radius, cfg/feature semantic changes.
- misleading: complexity debt on unchanged functions caused by cfg churn.
