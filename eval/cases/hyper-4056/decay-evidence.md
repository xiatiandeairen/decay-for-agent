# Decay Evidence

- evidence_count: 4

## Top risk findings

```text
1. [high] src/proto/h1/decode.rs
   semantic integer_platform_boundary_changed let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX);
   action: review pointer-sized integer boundary behavior on 32-bit and large-input cases
2. [high] src/proto/h1/decode.rs
   semantic integer_platform_boundary_changed let to_read = usize::try_from(*rem).unwrap_or(usize::MAX);
   action: review pointer-sized integer boundary behavior on 32-bit and large-input cases
3. [medium] src/proto/h1/decode.rs
   quality test_gap
   action: add focused tests or document why existing coverage is enough
4. [medium] src/proto/h1/decode.rs:decode
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
```

## Complexity impact

```text
changed_functions=2 evidence=1 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=src/proto/h1/decode.rs:decode
  metrics=cognitive:32->32, cyclomatic:15->15, statements:35->35
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
evidence=1 prod_files_changed=1 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=src/proto/h1/decode.rs
```

## Semantic impact

```text
evidence=2
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=3 category=semantic kind=integer_platform_boundary_changed subject=let to_read = usize::try_from(*remaining).unwrap_or(usize::MAX); source=semantic_diff confidence=high value=direct
  file=src/proto/h1/decode.rs
- id=4 category=semantic kind=integer_platform_boundary_changed subject=let to_read = usize::try_from(*rem).unwrap_or(usize::MAX); source=semantic_diff confidence=high value=direct
  file=src/proto/h1/decode.rs
```

## History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=4
```
