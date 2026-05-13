# Decay Evidence

- evidence_count: 2

## Top risk findings

```text
1. [high] src/body/incoming/channel.rs
   1 panic-like additions
   action: review runtime failure and boundary-safety invariants
2. [medium] src/body/incoming.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
```

## Complexity impact

```text
changed_functions=61 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
```

## Stability impact

```text
evidence=1
  value_note=direct for newly introduced unsafe/panic/public-api evidence; move/refactor diffs can weaken this signal
- id=1 category=stability kind=panic_like_added subject=line 158 source=stability
  file=src/body/incoming/channel.rs
```

## Quality impact

```text
evidence=1 prod_files_changed=5 test_files_changed=3 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=2 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=src/body/incoming.rs
```

## Semantic impact

```text
evidence=0
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
```

## History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=2
```
