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

1, 2, 3, 4, 5, 6, 7, 8, 9

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/ripgrep-3189/repo
  mode=git_ref
  base_input=9ec08522bee566c8fbf03c340acd7a1ff1789a9f
  base=9ec08522bee566c8fbf03c340acd7a1ff1789a9f
  head=12b81e5ba3468269d60521902db3e6fd5b3d3826
  changed_files=7
```

# Top risk findings

```text
1. [high] crates/ignore/src/walk.rs
   1 panic-like additions + 1 public API function added
   action: review runtime failure and boundary-safety invariants
2. [high] crates/ignore/src/walk.rs:build
   complexity worsened
   action: review function complexity and add focused regression tests
3. [medium] crates/core/flags/hiargs.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
4. [medium] crates/core/flags/hiargs.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
5. [medium] crates/core/flags/hiargs.rs:from_low_args
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
6. [medium] crates/core/flags/hiargs.rs:walk_builder
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
7. [medium] crates/ignore/src/dir.rs:add_child_path
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
8. [medium] crates/core/flags/hiargs.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

# Complexity impact

```text
changed_functions=16 evidence=4 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=crates/core/flags/hiargs.rs:from_low_args
  metrics=cognitive:55->55, cyclomatic:38->38, statements:42->42
- id=2 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=crates/core/flags/hiargs.rs:walk_builder
  metrics=cognitive:17->17, cyclomatic:11->11, statements:14->14
- id=3 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=crates/ignore/src/dir.rs:add_child_path
  metrics=cognitive:18->18, cyclomatic:10->10, statements:18->18
- id=4 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot confidence=high value=direct
  function=crates/ignore/src/walk.rs:build
  metrics=cognitive:27->29, cyclomatic:7->7, statements:18->18
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
- id=5 category=stability kind=panic_like_added subject=line 997 source=stability confidence=high value=direct
  file=crates/ignore/src/walk.rs
- id=6 category=stability kind=public_api_function_added subject=current_dir source=stability confidence=high value=direct
  file=crates/ignore/src/walk.rs
```

# Quality impact

```text
evidence=3 prod_files_changed=5 test_files_changed=1 docs_files_changed=1
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=7 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=crates/core/flags/hiargs.rs
- id=8 category=quality kind=blast_radius subject=multiple crates or domains source=quality confidence=low value=context
  file=crates/core/flags/hiargs.rs
- id=9 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=crates/core/flags/hiargs.rs
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
recorded=true changeset=1 evidence_saved=9
```
