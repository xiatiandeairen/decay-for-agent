# Decay Evidence

- evidence_count: 2

## Top risk findings

```text
1. [medium] crates/core/search.rs
   quality blast_radius
   action: split the diff or document the shared review rationale
2. [medium] crates/core/search.rs
   quality mixed_concerns
   action: split the diff or document the shared review rationale
```

## Complexity impact

```text
changed_functions=4 evidence=0 partial=false diagnostics=0
```

## Stability impact

```text
evidence=0
```

## Quality impact

```text
evidence=2 prod_files_changed=3 test_files_changed=1 docs_files_changed=0
- id=1 category=quality kind=blast_radius subject=multiple crates or domains source=quality
  file=crates/core/search.rs
- id=2 category=quality kind=mixed_concerns subject=multiple production domains source=quality
  file=crates/core/search.rs
```

## Semantic impact

```text
evidence=0
```

## History impact

```text
evidence=0
recorded=true changeset=1 evidence_saved=2
```
