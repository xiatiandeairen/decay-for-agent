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

1, 2, 3, 4, 5, 6, 7

# Diff context

```text
repo_root=/private/tmp/decay-pr-eval/hyper-4042/repo
  mode=git_ref
  base_input=d51cb71569bbca7927b3828ef11e3bec4fa97eb4
  base=d51cb71569bbca7927b3828ef11e3bec4fa97eb4
  head=4c38fdeab1dfacdca78e005dea8c51d2938db626
  changed_files=4
```

# Top risk findings

```text
1. [high] src/proto/h2/client.rs
   semantic lifecycle_ordering_changed this.pipe.as_mut().send_reset(h2::Reason::CANCEL);
   action: review shutdown/upgrade/cancel ordering and require regression coverage for cleanup or terminal-path behavior
2. [high] src/proto/h2/client.rs
   semantic boundary_expression_changed let cancel_result = this.cancel_rx.as_mut().map(|rx| Pin::new(rx).poll(cx));
   action: review changed boundary expression and require focused edge-case tests
3. [high] src/proto/h2/client.rs
   semantic boundary_expression_changed Some(Poll::Pending) | None => {}
   action: review changed boundary expression and require focused edge-case tests
4. [high] src/proto/h2/client.rs
   semantic boundary_expression_changed pub(crate) fn cancel(self: Pin<&mut Self>) {
   action: review changed boundary expression and require focused edge-case tests
5. [medium] src/client/dispatch.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
6. [medium] src/client/dispatch.rs
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
evidence=2 prod_files_changed=3 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/client/dispatch.rs
- id=2 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=src/client/dispatch.rs
```

# Semantic impact

```text
evidence=4
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=3 category=semantic kind=boundary_expression_changed subject=let cancel_result = this.cancel_rx.as_mut().map(|rx| Pin::new(rx).poll(cx)); source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs
- id=4 category=semantic kind=boundary_expression_changed subject=Some(Poll::Pending) | None => {} source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs
- id=5 category=semantic kind=boundary_expression_changed subject=pub(crate) fn cancel(self: Pin<&mut Self>) { source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs
- id=6 category=semantic kind=lifecycle_ordering_changed subject=this.pipe.as_mut().send_reset(h2::Reason::CANCEL); source=semantic_diff confidence=high value=direct
  file=src/proto/h2/client.rs
```

# History impact

```text
evidence=1
  value_note=context only; excluded from top risk unless direct diff evidence exists
- id=7 category=history kind=recently_bugfixed_area subject=fix(http2): non-utf8 char in Connection header may cause panic when calling to_str (#4019) source=git_history confidence=low value=context
  file=src/proto/h2/mod.rs
recorded=true changeset=1 evidence_saved=7
```
