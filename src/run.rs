use std::collections::HashMap;
use std::env;

use anyhow::Result;
use log::debug;
use serde::Serialize;

use crate::{action, collector, data_store, db, diagnose, dimension, profile, render, trend};

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub time_series: Vec<db::SnapshotScores>,
    pub collectors: HashMap<String, HashMap<String, String>>,
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

    let all_actions = action::collect_sorted(&all_issues);

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

    // Time series for trend engine (v5)
    let time_series = db::get_dimension_time_series(store.conn(), &project_path_str, None)?;

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
            time_series,
            collectors: collector_stats,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if markdown {
        let md = render::render_markdown(&render::MarkdownCtx {
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
        render::render_terminal(
            &collector_stats,
            &scores,
            comp,
            &trend_data,
            &dimensions,
            &all_issues,
            snapshot_id,
            &project_path,
        );
    }

    Ok(critical_count > 0)
}
