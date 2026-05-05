**English** · [中文](README.zh.md)

# decay

`decay` is a Rust function-level complexity regression detector for AI-assisted coding.

It currently answers one narrow question:

> Compared with a baseline, did this change make any function locally harder to maintain?

It is not a general code-quality platform, and it is not another complex-function leaderboard. v0.1.0 is still being dogfooded.

## Status

v0.1.0:

- Rust only
- dogfooding by the author
- no external users
- CLI output, SQLite schema, and thresholds may change without compatibility guarantees
- feature loop is implemented, but product value is not proven yet

## Quick Start

```bash
cd /path/to/your/rust/project

decay doctor             # diagnose current code risks; no baseline required; not a gate
decay baseline v1.0.0    # save the current tree as a named baseline
# ... edit code, or let an AI assistant edit code ...
decay diff v1.0.0        # compare current workspace against v1.0.0
decay baseline v1.1.0    # save a new named baseline
decay diff v1.0.0 v1.1.0 # compare two named baselines
```

Bare `decay` prints a concise command list. It does not scan or write storage.
Use `decay --help` for detailed options.

## Command Semantics

| Command | Purpose | Exit code |
|---|---|---|
| `decay` | concise command list | 0 |
| `decay doctor` | current-risk diagnosis | 0 unless runtime error |
| `decay baseline <version>` | save named baseline | 0 on success; 1 if same name differs without `--replace` |
| `decay diff <version>` | current workspace vs baseline | 0 clean; 1 degraded |
| `decay diff <from> <to>` | baseline vs baseline | 0 clean; 1 degraded |

`doctor` is a health check. `diff` is the commit-time regression judge.

## What Diff Reports

`decay diff` reports only regressions:

- newly added high-risk functions
- existing functions that crossed a risk boundary
- existing high-risk functions that got worse

It does not report deletions, improvements, unchanged functions, or below-threshold small increases.

Example:

```text
status=degraded from=v1.0.0 to=current degradations=2

[functions that crossed a risk boundary]
- src/store.rs:130 save_baseline
  problem=Function body grew beyond a focused size.
  change=Function size changed from 22 statements to 31 statements; recommended limit is 25 statements.
```

## Metrics

Active metrics:

| Metric | Threshold | Meaning |
|---|---:|---|
| `nesting` | 4 | maximum control-flow nesting depth |
| `cyclomatic` | 10 | McCabe branch complexity |
| `cognitive` | 15 | branch complexity weighted for reading burden |
| `params` | 5 | function parameter count |
| `statement_count` | 25 | executable steps inside the function |
| `max_condition_ops` | 4 | maximum boolean operators in one condition |

Threshold rule:

```text
value > threshold => breach
```

## Scope

Default `--scope prod` focuses on the primary maintained Rust source and filters common test/example/fixture noise.

Use a full view when needed:

```bash
decay doctor --scope all
decay diff v1.0.0 --scope all
```

`decay` reads root `.gitignore` and also supports `--exclude <pattern>`.

## Limits

- Rust only.
- No JSON output.
- No configurable thresholds.
- No semantic rename/move tracking; rename or move may look like delete + add.
- Closures are not tracked separately; their complexity rolls into the enclosing function.
- Per-file parse failure does not abort the scan, but the result is marked partial.
- No DB migration compatibility in v0.1.0.

## Docs

- [docs/roadmap.md](docs/roadmap.md): product direction and current status
- [docs/requirements/function-complexity-detection/prd.md](docs/requirements/function-complexity-detection/prd.md): v0.1.0 PRD
- [docs/arch/decay.md](docs/arch/decay.md): current architecture
- [docs/ops.md](docs/ops.md): dogfood / operations loop
- [docs/decision/v0.1.0-closeout.md](docs/decision/v0.1.0-closeout.md): v0.1.0 closeout decision

## License

Not yet specified. Treat as all rights reserved until a license file is added.
