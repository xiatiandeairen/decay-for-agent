# decay

Project health monitoring for AI agents.

## What is this?

Projects accumulate structural debt silently — files grow too large, modules become too coupled, complexity creeps up. By the time it's obvious, cleanup is expensive.

**decay** snapshots project metrics into SQLite, scores health across three dimensions (0–100), tracks trends over time, and generates actionable refactoring prescriptions.

## Quick Start

```bash
cargo install --path .

decay          # Full health check
decay --json   # Machine-readable output
decay --debug  # Verbose logging
```

## Example Output

```
Scanned: 42 files, 12 dirs, max depth 3
Git: 28 commits, 15 files changed (last 90 days)
Health: 88/100 (↑3) structural: 95 (→) complexity: 90 (↑5) fragility: 80 (↑5)
Issues (1 warning):
  [WARNING] fragility: top 10% files account for 55% of churn — distribute changes across more files
Snapshot #4 created for /path/to/project
```

## Features

| Feature | Description |
|---------|-------------|
| File scanning | Traverse project tree, collect file count / depth / size |
| Git analysis | Analyze 90-day commit history, identify churn hotspots |
| Three-dimension scoring | structural (0-100) + complexity (0-100) + fragility (0-100) |
| Composite score | Weighted average health score |
| Trend tracking | Compare with previous snapshot, show ↑↓→ arrows |
| Diagnosis | Identify specific issues with severity levels (critical/warning/info) |
| Prescriptions | Actionable refactoring suggestions for each issue |
| JSON output | `--json` for programmatic consumption |
| Debug logging | `--debug` for internal flow visibility |

## Scoring

Deduction-based scoring (100 = healthy, deduct points for violations):

| Dimension | Measures | Key thresholds |
|-----------|----------|---------------|
| structural | File count, directory depth, top-level sprawl | >500 files, depth >5, >15 top dirs |
| complexity | Large file ratio, average/max file size | >15KB files, avg >10KB, max >50KB |
| fragility | Churn concentration, hotspot intensity | top 10% >50% churn, single file >500 lines churn |

## Data Storage

Snapshots are stored in SQLite under the XDG data directory:
- macOS: `~/Library/Application Support/decay/snapshots.db`
- Linux: `~/.local/share/decay/snapshots.db`

Each project is identified by its absolute path. Multiple projects share the same database.

## License

[MIT](LICENSE)
