# Decay Evidence

- evidence_count: 6

## Top risk findings

```text
1. [high] src/client/conn.rs:handshake
   complexity worsened
   action: review function complexity and add focused regression tests
2. [high] src/proto/h1/role.rs:parse
   complexity worsened
   action: review function complexity and add focused regression tests
3. [high] src/proto/h1/role.rs:parse
   complexity worsened
   action: review function complexity and add focused regression tests
4. [medium] src/client/client.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
5. [medium] src/client/client.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
6. [medium] src/proto/h1/io.rs:parse
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
```

## Complexity impact

```text
changed_functions=28 evidence=4 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot
  function=src/client/conn.rs:handshake
  metrics=cognitive:13->13, cyclomatic:11->11, statements:27->28
- id=2 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=src/proto/h1/io.rs:parse
  metrics=cognitive:24->24, cyclomatic:12->12, statements:31->31
- id=3 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot
  function=src/proto/h1/role.rs:parse
  metrics=cognitive:50->54, cyclomatic:32->33, statements:92->95
- id=4 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot
  function=src/proto/h1/role.rs:parse
  metrics=cognitive:70->76, cyclomatic:31->32, statements:66->69
```

## Stability impact

```text
evidence=0
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
```

## Quality impact

```text
evidence=2 prod_files_changed=10 test_files_changed=2 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=5 category=quality kind=blast_radius subject=multiple crates or domains source=quality
  file=src/client/client.rs
- id=6 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=src/client/client.rs
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
recorded=true changeset=1 evidence_saved=6
```
