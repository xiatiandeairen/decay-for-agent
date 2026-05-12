# Decay Evidence

- evidence_count: 4

## Top risk findings

```text
1. [high] src/server/conn/http1.rs
   1 panic-like additions
   action: review runtime failure and boundary-safety invariants
2. [high] src/proto/h2/server.rs
   semantic default_value_changed DEFAULT_SETTINGS_MAX_HEADER_LIST_SIZE: 1024 * 16; // 16kb -> 16 << 20
   action: review behavior boundary change and add regression coverage for old and new defaults
3. [medium] src/client/conn/http2.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
4. [medium] src/client/conn/http2.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
```

## Complexity impact

```text
changed_functions=4 evidence=0 partial=false diagnostics=0
```

## Stability impact

```text
evidence=1
- id=1 category=stability kind=panic_like_added subject=line 504 source=stability
  file=src/server/conn/http1.rs
```

## Quality impact

```text
evidence=2 prod_files_changed=5 test_files_changed=3 docs_files_changed=0
- id=2 category=quality kind=blast_radius subject=multiple crates or domains source=quality
  file=src/client/conn/http2.rs
- id=3 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=src/client/conn/http2.rs
```

## Semantic impact

```text
evidence=1
- id=4 category=semantic kind=default_value_changed subject=DEFAULT_SETTINGS_MAX_HEADER_LIST_SIZE: 1024 * 16; // 16kb -> 16 << 20 source=semantic_diff
  file=src/proto/h2/server.rs
```

## History impact

```text
evidence=0
recorded=true changeset=1 evidence_saved=4
```
