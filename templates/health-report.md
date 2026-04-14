# {{project_name}} Health Report

decay v{{version}} | {{timestamp}} | Snapshot #{{snapshot_id}}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## Scores

| Dimension | Score | Trend |
|-----------|------:|-------|
| structural | {{structural}} | {{structural_trend}} |
| complexity | {{complexity}} | {{complexity_trend}} |
| fragility | {{fragility}} | {{fragility_trend}} |
| **composite** | **{{composite}}** | **{{composite_trend}}** |

## Scan

| Metric | Value |
|--------|------:|
| Files | {{file_count}} |
| Directories | {{dir_count}} |
| Max depth | {{max_depth}} |
| Commits (90d) | {{total_commits}} |
| Files changed | {{files_analyzed}} |

## Issues ({{issue_summary}})

{{issues_section}}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
