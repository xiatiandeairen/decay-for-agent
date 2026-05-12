# Decay Evidence

- evidence_count: 10

## Top risk findings

```text
1. [high] src/proto/h1/role.rs:parse_too_large_headers
   complexity new_high_risk
   action: review function complexity and add focused regression tests
2. [medium] src/body/incoming.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
3. [medium] src/body/incoming.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
4. [medium] src/body/incoming.rs:poll_frame
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
5. [medium] src/client/conn/http1.rs:handshake
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
6. [medium] src/error.rs:description
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
7. [medium] src/proto/h1/conn.rs:poll_read_head
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
8. [medium] src/proto/h1/role.rs:parse
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
9. [medium] src/proto/h1/role.rs:parse
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
```

## Complexity impact

```text
changed_functions=36 evidence=7 partial=false diagnostics=0
- id=1 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=src/body/incoming.rs:poll_frame
  metrics=cognitive:19->19, cyclomatic:12->12, statements:20->20
- id=2 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=src/client/conn/http1.rs:handshake
  metrics=cognitive:11->11, cyclomatic:10->10, statements:26->26
- id=3 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=src/error.rs:description
  metrics=cognitive:31->31, cyclomatic:31->31, statements:1->1
- id=4 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=src/proto/h1/conn.rs:poll_read_head
  metrics=cognitive:27->27, cyclomatic:15->15, statements:45->45
- id=5 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=src/proto/h1/role.rs:parse
  metrics=cognitive:54->54, cyclomatic:34->34, statements:98->98
- id=6 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot
  function=src/proto/h1/role.rs:parse
  metrics=cognitive:64->64, cyclomatic:29->29, statements:62->62
- id=7 category=complexity kind=new_high_risk subject=function source=diff_hunk+ast_snapshot
  function=src/proto/h1/role.rs:parse_too_large_headers
```

## Stability impact

```text
evidence=0
```

## Quality impact

```text
evidence=3 prod_files_changed=40 test_files_changed=12 docs_files_changed=0
- id=8 category=quality kind=build_or_ci_changed subject=build or ci source=quality
  file=build.rs
- id=9 category=quality kind=blast_radius subject=multiple crates or domains source=quality
  file=src/body/incoming.rs
- id=10 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=src/body/incoming.rs
```

## Semantic impact

```text
evidence=0
```

## History impact

```text
evidence=0
recorded=true changeset=1 evidence_saved=10
```
