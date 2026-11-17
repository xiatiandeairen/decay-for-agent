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

1, 2, 3

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/BurntSushi-ripgrep-762/repo
  mode=git_ref
  base_input=35f802166d1c5788e8c833f398d13b9c9e8cb360
  base=35f802166d1c5788e8c833f398d13b9c9e8cb360
  head=ff30708d4dfa7ba9fc856d8f9fc4fed8010bb075
  changed_files=3
```

# Top risk findings

```text
1. [high] ignore/src/gitignore.rs:add_line
   complexity worsened
   action: review function complexity and add focused regression tests
2. [medium] ignore/src/gitignore.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
3. [medium] ignore/src/gitignore.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

# Complexity impact

```text
changed_functions=2 evidence=1 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=worsened subject=function source=diff_hunk+ast_snapshot
  function=ignore/src/gitignore.rs:add_line
  metrics=cognitive:23->24, cyclomatic:17->18, statements:37->36
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
evidence=2 prod_files_changed=2 test_files_changed=3 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality
  file=ignore/src/gitignore.rs
- id=3 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=ignore/src/gitignore.rs
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
recorded=true changeset=1 evidence_saved=3
```
