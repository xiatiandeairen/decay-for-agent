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
repo_root=/private/tmp/decay-pr-eval-v3/hyper-3621/repo
  mode=git_ref
  base_input=b79be911696f6a93e8d408080ebbf558b612ce3c
  base=b79be911696f6a93e8d408080ebbf558b612ce3c
  head=2b3cc9fa30540555f22665c7ef9b1d61e55449d9
  changed_files=8
```

# Top risk findings

```text
1. [high] examples/client.rs
   protocol host header authority changed Host header authority -> host/port
   action: review protocol header semantics, credential leakage, and compatibility tests
2. [high] examples/client_json.rs
   protocol host header authority changed Host header authority -> host/port
   action: review protocol header semantics, credential leakage, and compatibility tests
3. [high] examples/single_threaded.rs
   protocol host header authority changed Host header authority -> host/port
   action: review protocol header semantics, credential leakage, and compatibility tests
4. [high] src/server/conn/http1.rs
   1 panic-like additions
   action: review runtime failure and boundary-safety invariants
5. [high] src/proto/h2/server.rs
   semantic default_value_changed DEFAULT_SETTINGS_MAX_HEADER_LIST_SIZE: 1024 * 16; // 16kb -> 16 << 20
   action: review behavior boundary change and add regression coverage for old and new defaults
6. [medium] src/client/conn/http2.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
7. [medium] src/client/conn/http2.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
8. [medium] src/client/conn/http2.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
```

# Complexity impact

```text
changed_functions=4 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
```

# Coverage impact

```text
evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
```

# Stability impact

```text
evidence=1
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=1 category=stability kind=panic_like_added subject=line 504 source=stability confidence=high value=direct
  file=src/server/conn/http1.rs
```

# Quality impact

```text
evidence=3 prod_files_changed=5 test_files_changed=3 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/client/conn/http2.rs
- id=3 category=quality kind=blast_radius subject=multiple crates or domains source=quality confidence=low value=context
  file=src/client/conn/http2.rs
- id=4 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=src/client/conn/http2.rs
```

# Semantic impact

```text
evidence=4
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=5 category=semantic kind=protocol_header_authority_changed subject=Host header authority -> host/port source=semantic_diff confidence=high value=direct
  file=examples/client.rs
- id=6 category=semantic kind=protocol_header_authority_changed subject=Host header authority -> host/port source=semantic_diff confidence=high value=direct
  file=examples/client_json.rs
- id=7 category=semantic kind=protocol_header_authority_changed subject=Host header authority -> host/port source=semantic_diff confidence=high value=direct
  file=examples/single_threaded.rs
- id=8 category=semantic kind=default_value_changed subject=DEFAULT_SETTINGS_MAX_HEADER_LIST_SIZE: 1024 * 16; // 16kb -> 16 << 20 source=semantic_diff confidence=medium value=direct
  file=src/proto/h2/server.rs
```

# History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=8
```
