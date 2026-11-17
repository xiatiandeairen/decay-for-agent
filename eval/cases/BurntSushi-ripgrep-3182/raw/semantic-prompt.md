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

1, 2, 3, 4, 5, 6

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/BurntSushi-ripgrep-3182/repo
  mode=git_ref
  base_input=916415857f084e63efda03cf25a67fe7f6d24244
  base=916415857f084e63efda03cf25a67fe7f6d24244
  head=1e80e6e8a4ba4126e04df36aa9558bf6cb6ce73c
  changed_files=4
```

# Top risk findings

```text
1. [high] crates/core/search.rs
   semantic error_propagation_changed cmd_builder.async_stderr(true);
   action: review caller-visible error propagation and require focused regression coverage
2. [medium] crates/core/search.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
3. [medium] crates/core/search.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
4. [medium] crates/printer/src/util.rs:replace_all
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
5. [medium] crates/printer/src/util.rs:replace_with_captures_in_context
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
6. [medium] crates/core/search.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

# Complexity impact

```text
changed_functions=7 evidence=2 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=crates/printer/src/util.rs:replace_all
  metrics=cognitive:9->9, cyclomatic:5->5, statements:16->16
- id=2 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=crates/printer/src/util.rs:replace_with_captures_in_context
  metrics=cognitive:4->6, cyclomatic:3->4, statements:11->11
```

# Coverage impact

```text
evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
```

# Stability impact

```text
evidence=0
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
```

# Quality impact

```text
evidence=3 prod_files_changed=2 test_files_changed=1 docs_files_changed=1
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=3 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=crates/core/search.rs
- id=4 category=quality kind=blast_radius subject=multiple crates or domains source=quality confidence=low value=context
  file=crates/core/search.rs
- id=5 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=crates/core/search.rs
```

# Semantic impact

```text
evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=6 category=semantic kind=error_propagation_changed subject=cmd_builder.async_stderr(true); source=semantic_diff confidence=medium value=direct
  file=crates/core/search.rs
```

# History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=6
```
