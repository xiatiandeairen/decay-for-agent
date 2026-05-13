# Decay Evidence

- evidence_count: 5

## Top risk findings

```text
1. [high] crates/globset/src/glob.rs:from_name
   complexity new_high_risk
   action: review function complexity and add focused regression tests
2. [high] crates/globset/src/glob.rs:parse_posix_class
   complexity new_high_risk
   action: review function complexity and add focused regression tests
3. [high] crates/globset/src/glob.rs:to_char_set
   complexity new_high_risk
   action: review function complexity and add focused regression tests
4. [high] crates/globset/src/glob.rs:parse_class
   complexity worsened
   action: review function complexity and add focused regression tests
5. [high] crates/globset/src/glob.rs:tokens_to_regex
   complexity worsened
   action: review function complexity and add focused regression tests
```

## Complexity impact

```text
changed_functions=12 evidence=5 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=new_high_risk subject=function source=diff_hunk+ast_snapshot
  function=crates/globset/src/glob.rs:from_name
- id=2 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot
  function=crates/globset/src/glob.rs:parse_class
  metrics=cognitive:25->36, cyclomatic:14->21, statements:38->46
- id=3 category=complexity kind=new_high_risk subject=function source=diff_hunk+ast_snapshot
  function=crates/globset/src/glob.rs:parse_posix_class
- id=4 category=complexity kind=new_high_risk subject=function source=diff_hunk+ast_snapshot
  function=crates/globset/src/glob.rs:to_char_set
- id=5 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot
  function=crates/globset/src/glob.rs:tokens_to_regex
  metrics=cognitive:32->34, cyclomatic:18->19, statements:32->34
```

## Stability impact

```text
evidence=0
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
```

## Quality impact

```text
evidence=0 prod_files_changed=2 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
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
recorded=true changeset=1 evidence_saved=5
```
