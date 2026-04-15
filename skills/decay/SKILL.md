---
name: decay
description: Use when user asks about project health, code quality trends, structural decay, complexity issues, or wants a health report. Also use after large refactors or sprint completions to assess impact.
---

# decay

`/decay` → Run project health check: scan files, analyze git history, score health, diagnose issues, generate prescriptions.

## When to Use

- User asks about project health, code quality, or structural issues
- After completing a refactor or sprint — assess impact
- User says "check health", "run decay", "项目健康", "代码质量"
- Before starting a large refactor — establish baseline

## Execution

Run the decay CLI in the current project directory:

```bash
# [RUN]
decay
```

If `decay` is not in PATH, try the project's build output:

```bash
# [RUN] fallback
cargo run --manifest-path {PLUGIN_DIR}/../../Cargo.toml 2>/dev/null || echo "decay CLI not found. Install with: cargo install --path {project_root}"
```

## Reading the Output

### Scores (0-100, higher = healthier)

| Dimension | Measures |
|-----------|----------|
| structural | File count, directory depth, top-level sprawl |
| complexity | Large file ratio, average/max file size |
| fragility | Git churn concentration, hotspot intensity |
| composite | Equal-weight average of all dimensions |

### Trend Arrows

| Symbol | Meaning |
|--------|---------|
| ↑N | Improved by N points since last snapshot |
| ↓N | Declined by N points |
| → | No change |

### Issue Levels

| Level | Action |
|-------|--------|
| CRITICAL | Must fix — structural risk |
| WARNING | Should fix — quality degrading |
| INFO | Monitor — not urgent |

### Actions (v4+)

Each issue may include structured `actions` — agent-consumable refactoring instructions:

| Field | Description |
|-------|-------------|
| `action_type` | split, extract, add, remove, replace, move, refactor |
| `target.file` | File or directory path |
| `target.line_range` | `[start, end]` line range (when available) |
| `target.symbol` | Function/module name (when available) |
| `priority` | critical > high > medium > low |
| `effort` | small (< 30min) / medium (30min-2hr) / large (> 2hr) |
| `reason` | Why this action is needed |

Top-level `actions` array is sorted by priority then effort (most urgent + cheapest first).

For JSON output with full action data, use:

```bash
# [RUN]
decay --json
```

## After Running

1. Present the health summary to the user
2. If critical issues exist, highlight the top actions from the `actions` array
3. If user just completed a refactor, compare with previous scores (trend arrows)
4. Do not automatically start fixing issues — wait for user direction
5. When user wants to fix issues, use the `actions` array to plan refactoring in priority order
