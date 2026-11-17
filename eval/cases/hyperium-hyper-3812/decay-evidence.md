# Decay Evidence

- evidence_count: 1

## Top risk findings

```text
1. [medium] src/proto/h1/io.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

## Complexity impact

```text
changed_functions=1 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
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
evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h1/io.rs
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
recorded=true changeset=1 evidence_saved=1
```
