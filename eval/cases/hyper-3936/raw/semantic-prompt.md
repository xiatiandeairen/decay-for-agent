# Task

You are the semantic selection layer for decay v0.2.0 M3.1.
Use only the fact evidence below. Do not use external APIs, web search, or unstated repository knowledge.

# Output Contract

Return JSON only, matching semantic-findings.v1:

```json
{"version":"semantic-findings.v1","findings":[]}
```

Each finding must include claim, supporting_facts, source_refs, confidence, limitation, and review_action.
Every source_refs item must be an evidence id from the allowed list.
Do not create a finding if it cannot cite at least one allowed evidence id.
Do not promote weak-only signals: mixed_concerns, blast_radius, history-only, or pure touched_existing_risky.
If the fact evidence is empty or weak-only, return an empty findings array.

# Allowed Evidence Ids

1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/hyper-3936/repo
  mode=git_ref
  base_input=72ebcffb7d82cda15aa74507b2cc522ca2a7a94d
  base=72ebcffb7d82cda15aa74507b2cc522ca2a7a94d
  head=8474cf8145838dc92e0170eaf8364a18cb82439a
  changed_files=10
```

# Top risk findings

```text
1. [high] src/client/client.rs
   1 public API function added
   action: review runtime failure and boundary-safety invariants
2. [high] src/client/conn.rs
   1 public API function added
   action: review runtime failure and boundary-safety invariants
3. [high] src/server/conn.rs
   1 public API function added
   action: review runtime failure and boundary-safety invariants
4. [high] src/server/conn/http1.rs
   1 public API function added
   action: review runtime failure and boundary-safety invariants
5. [high] src/server/server.rs
   1 public API function added
   action: review runtime failure and boundary-safety invariants
6. [high] src/client/conn.rs:handshake
   complexity worsened
   action: review function complexity and add focused regression tests
7. [high] src/proto/h1/role.rs:parse
   complexity worsened
   action: review function complexity and add focused regression tests
8. [high] src/proto/h1/role.rs:parse
   complexity worsened
   action: review function complexity and add focused regression tests
9. [medium] src/client/client.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
10. [medium] src/client/client.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
  truncated=true remaining=2
```

# Complexity impact

```text
changed_functions=28 evidence=4 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot confidence=high value=direct
  function=src/client/conn.rs:handshake
  metrics=cognitive:13->13, cyclomatic:11->11, statements:27->28
- id=2 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=src/proto/h1/io.rs:parse
  metrics=cognitive:24->24, cyclomatic:12->12, statements:31->31
- id=3 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot confidence=high value=direct
  function=src/proto/h1/role.rs:parse
  metrics=cognitive:50->54, cyclomatic:32->33, statements:92->95
- id=4 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot confidence=high value=direct
  function=src/proto/h1/role.rs:parse
  metrics=cognitive:70->76, cyclomatic:31->32, statements:66->69
```

# Coverage impact

```text
evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
```

# Stability impact

```text
evidence=5
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=5 category=stability kind=public_api_function_added subject=http10_disable_keep_alive source=stability confidence=high value=direct
  file=src/client/client.rs
- id=6 category=stability kind=public_api_function_added subject=http10_disable_keep_alive source=stability confidence=high value=direct
  file=src/client/conn.rs
- id=7 category=stability kind=public_api_function_added subject=http10_disable_keep_alive source=stability confidence=high value=direct
  file=src/server/conn.rs
- id=8 category=stability kind=public_api_function_added subject=http10_disable_keep_alive source=stability confidence=high value=direct
  file=src/server/conn/http1.rs
- id=9 category=stability kind=public_api_function_added subject=http10_disable_keep_alive source=stability confidence=high value=direct
  file=src/server/server.rs
```

# Quality impact

```text
evidence=3 prod_files_changed=10 test_files_changed=2 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=10 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/client/client.rs
- id=11 category=quality kind=blast_radius subject=multiple crates or domains source=quality confidence=low value=context
  file=src/client/client.rs
- id=12 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=src/client/client.rs
```

# Semantic impact

```text
evidence=0
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
```

# History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=12
```
