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

1, 2, 3, 4, 5, 6, 7, 8

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/hyperium-hyper-2544/repo
  mode=git_ref
  base_input=6a6a24030ec53321ce026fe294c56c86f21e19d4
  base=6a6a24030ec53321ce026fe294c56c86f21e19d4
  head=093319d3e21f5ae3d4c5690466aec7bd9de4cb0c
  changed_files=2
```

# Top risk findings

```text
1. [high] src/error.rs
   quality sensitive_test_gap
   action: add or justify missing tests for sensitive code
2. [high] src/proto/h1/role.rs
   2 panic-like additions
   action: review runtime failure and boundary-safety invariants
3. [high] src/error.rs:description
   complexity worsened
   action: review function complexity and add focused regression tests
4. [high] src/proto/h1/role.rs
   semantic error_propagation_changed Err(e) => maybe_panic!(e)
   action: review caller-visible error propagation and require focused regression coverage
5. [medium] src/error.rs
   quality test_gap
   action: add focused tests or document why existing coverage is enough
6. [medium] src/error.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
7. [medium] src/proto/h1/role.rs:parse
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
```

# Complexity impact

```text
changed_functions=2 evidence=2 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot confidence=high value=direct
  function=src/error.rs:description
  metrics=cognitive:31->32, cyclomatic:31->32, statements:1->1
- id=2 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=src/proto/h1/role.rs:parse
  metrics=cognitive:40->40, cyclomatic:20->20, statements:51->51
```

# Coverage impact

```text
evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
```

# Stability impact

```text
evidence=2
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=3 category=stability kind=panic_like_added subject=line 35 source=stability confidence=high value=direct
  file=src/proto/h1/role.rs
- id=4 category=stability kind=panic_like_added subject=line 53 source=stability confidence=high value=direct
  file=src/proto/h1/role.rs
```

# Quality impact

```text
evidence=3 prod_files_changed=2 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=5 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=src/error.rs
- id=6 category=quality kind=sensitive_test_gap subject=sensitive production change without tests source=quality confidence=medium value=supporting
  file=src/error.rs
- id=7 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=src/error.rs
```

# Semantic impact

```text
evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=8 category=semantic kind=error_propagation_changed subject=Err(e) => maybe_panic!(e) source=semantic_diff confidence=medium value=direct
  file=src/proto/h1/role.rs
```

# History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=8
```
