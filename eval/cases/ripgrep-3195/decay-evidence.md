# Decay Evidence

- evidence_count: 5

## Top risk findings

```text
1. [high] crates/searcher/src/line_buffer.rs
   semantic boundary_condition_changed if readlen == 0 {
   action: review changed boundary condition and require focused edge-case tests
2. [medium] crates/printer/src/hyperlink/aliases.rs
   quality test_gap
   action: add focused tests or document why existing coverage is enough
3. [medium] crates/printer/src/hyperlink/aliases.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
4. [medium] crates/printer/src/hyperlink/aliases.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
```

## Complexity impact

```text
changed_functions=3 evidence=1 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=improved subject=function source=diff_hunk+ast_snapshot confidence=medium value=direct
  function=crates/searcher/src/line_buffer.rs:fill
  metrics=cognitive:25->20, cyclomatic:15->13, statements:33->29
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
evidence=3 prod_files_changed=3 test_files_changed=0 docs_files_changed=1
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=crates/printer/src/hyperlink/aliases.rs
- id=3 category=quality kind=blast_radius subject=multiple crates or domains source=quality confidence=low value=context
  file=crates/printer/src/hyperlink/aliases.rs
- id=4 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=crates/printer/src/hyperlink/aliases.rs
```

## Semantic impact

```text
evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=5 category=semantic kind=boundary_condition_changed subject=if readlen == 0 { source=semantic_diff confidence=high value=direct
  file=crates/searcher/src/line_buffer.rs
```

## History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=5
```
