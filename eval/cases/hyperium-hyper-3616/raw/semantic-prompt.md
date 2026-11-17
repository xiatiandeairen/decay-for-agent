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
repo_root=/private/tmp/decay-pr-eval/hyperium-hyper-3616/repo
  mode=git_ref
  base_input=bc9a86f58f8bd5c35b2bfd7e632ec132280d79ba
  base=bc9a86f58f8bd5c35b2bfd7e632ec132280d79ba
  head=ff109f584d9d280a93fce29047c90c7088e2c6f8
  changed_files=2
```

# Top risk findings

```text
1. [high] src/server/conn/http1.rs
   semantic lifecycle_ordering_changed Pin::new(conn).graceful_shutdown()
   action: review shutdown/upgrade/cancel ordering and require regression coverage for cleanup or terminal-path behavior
2. [medium] src/server/conn/http1.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
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
evidence=1 prod_files_changed=1 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/server/conn/http1.rs
```

# Semantic impact

```text
evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=lifecycle_ordering_changed subject=Pin::new(conn).graceful_shutdown() source=semantic_diff confidence=high value=direct
  file=src/server/conn/http1.rs
```

# History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=2
```
