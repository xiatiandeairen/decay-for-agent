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
repo_root=/private/tmp/decay-pr-eval/hyperium-hyper-647/repo
  mode=git_ref
  base_input=32e09a04292b0247456a8fb9003a75a6abaa998e
  base=32e09a04292b0247456a8fb9003a75a6abaa998e
  head=a36e44af7d4e665a122c1498011ff10035f7376f
  changed_files=1
```

# Top risk findings

```text
1. [medium] src/http/h1.rs
   coverage parse_coverage_gap
   action: treat decay output as partial and inspect this diff manually or update parser coverage
2. [medium] src/http/h1.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

# Complexity impact

```text
changed_functions=0 evidence=0 partial=true diagnostics=2
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
```

# Coverage impact

```text
evidence=1
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
- id=1 category=coverage kind=parse_coverage_gap subject=decay scan coverage is partial source=scan confidence=high value=coverage_limit
  file=src/http/h1.rs
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
- id=2 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/http/h1.rs
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
diagnostic=parse error in src/http/h1.rs: source contains syntax errors
diagnostic=parse error in src/http/h1.rs: source contains syntax errors
recorded=true changeset=1 evidence_saved=2
```
