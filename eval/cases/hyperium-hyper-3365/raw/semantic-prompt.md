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

1, 2

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/hyperium-hyper-3365/repo
  mode=git_ref
  base_input=41eaf2042b8169d3dd067d49cfdbdaaf36678903
  base=41eaf2042b8169d3dd067d49cfdbdaaf36678903
  head=cc3f25188d3f2b89b25c55c8c1cd76c05f272ba9
  changed_files=1
```

# Top risk findings

```text
1. [high] src/client/pool.rs
   semantic boundary_expression_changed timeout: config.idle_timeout.filter(|&t| t > Duration::ZERO)
   action: review changed boundary expression and require focused edge-case tests
2. [medium] src/client/pool.rs
   quality test_gap
   action: add focused tests or document why existing coverage is enough
```

# Complexity impact

```text
changed_functions=1 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
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
evidence=1 prod_files_changed=1 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=src/client/pool.rs
```

# Semantic impact

```text
evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=boundary_expression_changed subject=timeout: config.idle_timeout.filter(|&t| t > Duration::ZERO) source=semantic_diff confidence=high value=direct
  file=src/client/pool.rs
```

# History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=2
```
