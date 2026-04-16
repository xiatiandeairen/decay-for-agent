use std::collections::HashMap;
use std::env;

use crate::{action, aggregate, diagnose, dimension, trend};

pub struct MarkdownCtx<'a> {
    pub snapshot_id: i64,
    pub project_path: &'a str,
    pub scores: &'a HashMap<String, Option<i32>>,
    pub composite: i32,
    pub trend_data: &'a Option<HashMap<String, trend::Delta>>,
    pub velocities: &'a [trend::Velocity],
    pub regressions: &'a [trend::Regression],
    pub forecasts: &'a [trend::Forecast],
    pub correlations: &'a [trend::Correlation],
    pub trajectory: &'a Option<trend::Trajectory>,
    pub collectors: &'a HashMap<String, HashMap<String, String>>,
    pub issues: &'a [diagnose::Issue],
    pub aggregated_issues: &'a [aggregate::AggregatedIssue],
    pub actions: &'a [action::Action],
}

pub fn render_markdown(ctx: &MarkdownCtx<'_>) -> String {
    let project_name = std::path::Path::new(ctx.project_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let timestamp = crate::util::now_utc();

    let (scores_header, scores_rows) = render_scores_table(ctx);
    let trajectory_section = render_trajectory_section(ctx);
    let scan_section = render_scan_section(ctx);
    let (issue_summary, issues_section) = render_issues_section(ctx.issues);
    let aggregated_section = render_aggregated_section(ctx.aggregated_issues);
    let actions_section = render_actions_section(ctx.actions);

    format!(
        "# {project_name} Health Report\n\
         \n\
         decay v{version} | {timestamp} | Snapshot #{snapshot_id}\n\
         \n\
         ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
         \n\
         ## Scores\n\
         \n\
         {scores_header}\
         {scores_rows}\n\
         \n\
         {trajectory_section}\
         {scan_section}\
         \n\
         ## Issues ({issue_summary})\n\
         \n\
         {issues_section}\n\
         \n\
         {aggregated_section}\
         {actions_section}\
         ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
        version = env!("CARGO_PKG_VERSION"),
        snapshot_id = ctx.snapshot_id,
    )
}

/// Build scores table header and rows, including velocity column when available.
fn render_scores_table(ctx: &MarkdownCtx<'_>) -> (String, String) {
    let vel_map: HashMap<&str, &trend::Velocity> = ctx
        .velocities
        .iter()
        .map(|v| (v.dimension.as_str(), v))
        .collect();

    let dimension_order = ["structural", "complexity", "fragility"];
    let has_velocity = !ctx.velocities.is_empty();
    let mut rows = String::new();

    for name in &dimension_order {
        let score_str = ctx
            .scores
            .get(*name)
            .and_then(|s| *s)
            .map_or("N/A".to_string(), |v| v.to_string());
        let trend_str = ctx
            .trend_data
            .as_ref()
            .and_then(|t| t.get(*name))
            .map_or("—".to_string(), |d| format!("{d}"));
        let vel_str = vel_map
            .get(name)
            .map_or("—".to_string(), |v| format!("{} {:.1}/snap", v.direction, v.slope));
        if has_velocity {
            rows.push_str(&format!("| {name} | {score_str} | {trend_str} | {vel_str} |\n"));
        } else {
            rows.push_str(&format!("| {name} | {score_str} | {trend_str} |\n"));
        }
    }

    for (name, score) in ctx.scores.iter() {
        if !dimension_order.contains(&name.as_str()) && name != "composite" {
            let score_str = score.map_or("N/A".to_string(), |v: i32| v.to_string());
            let trend_str = ctx
                .trend_data
                .as_ref()
                .and_then(|t| t.get(name))
                .map_or("—".to_string(), |d| format!("{d}"));
            let vel_str = vel_map
                .get(name.as_str())
                .map_or("—".to_string(), |v| format!("{} {:.1}/snap", v.direction, v.slope));
            if has_velocity {
                rows.push_str(&format!("| {name} | {score_str} | {trend_str} | {vel_str} |\n"));
            } else {
                rows.push_str(&format!("| {name} | {score_str} | {trend_str} |\n"));
            }
        }
    }

    let comp_trend = ctx
        .trend_data
        .as_ref()
        .and_then(|t| t.get("composite"))
        .map_or("—".to_string(), |d| format!("{d}"));
    let comp_vel = vel_map
        .get("composite")
        .map_or("—".to_string(), |v| format!("{} {:.1}/snap", v.direction, v.slope));
    let composite = ctx.composite;
    if has_velocity {
        rows.push_str(&format!(
            "| **composite** | **{composite}** | **{comp_trend}** | **{comp_vel}** |"
        ));
    } else {
        rows.push_str(&format!(
            "| **composite** | **{composite}** | **{comp_trend}** |"
        ));
    }

    let header = if has_velocity {
        "| Dimension | Score | Trend | Velocity |\n|-----------|------:|-------|----------|\n"
    } else {
        "| Dimension | Score | Trend |\n|-----------|------:|-------|\n"
    };

    (header.to_string(), rows)
}

/// Build the Health Trajectory section (regressions, forecasts, correlations).
fn render_trajectory_section(ctx: &MarkdownCtx<'_>) -> String {
    let trajectory = match ctx.trajectory {
        Some(traj) => traj,
        None => return String::new(),
    };

    let mut s = format!("## Health Trajectory ({})\n\n", trajectory.overall_direction);

    if !ctx.regressions.is_empty() {
        s.push_str("### Regressions\n\n");
        for r in ctx.regressions {
            s.push_str(&format!(
                "- **{}** ({severity}): {prev} → {curr} (−{drop}, threshold: {thresh:.1})\n",
                r.dimension,
                severity = r.severity,
                prev = r.previous_score,
                curr = r.current_score,
                drop = r.drop,
                thresh = r.threshold,
            ));
        }
        s.push('\n');
    }

    if !ctx.forecasts.is_empty() {
        s.push_str("### Forecasts\n\n");
        for f in ctx.forecasts {
            s.push_str(&format!(
                "- **{}** will breach {} in ~{} snapshots (current: {}, slope: {:.1}/snap, R²={:.2})\n",
                f.dimension, f.threshold, f.snapshots_until_breach,
                f.current_score, f.slope, f.r_squared,
            ));
        }
        s.push('\n');
    }

    if !ctx.correlations.is_empty() {
        s.push_str("### Correlations\n\n");
        for c in ctx.correlations {
            let sign = if c.coefficient > 0.0 { "+" } else { "" };
            s.push_str(&format!(
                "- **{}** ↔ **{}**: {sign}{:.2} ({strength})\n",
                c.dim_a, c.dim_b, c.coefficient,
                strength = c.strength,
            ));
        }
        s.push('\n');
    }

    if ctx.regressions.is_empty() && ctx.forecasts.is_empty() && ctx.correlations.is_empty() {
        s.push_str("No regressions, forecasts, or correlations detected.\n\n");
    }

    s
}

/// Build the Scan stats table from collector data.
fn render_scan_section(ctx: &MarkdownCtx<'_>) -> String {
    let scan_stats = ctx.collectors.get("file_scan");
    let git_stats = ctx.collectors.get("git_history");

    let stat = |source: Option<&HashMap<String, String>>, key: &str| -> String {
        source
            .and_then(|s| s.get(key))
            .cloned()
            .unwrap_or_else(|| "N/A".to_string())
    };

    let file_count = stat(scan_stats, "files");
    let dir_count = stat(scan_stats, "dirs");
    let max_depth = stat(scan_stats, "max_depth");
    let total_commits = stat(git_stats, "commits");
    let files_analyzed = stat(git_stats, "files_analyzed");

    format!(
        "## Scan\n\
         \n\
         | Metric | Value |\n\
         |--------|------:|\n\
         | Files | {file_count} |\n\
         | Directories | {dir_count} |\n\
         | Max depth | {max_depth} |\n\
         | Commits (90d) | {total_commits} |\n\
         | Files changed | {files_analyzed} |"
    )
}

/// Build the Issues section grouped by level, returning (summary, body).
fn render_issues_section(issues: &[diagnose::Issue]) -> (String, String) {
    let critical_count = issues.iter().filter(|i| i.level == diagnose::Level::Critical).count();
    let warning_count = issues.iter().filter(|i| i.level == diagnose::Level::Warning).count();
    let info_count = issues.iter().filter(|i| i.level == diagnose::Level::Info).count();

    let summary = if issues.is_empty() {
        "none".to_string()
    } else {
        let mut parts = Vec::new();
        if critical_count > 0 {
            parts.push(format!("{critical_count} critical"));
        }
        if warning_count > 0 {
            parts.push(format!("{warning_count} warning"));
        }
        if info_count > 0 {
            parts.push(format!("{info_count} info"));
        }
        parts.join(", ")
    };

    let body = if issues.is_empty() {
        "No issues found.".to_string()
    } else {
        let mut sections = Vec::new();
        for (level, label) in [
            (diagnose::Level::Critical, "Critical"),
            (diagnose::Level::Warning, "Warning"),
            (diagnose::Level::Info, "Info"),
        ] {
            let level_issues: Vec<&diagnose::Issue> =
                issues.iter().filter(|i| i.level == level).collect();
            if !level_issues.is_empty() {
                sections.push(format!("### {label}\n"));
                for issue in level_issues {
                    let line = match issue.actions.first() {
                        Some(a) => {
                            let class = issue.classification.map(|c| format!(" `{c}`")).unwrap_or_default();
                            let mut s = format!("- **{}**{class}: {} — *{}*", issue.category, issue.message, a.suggestion);
                            for detail in &a.details {
                                s.push_str(&format!("\n  - {detail}"));
                            }
                            s
                        },
                        None => {
                            let class = issue.classification.map(|c| format!(" `{c}`")).unwrap_or_default();
                            format!("- **{}**{class}: {}", issue.category, issue.message)
                        },
                    };
                    sections.push(line);
                }
                sections.push(String::new());
            }
        }
        sections.join("\n")
    };

    (summary, body)
}

/// Build the Root Cause Analysis section from aggregated issues.
fn render_aggregated_section(aggregated_issues: &[aggregate::AggregatedIssue]) -> String {
    if aggregated_issues.is_empty() {
        return String::new();
    }

    let mut s = String::from("## Root Cause Analysis\n\n");
    for agg in aggregated_issues {
        s.push_str(&format!(
            "### {} `{}`\n\n- **Affected**: {} files ({})\n- **Approach**: {}\n\n",
            agg.root_cause,
            agg.category,
            agg.issue_count,
            agg.affected_files.join(", "),
            agg.suggested_approach,
        ));
    }
    s
}

/// Build the Actions table.
fn render_actions_section(actions: &[action::Action]) -> String {
    if actions.is_empty() {
        return String::new();
    }

    let mut rows = String::from("## Actions\n\n| Priority | Type | Target | Effort | Suggestion |\n|----------|------|--------|--------|------------|\n");
    for a in actions {
        let target = format_target(&a.target);
        rows.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            a.priority, a.action_type, target, a.effort, a.suggestion
        ));
    }
    rows.push('\n');
    rows
}

/// Render terminal output (non-JSON, non-markdown, non-quiet).
pub fn render_terminal(
    collector_stats: &HashMap<String, HashMap<String, String>>,
    scores: &HashMap<String, Option<i32>>,
    comp: i32,
    trend_data: &Option<HashMap<String, trend::Delta>>,
    velocities: &[trend::Velocity],
    regressions: &[trend::Regression],
    forecasts: &[trend::Forecast],
    correlations: &[trend::Correlation],
    trajectory: &Option<trend::Trajectory>,
    summary: Option<&crate::summary::Summary>,
    dimensions: &[Box<dyn dimension::Dimension>],
    issues: &[diagnose::Issue],
    snapshot_id: i64,
    project_path: &std::path::Path,
) {
    // Summary first — the most important info
    if let Some(s) = summary {
        println!("━━━ {} ━━━", s.headline);
        if !s.top_actions.is_empty() {
            println!("Top actions:");
            for a in &s.top_actions {
                println!("  [{:>8}] {} ({})", a.priority, a.what, a.effort);
            }
        }
        println!();
    }

    if let Some(scan) = collector_stats.get("file_scan") {
        let files = scan.get("files").map_or("?", |s| s);
        let dirs = scan.get("dirs").map_or("?", |s| s);
        let depth = scan.get("max_depth").map_or("?", |s| s);
        println!("Scanned: {files} files, {dirs} dirs, max depth {depth}");
    }

    if let Some(git) = collector_stats.get("git_history") {
        let commits = git.get("commits").map_or("?", |s| s);
        let analyzed = git.get("files_analyzed").map_or("?", |s| s);
        println!("Git: {commits} commits, {analyzed} files changed (last 90 days)");
    }

    let vel_map: HashMap<&str, &trend::Velocity> =
        velocities.iter().map(|v| (v.dimension.as_str(), v)).collect();

    let mut health_parts = Vec::new();
    let comp_trend = trend_data.as_ref().and_then(|t| t.get("composite"));
    match comp_trend {
        Some(cd) => health_parts.push(format!("Health: {comp}/100 ({cd})")),
        None => health_parts.push(format!("Health: {comp}/100")),
    }
    for dim in dimensions {
        let name = dim.name();
        let score_str = scores
            .get(name)
            .and_then(|s| *s)
            .map_or("N/A".to_string(), |v| v.to_string());
        let trend_str = trend_data
            .as_ref()
            .and_then(|t| t.get(name))
            .map_or(String::new(), |d| format!(" ({d})"));
        let vel_str = vel_map
            .get(name)
            .map_or(String::new(), |v| format!(" {}{:.1}/snap", v.direction, v.slope));
        health_parts.push(format!("{name}: {score_str}{trend_str}{vel_str}"));
    }
    println!("{}", health_parts.join(" "));

    if let Some(traj) = trajectory {
        println!(
            "Trajectory: {} | {} velocities, {} regressions, {} forecasts, {} correlations",
            traj.overall_direction,
            traj.velocities.len(),
            traj.regressions.len(),
            traj.forecasts.len(),
            traj.correlations.len(),
        );
    }
    for r in regressions {
        eprintln!(
            "⚠ REGRESSION: {} {} → {} (−{}, threshold: {:.1})",
            r.dimension, r.previous_score, r.current_score, r.drop, r.threshold
        );
    }
    for f in forecasts {
        eprintln!(
            "⚡ FORECAST: {} will breach {} in ~{} snapshots (R²={:.2})",
            f.dimension, f.threshold, f.snapshots_until_breach, f.r_squared
        );
    }
    if !correlations.is_empty() {
        let pairs: Vec<String> = correlations
            .iter()
            .map(|c| format!("{} ↔ {} ({:.2})", c.dim_a, c.dim_b, c.coefficient))
            .collect();
        println!("Correlations: {}", pairs.join(", "));
    }

    diagnose::print_issues(issues);

    println!(
        "Snapshot #{snapshot_id} created for {}",
        project_path.display()
    );
}

fn format_target(target: &action::Target) -> String {
    if let Some((start, end)) = target.line_range {
        if start == end {
            format!("{}:{}", target.file, start)
        } else {
            format!("{}:{}-{}", target.file, start, end)
        }
    } else {
        target.file.clone()
    }
}
