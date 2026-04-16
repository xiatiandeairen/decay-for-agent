use std::collections::HashMap;
use std::env;
use std::path::Path;

use anyhow::Result;
use log::debug;
use serde::Serialize;

use crate::{action, aggregate, chronic, classify, collector, data_store, db, diagnose, dimension, impact, patch, prevention, profile, render, report, summary, trend};

#[derive(Serialize)]
pub struct Report {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<summary::Summary>,
    pub project_type: profile::ProjectType,
    pub snapshot_id: i64,
    pub scores: HashMap<String, Option<i32>>,
    pub composite: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<HashMap<String, trend::Delta>>,
    pub issues: Vec<diagnose::Issue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aggregated_issues: Vec<aggregate::AggregatedIssue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub patches: Vec<patch::Patch>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub preventions: Vec<prevention::Prevention>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chronic_warnings: Vec<chronic::ChronicWarning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_report: Option<report::DiagnosticReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<action::Action>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub velocities: Vec<trend::Velocity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub regressions: Vec<trend::Regression>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forecasts: Vec<trend::Forecast>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub correlations: Vec<trend::Correlation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<trend::Trajectory>,
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

    let collector_stats = run_collectors(&conn, snapshot_id, &project_path, json || markdown || quiet)?;

    let project_type = profile::detect(&project_path);
    let score_profile = profile::ScoreProfile::for_type(project_type);
    debug!("detected project type: {project_type:?}");

    let store = data_store::DataStore::new(conn, snapshot_id, project_path_str.clone());

    let (scores, comp, mut all_issues, _) = evaluate(&store, &score_profile, snapshot_id, &project_path_str)?;
    classify::classify_issues(&mut all_issues);

    // Enrich actions with development impact
    let coupling_map = impact::build_coupling_map(store.conn(), snapshot_id).unwrap_or_default();
    let source_files = store.source_files();
    for issue in &mut all_issues {
        for act in &mut issue.actions {
            let line_count = source_files
                .iter()
                .find(|sf| sf.path == act.target.file)
                .map(|sf| sf.line_count)
                .unwrap_or(0);
            act.impact = Some(impact::compute_impact(&act.target.file, line_count, &coupling_map));
        }
    }

    // Re-collect actions after impact enrichment
    let all_actions = action::collect_sorted(&all_issues);
    let aggregated_issues = aggregate::aggregate_issues(&all_issues);
    let patches = patch::generate_patches(&all_issues, store.source_files());
    let preventions = prevention::generate_preventions(&all_issues);
    let diagnostic_report = if all_issues.is_empty() {
        None
    } else {
        Some(report::build_diagnostic_report(&all_issues, &aggregated_issues, &patches, &preventions))
    };

    let trend_data = db::get_previous_dimension_scores(store.conn(), &project_path_str, snapshot_id)?
        .map(|prev| trend::compare_dimensions(&scores, &prev));
    let time_series = db::get_dimension_time_series(store.conn(), &project_path_str, None)?;
    let trajectory = if time_series.len() >= 3 {
        Some(trend::build_trajectory(&time_series, 2.0, 60))
    } else {
        None
    };
    let velocities = trajectory.as_ref().map_or_else(Vec::new, |t| t.velocities.clone());
    let regressions = trajectory.as_ref().map_or_else(Vec::new, |t| t.regressions.clone());
    let forecasts = trajectory.as_ref().map_or_else(Vec::new, |t| t.forecasts.clone());
    let correlations = trajectory.as_ref().map_or_else(Vec::new, |t| t.correlations.clone());
    let chronic_warnings = chronic::detect_chronic_decay(&scores, trajectory.as_ref());

    let agent_summary = summary::generate_summary(comp, &all_issues, &all_actions, trajectory.as_ref());

    let critical_count = all_issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Critical)
        .count();

    output(
        json, markdown, quiet,
        &Report {
            summary: Some(agent_summary),
            project_type, snapshot_id,
            scores: scores.clone(), composite: comp,
            trend: trend_data,
            issues: all_issues, aggregated_issues, patches, preventions,
            chronic_warnings, diagnostic_report,
            actions: all_actions,
            velocities, regressions, forecasts, correlations, trajectory,
            time_series, collectors: collector_stats,
        },
        &scores, comp, critical_count, snapshot_id, &project_path,
    );

    Ok(critical_count > 0)
}

fn run_collectors(
    conn: &rusqlite::Connection,
    snapshot_id: i64,
    project_path: &Path,
    silent: bool,
) -> Result<HashMap<String, HashMap<String, String>>> {
    let collectors = collector::all_collectors();
    let mut stats: HashMap<String, HashMap<String, String>> = HashMap::new();

    for c in &collectors {
        c.ensure_schema(conn)?;
        if !c.available(project_path) {
            debug!("collector {} skipped: not available", c.name());
            continue;
        }
        match c.collect(conn, snapshot_id, project_path) {
            Ok(summary) => {
                debug!("collector {}: {:?}", summary.name, summary.stats);
                stats.insert(summary.name, summary.stats);
            }
            Err(e) => {
                debug!("collector {} failed: {e}", c.name());
                if !silent {
                    eprintln!("{} skipped: {e}", c.name());
                }
            }
        }
    }
    Ok(stats)
}

fn evaluate(
    store: &data_store::DataStore,
    score_profile: &profile::ScoreProfile,
    snapshot_id: i64,
    project_path_str: &str,
) -> Result<(HashMap<String, Option<i32>>, i32, Vec<diagnose::Issue>, Vec<action::Action>)> {
    let dimensions = dimension::all_dimensions();
    let mut scores: HashMap<String, Option<i32>> = HashMap::new();
    let mut all_issues: Vec<diagnose::Issue> = Vec::new();

    for dim in &dimensions {
        let result = dim.evaluate(store)?;
        debug!("dimension {}: score={:?}", result.name, result.score);
        scores.insert(result.name.clone(), result.score);
        all_issues.extend(result.issues);
    }

    all_issues.sort_by_key(|i| i.level);
    let all_actions = action::collect_sorted(&all_issues);

    let comp = score_profile.weighted_composite(&scores);
    scores.insert("composite".to_string(), Some(comp));

    let score_pairs: Vec<(String, Option<i32>)> = scores.iter().map(|(k, v)| (k.clone(), *v)).collect();
    db::insert_dimension_scores(store.conn(), snapshot_id, &score_pairs)?;

    let s = scores.get("structural").and_then(|s| *s).unwrap_or(0);
    let c = scores.get("complexity").and_then(|s| *s).unwrap_or(0);
    let f = scores.get("fragility").and_then(|s| *s);
    db::insert_scores(store.conn(), snapshot_id, s, c, f, comp)?;

    debug!("scores: {scores:?} composite={comp}");

    Ok((scores, comp, all_issues, all_actions))
}

fn output(
    json: bool, markdown: bool, quiet: bool,
    report: &Report,
    scores: &HashMap<String, Option<i32>>,
    comp: i32, critical_count: usize,
    snapshot_id: i64, project_path: &Path,
) {
    if json {
        println!("{}", serde_json::to_string_pretty(report).unwrap_or_default());
    } else if markdown {
        let project_path_str = project_path.to_string_lossy();
        let md = render::render_markdown(&render::MarkdownCtx {
            snapshot_id,
            project_path: &project_path_str,
            scores,
            composite: comp,
            trend_data: &report.trend,
            velocities: &report.velocities,
            regressions: &report.regressions,
            forecasts: &report.forecasts,
            correlations: &report.correlations,
            trajectory: &report.trajectory,
            collectors: &report.collectors,
            issues: &report.issues,
            aggregated_issues: &report.aggregated_issues,
            actions: &report.actions,
        });
        println!("{md}");
    } else if quiet {
        println!("Health: {comp}/100 ({critical_count} critical)");
    } else {
        let dimensions = dimension::all_dimensions();
        render::render_terminal(
            &report.collectors,
            scores,
            comp,
            &report.trend,
            &report.velocities,
            &report.regressions,
            &report.forecasts,
            &report.correlations,
            &report.trajectory,
            report.summary.as_ref(),
            &dimensions,
            &report.issues,
            snapshot_id,
            project_path,
        );
    }
}
