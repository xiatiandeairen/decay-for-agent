use std::collections::HashMap;
use std::env;

use anyhow::Result;
use log::debug;
use serde::Serialize;

use crate::{action, collector, data_store, db, diagnose, dimension, profile, trend};

#[derive(Serialize)]
pub struct Report {
    pub project_type: profile::ProjectType,
    pub snapshot_id: i64,
    pub scores: HashMap<String, Option<i32>>,
    pub composite: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<HashMap<String, trend::Delta>>,
    pub issues: Vec<diagnose::Issue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<action::Action>,
    pub collectors: HashMap<String, HashMap<String, String>>,
}

pub struct MarkdownCtx<'a> {
    pub snapshot_id: i64,
    pub project_path: &'a str,
    pub scores: &'a HashMap<String, Option<i32>>,
    pub composite: i32,
    pub trend_data: &'a Option<HashMap<String, trend::Delta>>,
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
        collectors,
        issues,
        actions: _,
    } = ctx;

    let scan_stats = collectors.get("file_scan");
    let git_stats = collectors.get("git_history");

    // Build scores table rows dynamically
    let dimension_order = ["structural", "complexity", "fragility"];
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
        scores_rows.push_str(&format!("| {name} | {score_str} | {trend_str} |\n"));
    }
    // Add any dimensions not in the fixed order
    for (name, score) in scores.iter() {
        if !dimension_order.contains(&name.as_str()) && name != "composite" {
            let score_str = score.map_or("N/A".to_string(), |v: i32| v.to_string());
            let trend_str = trend_data
                .as_ref()
                .and_then(|t| t.get(name))
                .map_or("—".to_string(), |d| format!("{d}"));
            scores_rows.push_str(&format!("| {name} | {score_str} | {trend_str} |\n"));
        }
    }

    let comp_trend = trend_data
        .as_ref()
        .and_then(|t| t.get("composite"))
        .map_or("—".to_string(), |d| format!("{d}"));
    scores_rows.push_str(&format!(
        "| **composite** | **{composite}** | **{comp_trend}** |"
    ));

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
                    let line = match &issue.prescription {
                        Some(rx) => {
                            format!("- **{}**: {} — *{}*", issue.category, issue.message, rx)
                        }
                        None => format!("- **{}**: {}", issue.category, issue.message),
                    };
                    sections.push(line);
                }
                sections.push(String::new());
            }
        }
        sections.join("\n")
    };

    let actions_section = if ctx.actions.is_empty() {
        String::new()
    } else {
        let mut rows = String::from("## Actions\n\n| Priority | Type | Target | Effort | Reason |\n|----------|------|--------|--------|--------|\n");
        for a in ctx.actions {
            let target = if let Some((start, end)) = a.target.line_range {
                if start == end {
                    format!("{}:{}", a.target.file, start)
                } else {
                    format!("{}:{}-{}", a.target.file, start, end)
                }
            } else {
                a.target.file.clone()
            };
            rows.push_str(&format!(
                "| {} | {} | {} | {:?} | {} |\n",
                a.priority, a.action_type, target, a.effort, a.reason
            ));
        }
        rows.push('\n');
        rows
    };

    let project_name = std::path::Path::new(project_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let timestamp = crate::util::now_utc();

    // Use a simplified template approach — build the full markdown
    format!(
        "# {project_name} Health Report\n\
         \n\
         decay v{version} | {timestamp} | Snapshot #{snapshot_id}\n\
         \n\
         ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
         \n\
         ## Scores\n\
         \n\
         | Dimension | Score | Trend |\n\
         |-----------|------:|-------|\n\
         {scores_rows}\n\
         \n\
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

/// Run the core decay logic.
///
/// Returns `Ok(true)` if critical issues exist, `Ok(false)` otherwise.
pub fn run(json: bool, markdown: bool, quiet: bool) -> Result<bool> {
    debug!("decay starting");

    let conn = db::init()?;
    let project_path = env::current_dir()?;
    let project_path_str = project_path.to_string_lossy().to_string();
    let snapshot_id = db::create_snapshot(&conn, &project_path_str)?;
    debug!("snapshot {snapshot_id} created for {project_path_str}");

    // Run all collectors via registry
    let collectors = collector::all_collectors();
    let mut collector_stats: HashMap<String, HashMap<String, String>> = HashMap::new();

    for c in &collectors {
        c.ensure_schema(&conn)?;
        if !c.available(&project_path) {
            debug!("collector {} skipped: not available", c.name());
            continue;
        }
        match c.collect(&conn, snapshot_id, &project_path) {
            Ok(summary) => {
                debug!("collector {}: {:?}", summary.name, summary.stats);
                collector_stats.insert(summary.name, summary.stats);
            }
            Err(e) => {
                debug!("collector {} failed: {e}", c.name());
                if !json && !markdown && !quiet {
                    eprintln!("{} skipped: {e}", c.name());
                }
            }
        }
    }

    // Detect project type and load score profile
    let project_type = profile::detect(&project_path);
    let score_profile = profile::ScoreProfile::for_type(project_type);
    debug!("detected project type: {project_type:?}");

    // Build shared DataStore for all dimensions (lazy-loads source files, deps)
    let store = data_store::DataStore::new(conn, snapshot_id, project_path_str.clone());

    // Evaluate all dimensions via registry
    let dimensions = dimension::all_dimensions();
    let mut scores: HashMap<String, Option<i32>> = HashMap::new();
    let mut all_issues: Vec<diagnose::Issue> = Vec::new();

    for dim in &dimensions {
        let result = dim.evaluate(&store)?;
        debug!("dimension {}: score={:?}", result.name, result.score);
        scores.insert(result.name.clone(), result.score);
        all_issues.extend(result.issues);
    }

    all_issues.sort_by_key(|i| i.level);

    // Collect all actions from issues, dedup, sort by priority then effort
    let mut all_actions: Vec<action::Action> = all_issues
        .iter()
        .flat_map(|i| i.actions.iter().cloned())
        .collect();
    // Dedup: same dimension + file + action_type → keep first (higher severity)
    all_actions.dedup_by(|b, a| {
        a.dimension == b.dimension
            && a.target.file == b.target.file
            && a.action_type == b.action_type
    });
    // Sort: priority asc (Critical first), then effort asc (Small first)
    all_actions.sort_by(|a, b| {
        a.priority.cmp(&b.priority).then(a.effort.cmp(&b.effort))
    });

    // Compute weighted composite using score profile
    let comp = score_profile.weighted_composite(&scores);
    scores.insert("composite".to_string(), Some(comp));

    // Persist dimension scores
    let score_pairs: Vec<(String, Option<i32>)> = scores
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    db::insert_dimension_scores(store.conn(), snapshot_id, &score_pairs)?;

    // Also persist to legacy scores table for backward compat
    let s = scores.get("structural").and_then(|s| *s).unwrap_or(0);
    let c = scores.get("complexity").and_then(|s| *s).unwrap_or(0);
    let f = scores.get("fragility").and_then(|s| *s);
    db::insert_scores(store.conn(), snapshot_id, s, c, f, comp)?;

    debug!("scores: {scores:?} composite={comp}");

    // Trend comparison
    let trend_data = db::get_previous_dimension_scores(store.conn(), &project_path_str, snapshot_id)?
        .map(|prev| trend::compare_dimensions(&scores, &prev));

    let critical_count = all_issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Critical)
        .count();

    if json {
        let report = Report {
            project_type,
            snapshot_id,
            scores: scores.clone(),
            composite: comp,
            trend: trend_data,
            issues: all_issues,
            actions: all_actions,
            collectors: collector_stats,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if markdown {
        let md = render_markdown(&MarkdownCtx {
            snapshot_id,
            project_path: &project_path_str,
            scores: &scores,
            composite: comp,
            trend_data: &trend_data,
            collectors: &collector_stats,
            issues: &all_issues,
            actions: &all_actions,
        });
        println!("{md}");
    } else if quiet {
        println!("Health: {comp}/100 ({critical_count} critical)");
    } else {
        // Default terminal output
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

        match &trend_data {
            Some(t) => {
                let mut health_parts = vec![format!("Health: {comp}/100")];
                if let Some(cd) = t.get("composite") {
                    health_parts[0] = format!("Health: {comp}/100 ({cd})");
                }
                for dim in &dimensions {
                    let name = dim.name();
                    let score_str = scores
                        .get(name)
                        .and_then(|s| *s)
                        .map_or("N/A".to_string(), |v| v.to_string());
                    let trend_str = t
                        .get(name)
                        .map_or(String::new(), |d| format!(" ({d})"));
                    health_parts.push(format!("{name}: {score_str}{trend_str}"));
                }
                println!("{}", health_parts.join(" "));
            }
            None => {
                let mut health_parts = vec![format!("Health: {comp}/100")];
                for dim in &dimensions {
                    let name = dim.name();
                    let score_str = scores
                        .get(name)
                        .and_then(|s| *s)
                        .map_or("N/A".to_string(), |v| v.to_string());
                    health_parts.push(format!("{name}: {score_str}"));
                }
                println!("{}", health_parts.join(" "));
            }
        }

        diagnose::print_issues(&all_issues);

        println!(
            "Snapshot #{snapshot_id} created for {}",
            project_path.display()
        );
    }

    Ok(critical_count > 0)
}
