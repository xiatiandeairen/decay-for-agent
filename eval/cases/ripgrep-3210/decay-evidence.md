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
```

## Quality impact

```text
evidence=0 prod_files_changed=2 test_files_changed=1 docs_files_changed=0
```

## Semantic impact

```text
evidence=0
```

## History impact

```text
evidence=0
recorded=true changeset=1 evidence_saved=5
```
