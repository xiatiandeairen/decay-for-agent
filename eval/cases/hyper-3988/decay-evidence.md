# Decay Evidence

- evidence_count: 3

## Top risk findings

```text
1. [high] src/proto/h1/dispatch.rs:poll_loop
   complexity worsened
   action: review function complexity and add focused regression tests
2. [medium] src/proto/h1/dispatch.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
3. [medium] dependency manifests changed
   1 dependency files changed
   action: review dependency and feature changes for behavior/security impact
```

## Complexity impact

```text
changed_functions=2 evidence=1 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot confidence=high value=direct
  function=src/proto/h1/dispatch.rs:poll_loop
  metrics=cognitive:7->20, cyclomatic:6->14, statements:8->17
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
evidence=2 prod_files_changed=1 test_files_changed=4 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=dependency_changed subject=dependency manifest source=quality confidence=low value=context
  file=Cargo.toml
- id=3 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h1/dispatch.rs
```

## Semantic impact

```text
evidence=0
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
```

## History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=3
```
