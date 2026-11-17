# Decay Evidence

- evidence_count: 3

## Top risk findings

```text
1. [medium] ignore/src/dir.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
2. [medium] ignore/src/dir.rs:matched
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
3. [medium] ignore/src/dir.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

## Complexity impact

```text
changed_functions=4 evidence=1 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=ignore/src/dir.rs:matched
  metrics=cognitive:14->14, cyclomatic:13->13, statements:24->24
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
evidence=2 prod_files_changed=2 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality
  file=ignore/src/dir.rs
- id=3 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=ignore/src/dir.rs
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
