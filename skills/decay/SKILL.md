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

## After Running

1. Present the health summary to the user
2. If critical issues exist, suggest addressing them
3. If user just completed a refactor, compare with previous scores (trend arrows)
4. Do not automatically start fixing issues — wait for user direction
