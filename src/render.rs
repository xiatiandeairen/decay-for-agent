use std::collections::HashMap;
use std::env;

use crate::{action, diagnose, dimension, trend};

pub struct MarkdownCtx<'a> {
    pub snapshot_id: i64,
    pub project_path: &'a str,
    pub scores: &'a HashMap<String, Option<i32>>,
    pub composite: i32,
    pub trend_data: &'a Option<HashMap<String, trend::Delta>>,
    pub velocities: &'a [trend::Velocity],
    pub regressions: &'a [trend::Regression],
    pub collectors: &'a HashMap<String, HashMap<String, String>>,
    pub issues: &'a [diagnose::Issue],
    pub actions: &'a [action::Action],
}

pub fn render_markdown(ctx: &MarkdownCtx<'_>) -> String {
    let MarkdownCtx {
        snapshot_id,
        project_path,
        scores,
        composite,
        trend_data,
        velocities,
        regressions,
        collectors,
        issues,
        actions,
    } = ctx;

    let scan_stats = collectors.get("file_scan");
    let git_stats = collectors.get("git_history");

    // Build velocity lookup
    let vel_map: HashMap<&str, &trend::Velocity> =
        velocities.iter().map(|v| (v.dimension.as_str(), v)).collect();

    // Build scores table rows dynamically
    let dimension_order = ["structural", "complexity", "fragility"];
    let has_velocity = !velocities.is_empty();
    let mut scores_rows = String::new();
    for name in &dimension_order {
        let score_str = scores
            .get(*name)
            .and_then(|s| *s)
            .map_or("N/A".to_string(), |v| v.to_string());
        let trend_str = trend_data
            .as_ref()
            .and_then(|t| t.get(*name))
            .map_or("—".to_string(), |d| format!("{d}"));
        let vel_str = vel_map
            .get(name)
            .map_or("—".to_string(), |v| format!("{} {:.1}/snap", v.direction, v.slope));
        if has_velocity {
            scores_rows.push_str(&format!("| {name} | {score_str} | {trend_str} | {vel_str} |\n"));
        } else {
            scores_rows.push_str(&format!("| {name} | {score_str} | {trend_str} |\n"));
        }
    }
    for (name, score) in scores.iter() {
        if !dimension_order.contains(&name.as_str()) && name != "composite" {
            let score_str = score.map_or("N/A".to_string(), |v: i32| v.to_string());
            let trend_str = trend_data
                .as_ref()
                .and_then(|t| t.get(name))
                .map_or("—".to_string(), |d| format!("{d}"));
            let vel_str = vel_map
                .get(name.as_str())
                .map_or("—".to_string(), |v| format!("{} {:.1}/snap", v.direction, v.slope));
            if has_velocity {
                scores_rows.push_str(&format!("| {name} | {score_str} | {trend_str} | {vel_str} |\n"));
            } else {
                scores_rows.push_str(&format!("| {name} | {score_str} | {trend_str} |\n"));
            }
        }
    }

    let comp_trend = trend_data
        .as_ref()
        .and_then(|t| t.get("composite"))
        .map_or("—".to_string(), |d| format!("{d}"));
    let comp_vel = vel_map
        .get("composite")
        .map_or("—".to_string(), |v| format!("{} {:.1}/snap", v.direction, v.slope));
    if has_velocity {
        scores_rows.push_str(&format!(
            "| **composite** | **{composite}** | **{comp_trend}** | **{comp_vel}** |"
        ));
    } else {
        scores_rows.push_str(&format!(
            "| **composite** | **{composite}** | **{comp_trend}** |"
        ));
    }

    let file_count = scan_stats
        .and_then(|s| s.get("files"))
        .cloned()
        .unwrap_or_else(|| "N/A".to_string());
    let dir_count = scan_stats
        .and_then(|s| s.get("dirs"))
        .cloned()
        .unwrap_or_else(|| "N/A".to_string());
    let max_depth = scan_stats
        .and_then(|s| s.get("max_depth"))
        .cloned()
        .unwrap_or_else(|| "N/A".to_string());
    let total_commits = git_stats
        .and_then(|s| s.get("commits"))
        .cloned()
        .unwrap_or_else(|| "N/A".to_string());
    let files_analyzed = git_stats
        .and_then(|s| s.get("files_analyzed"))
        .cloned()
        .unwrap_or_else(|| "N/A".to_string());

    let critical_count = issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Critical)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Warning)
        .count();
    let info_count = issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Info)
        .count();

    let issue_summary = if issues.is_empty() {
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

    let issues_section = if issues.is_empty() {
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
                        Some(a) => format!("- **{}**: {} — *{}*", issue.category, issue.message, a.suggestion),
                        None => format!("- **{}**: {}", issue.category, issue.message),
                    };
                    sections.push(line);
                }
                sections.push(String::new());
            }
        }
        sections.join("\n")
    };

    let actions_section = if actions.is_empty() {
        String::new()
    } else {
        let mut rows = String::from("## Actions\n\n| Priority | Type | Target | Effort | Suggestion |\n|----------|------|--------|--------|------------|\n");
        for a in *actions {
            let target = format_target(&a.target);
            rows.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                a.priority, a.action_type, target, a.effort, a.suggestion
            ));
        }
        rows.push('\n');
        rows
    };

    let scores_header = if has_velocity {
        "| Dimension | Score | Trend | Velocity |\n|-----------|------:|-------|----------|\n"
    } else {
        "| Dimension | Score | Trend |\n|-----------|------:|-------|\n"
    };

    let regressions_section = if regressions.is_empty() {
        String::new()
    } else {
        let mut s = String::from("## Regressions\n\n");
        for r in *regressions {
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
        s
    };

    let project_name = std::path::Path::new(project_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let timestamp = crate::util::now_utc();

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
         {regressions_section}\
         ## Scan\n\
         \n\
         | Metric | Value |\n\
         |--------|------:|\n\
         | Files | {file_count} |\n\
         | Directories | {dir_count} |\n\
         | Max depth | {max_depth} |\n\
         | Commits (90d) | {total_commits} |\n\
         | Files changed | {files_analyzed} |\n\
         \n\
         ## Issues ({issue_summary})\n\
         \n\
         {issues_section}\n\
         \n\
         {actions_section}\
         ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// Render terminal output (non-JSON, non-markdown, non-quiet).
pub fn render_terminal(
    collector_stats: &HashMap<String, HashMap<String, String>>,
    scores: &HashMap<String, Option<i32>>,
    comp: i32,
    trend_data: &Option<HashMap<String, trend::Delta>>,
    velocities: &[trend::Velocity],
    regressions: &[trend::Regression],
    dimensions: &[Box<dyn dimension::Dimension>],
    issues: &[diagnose::Issue],
    snapshot_id: i64,
    project_path: &std::path::Path,
) {
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

    for r in regressions {
        eprintln!(
            "⚠ REGRESSION: {} {} → {} (−{}, threshold: {:.1})",
            r.dimension, r.previous_score, r.current_score, r.drop, r.threshold
        );
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
