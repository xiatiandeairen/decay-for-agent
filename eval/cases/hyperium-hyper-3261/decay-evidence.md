# Decay Evidence

- evidence_count: 4

## Top risk findings

```text
1. [high] src/proto/h1/dispatch.rs
   semantic lifecycle_ordering_changed if self.conn.is_write_closed() || self.conn.has_initial_read_write_state() {
   action: review shutdown/upgrade/cancel ordering and require regression coverage for cleanup or terminal-path behavior
2. [high] src/proto/h1/conn.rs
   semantic boundary_expression_changed && self.io.read_buf().is_empty()
   action: review changed boundary expression and require focused edge-case tests
3. [high] src/proto/h1/dispatch.rs
   semantic boundary_condition_changed if self.conn.is_write_closed() || self.conn.has_initial_read_write_state() {
   action: review changed boundary condition and require focused edge-case tests
4. [medium] src/proto/h1/conn.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

## Complexity impact

```text
changed_functions=2 evidence=0 partial=false diagnostics=0
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
evidence=1 prod_files_changed=2 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/proto/h1/conn.rs
```

## Semantic impact

```text
evidence=3
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=boundary_expression_changed subject=&& self.io.read_buf().is_empty() source=semantic_diff confidence=high value=direct
  file=src/proto/h1/conn.rs
- id=3 category=semantic kind=boundary_condition_changed subject=if self.conn.is_write_closed() || self.conn.has_initial_read_write_state() { source=semantic_diff confidence=high value=direct
  file=src/proto/h1/dispatch.rs
- id=4 category=semantic kind=lifecycle_ordering_changed subject=if self.conn.is_write_closed() || self.conn.has_initial_read_write_state() { source=semantic_diff confidence=high value=direct
  file=src/proto/h1/dispatch.rs
```

## History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=4
```
