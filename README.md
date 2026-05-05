**English** · [中文](README.zh.md)

# decay

A function-level complexity regression detector for projects written with AI assistance.

> **Status: v0.1 — Rust only, actively dogfooding, expect rough edges.**
> No external users yet. Interfaces, thresholds, and output format may change.

## Why

AI coding assistants tend to fix bugs by adding another `if`. The problem: when you ask the same assistant whether the code got worse, it has a structural bias toward saying no (sycophancy). You end up with slow, invisible complexity creep.

`decay` is a small, opinionated outsider:

- It does not generate code, so it has no stake in defending what was written.
- It looks at **delta**, not absolute values. `lizard`, Clippy, and ESLint complexity rules tell you the function is complex right now. `decay` tells you that *this change* made it worse.
- It works at function granularity, persisted across snapshots, so slow regressions across many sessions are visible.

## What it does

Run `decay` in a Rust project. It parses every `.rs` file with tree-sitter, computes four metrics per function, and stores a snapshot. Run it again later and `decay diff` shows you which functions regressed.

Real output from running `decay` on this repo during development — the tool flagged three functions the AI had just written without realizing they had crossed thresholds:

```
decay v0.1.0
Scanned 225 files, 896 functions in 0.27s
Snapshot #2 saved

34 functions exceed threshold:

  src/cli/diff_cmd.rs:92  collect_metric_lines
    cognitive: 23 ⚠ (>15)

  src/cli/scan.rs:92  print_exceeded
    cognitive: 16 ⚠ (>15)

  src/metric/cognitive.rs:131  score_match
    nesting: 5 ⚠ (>4)
  ...
```

These three were genuine regressions written by AI subagents during v0.1 implementation, not caught in review, surfaced by the tool on its first self-scan. They have since been refactored.

## Install

From source (no crates.io release yet):

```bash
git clone <this repo>
cd decay
cargo install --path .
```

## Quick start

```bash
cd /path/to/your/rust/project

decay         # scan, save snapshot, list functions over threshold
# ... edit code, let an AI assistant edit code, etc. ...
decay         # take another snapshot
decay diff    # compare against previous snapshot
```

`decay diff` reports a function only when it is genuinely worse: newly added and over threshold, newly crossed a threshold, or already over and got worse. Drops and unchanged functions are silent.

## Status & limits

This is an honest list. Read it before relying on the output.

- **Rust only.** No TypeScript, Python, or anything else. Multi-language is on the roadmap, not implemented.
- **Function rename/move is reported as `delete + add`.** The fingerprint is `xxh3(file + name + param_types)`, so renaming or moving a function across files breaks tracking.
- **Closures are not tracked separately.** Their complexity rolls into the enclosing function. A long `query_map` closure will inflate the outer function's score.
- **Exit code does not distinguish "regressed" from "clean."** Both return 0. Agent integration that gates on exit code is not reliable yet.
- **`.gitignore` is not read.** Only `target/` and `.git/` are excluded. You may need to clean build artifacts or vendored copies before scanning.
- **Thresholds are hard-coded** (`nesting 4`, `cyclomatic 10`, `cognitive 15`, `params 5`). Not configurable in v0.1.
- **No external validation yet.** The author is the only user. The thresholds and the cognitive complexity formula's behavior on idiomatic Rust (`?` chains, match arms) are calibrated against intuition, not a corpus.
- **Same name + same params across different `impl` blocks share a fingerprint.** Rare in practice, accepted for v0.1.

If any of these blocks your use case, `decay` is not ready for you yet.

## How it works

- **Parse** — tree-sitter-rust extracts every `function_item` (including `impl` methods and trait default implementations). Function signatures without bodies, closures, and macro-generated functions are skipped.
- **Measure** — four metrics per function:
  - **Nesting** — maximum block depth.
  - **Cyclomatic** — McCabe (branches + 1).
  - **Cognitive** — SonarSource formula, with a nesting bonus so deeply nested branches weigh more than shallow ones.
  - **Params** — signature arity.
- **Fingerprint** — `xxh3_64(file ⊕ name ⊕ param_types)`, with parameter types normalized (lifetimes stripped, whitespace removed). Stable across processes.
- **Persist** — SQLite at `dirs::data_dir()/decay/snapshots.db`. Two tables: `snapshots`, `functions`.
- **Diff** — align two snapshots by fingerprint, classify each function as `Added`, `CrossedThreshold`, or `Worsened`, sort by `max(value − threshold)` descending.

Details and rationale: [`docs/plans/v0.1.md`](docs/plans/v0.1.md), [`docs/audit.md`](docs/audit.md).

## License

Not yet specified. Treat as all rights reserved until a license file is added.
