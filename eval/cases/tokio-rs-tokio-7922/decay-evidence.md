# Decay Evidence

- evidence_count: 7

## Top risk findings

```text
1. [high] tokio/src/runtime/blocking/pool.rs
   semantic concurrency_ordering_changed let mut is_counted_idle;
   action: review wakeup, shutdown, or completion ordering and require focused concurrency regression coverage
2. [high] tokio/src/runtime/blocking/pool.rs
   semantic concurrency_ordering_changed is_counted_idle = false;
   action: review wakeup, shutdown, or completion ordering and require focused concurrency regression coverage
3. [high] tokio/src/runtime/blocking/pool.rs
   semantic concurrency_ordering_changed if is_counted_idle {
   action: review wakeup, shutdown, or completion ordering and require focused concurrency regression coverage
4. [high] tokio/src/runtime/blocking/pool.rs:run
   complexity worsened
   action: review function complexity and add focused regression tests
5. [medium] tokio/src/runtime/blocking/pool.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

## Complexity impact

```text
changed_functions=1 evidence=1 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot confidence=high value=direct
  function=tokio/src/runtime/blocking/pool.rs:run
  metrics=cognitive:22->23, cyclomatic:14->15, statements:42->46
```

## Coverage impact

```text
evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
```

## Stability impact

```text
evidence=0
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
```

## Quality impact

```text
evidence=2 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=build_or_ci_changed subject=build or ci source=quality confidence=low value=context
  file=.github/workflows/loom.yml
- id=3 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=tokio/src/runtime/blocking/pool.rs
```

## Semantic impact

```text
evidence=3
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=4 category=semantic kind=concurrency_ordering_changed subject=let mut is_counted_idle; source=semantic_diff confidence=high value=direct
  file=tokio/src/runtime/blocking/pool.rs
- id=5 category=semantic kind=concurrency_ordering_changed subject=is_counted_idle = false; source=semantic_diff confidence=high value=direct
  file=tokio/src/runtime/blocking/pool.rs
- id=6 category=semantic kind=concurrency_ordering_changed subject=if is_counted_idle { source=semantic_diff confidence=high value=direct
  file=tokio/src/runtime/blocking/pool.rs
```

## History impact

```text
evidence=1
  value_note=context only; excluded from top risk unless direct diff evidence exists
- id=7 category=history kind=recently_bugfixed_area subject=runtime: fix TOCTOU issue when decreasing `num_idle_threads` (#7918) source=git_history confidence=low value=context
  file=tokio/src/runtime/blocking/pool.rs
recorded=true changeset=1 evidence_saved=7
```
