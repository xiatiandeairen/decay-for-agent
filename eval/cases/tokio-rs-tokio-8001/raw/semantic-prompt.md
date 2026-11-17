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

1, 2, 3, 4

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/tokio-rs-tokio-8001/repo
  mode=git_ref
  base_input=654d38b13228a13498e793d8bb4f6ba50fd1016a
  base=654d38b13228a13498e793d8bb4f6ba50fd1016a
  head=dd0325e3f567581aecb7ac97ad73d35a35987c59
  changed_files=2
```

# Top risk findings

```text
1. [high] tokio/src/net/udp.rs
   semantic readiness_error_propagation_changed .async_io(Interest::READABLE | Interest::ERROR, || self.io.recv(buf))
   action: review error-path readiness registration and require regression coverage for surfaced socket errors
2. [high] tokio/src/net/udp.rs
   semantic readiness_error_propagation_changed .async_io(Interest::READABLE | Interest::ERROR, || self.io.peek(buf))
   action: review error-path readiness registration and require regression coverage for surfaced socket errors
3. [medium] tokio/src/net/udp.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

# Complexity impact

```text
changed_functions=5 evidence=0 partial=false diagnostics=0
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
  file=tokio/src/net/udp.rs
```

# Semantic impact

```text
evidence=2
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=readiness_error_propagation_changed subject=.async_io(Interest::READABLE | Interest::ERROR, || self.io.recv(buf)) source=semantic_diff confidence=high value=direct
  file=tokio/src/net/udp.rs
- id=3 category=semantic kind=readiness_error_propagation_changed subject=.async_io(Interest::READABLE | Interest::ERROR, || self.io.peek(buf)) source=semantic_diff confidence=high value=direct
  file=tokio/src/net/udp.rs
```

# History impact

```text
evidence=1
  value_note=context only; excluded from top risk unless direct diff evidence exists
- id=4 category=history kind=churn_hotspot subject=1 commits in 90d, 187 lines changed in 180d source=git_history confidence=low value=context
  file=tokio/src/net/udp.rs
recorded=true changeset=1 evidence_saved=4
```
