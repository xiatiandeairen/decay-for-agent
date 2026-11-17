# Decay Evidence

- evidence_count: 5

## Top risk findings

```text
1. [high] src/ffi/http_types.rs
   quality sensitive_test_gap
   action: add or justify missing tests for sensitive code
2. [high] src/ffi/http_types.rs
   semantic boundary_expression_changed let req = non_null!(&mut *req ?= hyper_code::HYPERE_INVALID_ARG);
   action: review changed boundary expression and require focused edge-case tests
3. [high] src/ffi/http_types.rs
   semantic boundary_expression_changed let req = non_null!(&mut *req ?= std::ptr::null_mut());
   action: review changed boundary expression and require focused edge-case tests
4. [high] src/ffi/http_types.rs
   semantic boundary_expression_changed let resp = non_null!(&mut *resp ?= std::ptr::null_mut());
   action: review changed boundary expression and require focused edge-case tests
5. [medium] src/ffi/http_types.rs
   quality test_gap
   action: add focused tests or document why existing coverage is enough
```

## Complexity impact

```text
changed_functions=0 evidence=0 partial=false diagnostics=0
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
evidence=2 prod_files_changed=1 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=src/ffi/http_types.rs
- id=2 category=quality kind=sensitive_test_gap subject=sensitive production change without tests source=quality confidence=medium value=supporting
  file=src/ffi/http_types.rs
```

## Semantic impact

```text
evidence=3
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=3 category=semantic kind=boundary_expression_changed subject=let req = non_null!(&mut *req ?= hyper_code::HYPERE_INVALID_ARG); source=semantic_diff confidence=high value=direct
  file=src/ffi/http_types.rs
- id=4 category=semantic kind=boundary_expression_changed subject=let req = non_null!(&mut *req ?= std::ptr::null_mut()); source=semantic_diff confidence=high value=direct
  file=src/ffi/http_types.rs
- id=5 category=semantic kind=boundary_expression_changed subject=let resp = non_null!(&mut *resp ?= std::ptr::null_mut()); source=semantic_diff confidence=high value=direct
  file=src/ffi/http_types.rs
```

## History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=5
```
