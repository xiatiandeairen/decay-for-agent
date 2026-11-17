# Decay Evidence

- evidence_count: 12

## Top risk findings

```text
1. [high] src/proto/h1/dispatch.rs
   semantic drop_error_propagation_changed sender.send_error(crate::Error::new_incomplete());
   action: review drop-path incomplete error surfacing and require regression coverage for truncated body behavior
2. [medium] src/client/conn/http2.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
3. [medium] src/client/conn/http2.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
4. [medium] src/proto/h1/dispatch.rs:poll_read
   complexity touched_existing_risky
   action: review function complexity and add focused regression tests
5. [medium] src/client/conn/http2.rs
   quality prod_change_with_tests
   action: review changed behavior invariants and confirm regression tests cover the risky path
6. [medium] dependency manifests changed
   1 dependency files changed
   action: review dependency and feature changes for behavior/security impact
```

## Complexity impact

```text
changed_functions=14 evidence=1 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
- id=1 category=complexity kind=touched_existing_risky subject=function source=diff_hunk+ast_snapshot confidence=low value=context
  function=src/proto/h1/dispatch.rs:poll_read
  metrics=cognitive:62->62, cyclomatic:18->18, statements:35->35
```

## Coverage impact

```text
evidence=0
  value_note=scan coverage gaps limit decay confidence; this is tool coverage evidence, not proof of product code risk
```

## Stability impact

```text
evidence=0
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
```

## Quality impact

```text
evidence=4 prod_files_changed=5 test_files_changed=1 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=dependency_changed subject=dependency manifest source=quality confidence=low value=context
  file=Cargo.toml
- id=3 category=quality kind=prod_change_with_tests subject=production code changed with test changes source=quality confidence=low value=supporting
  file=src/client/conn/http2.rs
- id=4 category=quality kind=blast_radius subject=multiple crates or domains source=quality confidence=low value=context
  file=src/client/conn/http2.rs
- id=5 category=quality kind=mixed_concerns subject=multiple production domains source=quality confidence=low value=context
  file=src/client/conn/http2.rs
```

## Semantic impact

```text
evidence=1
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=6 category=semantic kind=drop_error_propagation_changed subject=sender.send_error(crate::Error::new_incomplete()); source=semantic_diff confidence=high value=direct
  file=src/proto/h1/dispatch.rs
```

## History impact

```text
evidence=6
  value_note=context only; excluded from top risk unless direct diff evidence exists
- id=7 category=history kind=churn_hotspot subject=4 commits in 90d, 70 lines changed in 180d source=git_history confidence=low value=context
  file=src/client/conn/http2.rs
- id=8 category=history kind=frequent_rework subject=2 commits in 30d, 5 commits in 180d source=git_history confidence=low value=context
  file=src/client/conn/http2.rs
- id=9 category=history kind=recently_bugfixed_area subject=docs(client): fix HTTP/2 max concurrent stream link to spec (#4037) source=git_history confidence=low value=context
  file=src/client/conn/http2.rs
- id=10 category=history kind=recently_bugfixed_area subject=fix(http1): fix rare missed write wakeup on connections v2 source=git_history confidence=low value=context
  file=src/proto/h1/dispatch.rs
- id=11 category=history kind=churn_hotspot subject=3 commits in 90d, 51 lines changed in 180d source=git_history confidence=low value=context
  file=src/proto/h2/client.rs
- id=12 category=history kind=recently_bugfixed_area subject=fix(http2): cancel sending client request body on response future drop (#4042) source=git_history confidence=low value=context
  file=src/proto/h2/client.rs
recorded=true changeset=1 evidence_saved=12
```
