# Decay Evidence

- evidence_count: 3

## Top risk findings

```text
1. [high] src/printer.rs
   1 panic-like additions
   action: review runtime failure and boundary-safety invariants
2. [medium] src/printer.rs
   coverage parse_coverage_gap
   action: treat decay output as partial and inspect this diff manually or update parser coverage
3. [medium] src/printer.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

## Complexity impact

```text
changed_functions=0 evidence=0 partial=true diagnostics=2
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
```

## Coverage impact

```text
evidence=1
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
- id=1 category=coverage kind=parse_coverage_gap subject=decay scan coverage is partial source=scan confidence=high value=coverage_limit
  file=src/printer.rs
```

## Stability impact

```text
evidence=1
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=2 category=stability kind=panic_like_added subject=line 302 source=stability confidence=high value=direct
  file=src/printer.rs
```

## Quality impact

```text
evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=3 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/printer.rs
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
diagnostic=parse error in src/printer.rs: source contains syntax errors
diagnostic=parse error in src/printer.rs: source contains syntax errors
recorded=true changeset=1 evidence_saved=3
```
