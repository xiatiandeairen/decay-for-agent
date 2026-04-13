<p align="center">
  <h1 align="center">decay-for-agent</h1>
  <p align="center">
    Project decay prevention — multi-dimensional health scoring, trend tracking, and rule-based refactoring for AI agents.
  </p>
</p>

<p align="center">
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#contributing">Contributing</a>
</p>

<p align="center">
  <a href="https://github.com/xiatiandeairen/decay-for-agent/actions/workflows/ci.yml"><img src="https://github.com/xiatiandeairen/decay-for-agent/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/xiatiandeairen/decay-for-agent/releases"><img src="https://img.shields.io/github/v/release/xiatiandeairen/decay-for-agent?include_prereleases" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
</p>

---

## What is this?

**decay-for-agent** is a [Claude Code plugin](https://docs.anthropic.com/en/docs/claude-code) that gives AI agents structured project health monitoring. Instead of ad-hoc code reviews, Decay tracks measurable decay signals over time and generates actionable refactoring prescriptions.

**The problem:** Projects accumulate structural debt silently — files grow too large, modules become too coupled, complexity creeps up. By the time it's obvious, cleanup is expensive.

**The solution:** Decay snapshots project metrics into SQLite on every sprint, scores health across multiple dimensions (0–100), detects trends, runs rule checks, and outputs refactoring plans that can be fed directly to `/sprint`.

### Key Features

- **Multi-dimensional scoring** — Composite health score across structural, complexity, coupling, and size dimensions
- **Trend tracking** — SQLite-backed history with before/after comparison across snapshots
- **Rule engine** — Configurable rules (`max_file_lines`, `max_module_imports`, etc.) with pass/warn/fail output
- **Refactoring prescriptions** — Diagnosed problems become structured refactoring plans in terminal, Markdown, or sprint format
- **PostToolUse hooks** — Optionally auto-snapshot after file-modifying tool calls

## Installation

### As a Claude Code plugin (recommended)

Clone into your project's plugins directory:

```bash
# Navigate to your project
cd your-project

# Add as a git submodule (recommended)
git submodule add https://github.com/xiatiandeairen/decay-for-agent.git src/plugins/decay

# Or clone directly
git clone https://github.com/xiatiandeairen/decay-for-agent.git src/plugins/decay
```

Register in your `.claude/settings.json`:

```json
{
  "plugins": ["src/plugins/decay"]
}
```

### Verify installation

Once installed, the following slash command becomes available in Claude Code:

```
/decay    — Project health check: score, diagnose, trend, prescribe
```

## Quick Start

```
> /decay

# Decay snapshots the project and shows trend:
#   structural_score:  78  (↓ from 82)
#   complexity_score:  65  (↓ from 71)
#   coupling_score:    81  (stable)
#   composite:         74  ⚠️  warn
#
# Top issues:
#   - AuthService.swift: 923 lines (> 800 threshold) [FAIL]
#   - DataEngine.swift:  612 lines (> 500 threshold) [WARN]
#   - avg module imports: 14.2 (> 12 threshold)     [WARN]
```

Run with a specific subcommand:

```bash
ENTRY="src/plugins/decay/scripts/entry.sh"

bash $ENTRY snapshot          # Collect and store a new snapshot
bash $ENTRY score             # Show composite + dimension scores (0–100)
bash $ENTRY diagnose          # Problem list with improvement suggestions
bash $ENTRY prescribe         # Refactoring plan (terminal summary)
bash $ENTRY prescribe --markdown  # Full Markdown report
bash $ENTRY prescribe --sprint    # Sprint-ready task description
bash $ENTRY trend             # CLI trend table across snapshots
bash $ENTRY report            # Write collab/reports/decay-{date}.md
bash $ENTRY rules             # Run all rule checks (pass/warn/fail)
bash $ENTRY status            # Latest snapshot summary
```

Pipe the prescription into `/sprint`:

```
> /sprint $(bash src/plugins/decay/scripts/entry.sh prescribe --sprint)
```

## Architecture

```
decay-for-agent/
├── .claude-plugin/
│   └── plugin.json            # Plugin metadata (name, version, keywords)
├── config/
│   ├── default.toml           # Default thresholds and rule configuration
│   └── schema.sql             # SQLite schema (snapshots, metrics, trends)
├── decay/                     # Python core
│   ├── snapshot.py            # Metric collection (file sizes, imports, complexity)
│   ├── scorer.py              # Dimension scoring (0–100 per axis)
│   ├── trend.py               # Cross-snapshot trend analysis
│   ├── diagnoser.py           # Problem detection and prioritization
│   ├── prescriber.py          # Refactoring plan generation
│   ├── report.py              # Markdown report writer
│   ├── db.py                  # SQLite persistence layer
│   ├── config.py              # Config loading (default.toml + project overrides)
│   ├── output.py              # Terminal formatting helpers
│   └── rules/
│       ├── engine.py          # Rule base class and RuleResult type
│       └── builtin.py         # Built-in rules (max_file_lines, max_module_imports, …)
├── hooks/
│   └── hooks.json             # PostToolUse hook for auto-snapshot
├── scripts/
│   ├── entry.sh               # CLI entry point — routes subcommands
│   └── collect.sh             # Shell-level metric collection helpers
├── skills/
│   └── decay/SKILL.md         # /decay skill definition for Claude Code
└── tests/
    └── decay-test.sh          # Integration test suite
```

### Data Flow

```
Project source files
        │
        ▼
┌──────────────┐   snapshot   ┌─────────────────────┐
│  collect.sh  │─────────────▶│  SQLite (snapshots,  │
│  + snapshot  │              │  metrics, trends)    │
└──────────────┘              └──────────┬──────────┘
                                         │
          ┌──────────────────────────────┼──────────────────────────────┐
          ▼                              ▼                              ▼
   ┌────────────┐               ┌──────────────┐              ┌──────────────┐
   │  scorer    │               │   diagnoser  │              │    trend     │
   │  0–100     │               │  issues list │              │  comparison  │
   └────────────┘               └──────┬───────┘              └──────────────┘
                                        │
                                        ▼
                               ┌──────────────────┐
                               │   prescriber     │
                               │  terminal /      │
                               │  markdown /      │
                               │  sprint format   │
                               └──────────────────┘
```

### Scoring Dimensions

| Dimension | What it measures |
|-----------|-----------------|
| `structural` | File size distribution, line count outliers |
| `complexity` | Cyclomatic complexity, nesting depth |
| `coupling` | Module import counts, dependency fan-in/out |
| `size` | Overall project growth rate |

Score range: 0 (critical) → 100 (healthy). Composite is a weighted average.

### Rule Severity

| Status | Meaning |
|--------|---------|
| `pass` | Within configured threshold |
| `warn` | Exceeds soft threshold — monitor |
| `fail` | Exceeds hard threshold — action required |

Rules are configurable in `config/default.toml` and can be overridden per project.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Make your changes
4. Run the test suite: `bash tests/decay-test.sh`
5. Submit a pull request

### Development

```bash
# Run a snapshot against the current project
bash scripts/entry.sh snapshot

# Run all rule checks
bash scripts/entry.sh rules

# Run integration tests
bash tests/decay-test.sh
```

## License

[MIT](LICENSE)
