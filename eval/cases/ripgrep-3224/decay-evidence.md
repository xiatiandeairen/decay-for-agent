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
```

## Stability impact

```text
evidence=0
```

## Quality impact

```text
evidence=1 prod_files_changed=1 test_files_changed=0 docs_files_changed=0
- id=1 category=quality kind=test_gap subject=production change without tests source=quality
  file=crates/cli/src/decompress.rs
```

## Semantic impact

```text
evidence=8
- id=2 category=semantic kind=default_value_changed subject=ARGS_GZIP: &["gzip", "-d", "-c"] -> &["gzip", "-d", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
- id=3 category=semantic kind=default_value_changed subject=ARGS_BZIP: &["bzip2", "-d", "-c"] -> &["bzip2", "-d", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
- id=4 category=semantic kind=default_value_changed subject=ARGS_XZ: &["xz", "-d", "-c"] -> &["xz", "-d", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
- id=5 category=semantic kind=default_value_changed subject=ARGS_LZ4: &["lz4", "-d", "-c"] -> &["lz4", "-d", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
- id=6 category=semantic kind=default_value_changed subject=ARGS_LZMA: &["xz", "--format=lzma", "-d", "-c"] -> &["xz", "--format=lzma", "-d", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
- id=7 category=semantic kind=default_value_changed subject=ARGS_BROTLI: &["brotli", "-d", "-c"] -> &["brotli", "-d", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
- id=8 category=semantic kind=default_value_changed subject=ARGS_ZSTD: &["zstd", "-q", "-d", "-c"] -> &["zstd", "-q", "-d", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
- id=9 category=semantic kind=default_value_changed subject=ARGS_UNCOMPRESS: &["uncompress", "-c"] -> &["uncompress", "-c", "--"] source=semantic_diff
  file=crates/cli/src/decompress.rs
```

## History impact

```text
evidence=0
recorded=true changeset=1 evidence_saved=9
```
