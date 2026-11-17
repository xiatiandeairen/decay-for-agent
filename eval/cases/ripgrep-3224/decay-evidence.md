# Decay Evidence

- evidence_count: 9

## Top risk findings

```text
1. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_GZIP: &["gzip", "-d", "-c"] -> &["gzip", "-d", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
2. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_BZIP: &["bzip2", "-d", "-c"] -> &["bzip2", "-d", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
3. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_XZ: &["xz", "-d", "-c"] -> &["xz", "-d", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
4. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_LZ4: &["lz4", "-d", "-c"] -> &["lz4", "-d", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
5. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_LZMA: &["xz", "--format=lzma", "-d", "-c"] -> &["xz", "--format=lzma", "-d", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
6. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_BROTLI: &["brotli", "-d", "-c"] -> &["brotli", "-d", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
7. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_ZSTD: &["zstd", "-q", "-d", "-c"] -> &["zstd", "-q", "-d", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
8. [high] crates/cli/src/decompress.rs
   semantic default_value_changed ARGS_UNCOMPRESS: &["uncompress", "-c"] -> &["uncompress", "-c", "--"]
   action: review behavior boundary change and add regression coverage for old and new defaults
9. [medium] crates/cli/src/decompress.rs
   quality test_gap
   action: add focused tests or document why existing coverage is enough
```

## Complexity impact

```text
changed_functions=1 evidence=0 partial=false diagnostics=0
  value_note=direct for worsened/new_high_risk; touched_existing_risky is supporting context, not proof of current-diff risk
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
evidence=1 prod_files_changed=1 test_files_changed=0 docs_files_changed=0
  value_note=missing-test signals are supporting evidence; broad scope signals are weak unless paired with direct diff evidence
- id=1 category=quality kind=test_gap subject=production change without tests source=quality confidence=low value=supporting
  file=crates/cli/src/decompress.rs
```

## Semantic impact

```text
evidence=8
  value_note=high confidence only for explicit scanned patterns; this is not general semantic understanding
- id=2 category=semantic kind=default_value_changed subject=ARGS_GZIP: &["gzip", "-d", "-c"] -> &["gzip", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
- id=3 category=semantic kind=default_value_changed subject=ARGS_BZIP: &["bzip2", "-d", "-c"] -> &["bzip2", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
- id=4 category=semantic kind=default_value_changed subject=ARGS_XZ: &["xz", "-d", "-c"] -> &["xz", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
- id=5 category=semantic kind=default_value_changed subject=ARGS_LZ4: &["lz4", "-d", "-c"] -> &["lz4", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
- id=6 category=semantic kind=default_value_changed subject=ARGS_LZMA: &["xz", "--format=lzma", "-d", "-c"] -> &["xz", "--format=lzma", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
- id=7 category=semantic kind=default_value_changed subject=ARGS_BROTLI: &["brotli", "-d", "-c"] -> &["brotli", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
- id=8 category=semantic kind=default_value_changed subject=ARGS_ZSTD: &["zstd", "-q", "-d", "-c"] -> &["zstd", "-q", "-d", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
- id=9 category=semantic kind=default_value_changed subject=ARGS_UNCOMPRESS: &["uncompress", "-c"] -> &["uncompress", "-c", "--"] source=semantic_diff confidence=medium value=direct
  file=crates/cli/src/decompress.rs
```

## History impact

```text
evidence=0
  value_note=context only; excluded from top risk unless direct diff evidence exists
recorded=true changeset=1 evidence_saved=9
```
