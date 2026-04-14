mod db;
mod diagnose;
mod git;
mod scan;
mod score;
mod trend;

use std::env;

use anyhow::Result;
use clap::Parser;
use log::debug;
use serde::Serialize;

/// Project health monitoring for AI agents
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Output results as JSON
    #[arg(long)]
    json: bool,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[derive(Serialize)]
struct Report {
    snapshot_id: i64,
    scores: Scores,
    #[serde(skip_serializing_if = "Option::is_none")]
    trend: Option<trend::Trend>,
    issues: Vec<diagnose::Issue>,
    scan: scan::ScanSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    git: Option<git::GitSummary>,
}

#[derive(Serialize)]
struct Scores {
    structural: i32,
    complexity: i32,
    fragility: Option<i32>,
    composite: i32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        unsafe {
            env::set_var("RUST_LOG", "debug");
        }
    }
    env_logger::init();

    debug!("decay starting");

    let conn = db::init()?;
    let project_path = env::current_dir()?;
    let project_path_str = project_path.to_string_lossy().to_string();
    let snapshot_id = db::create_snapshot(&conn, &project_path_str)?;
    debug!("snapshot {snapshot_id} created for {project_path_str}");

    let scan_summary = scan::collect(&conn, snapshot_id, &project_path)?;
    debug!(
        "scan complete: {} files, {} dirs",
        scan_summary.file_count, scan_summary.dir_count
    );

    let git_summary = match git::collect(&conn, snapshot_id, &project_path, 90) {
        Ok(summary) => {
            debug!(
                "git analysis complete: {} commits, {} files",
                summary.total_commits, summary.files_analyzed
            );
            Some(summary)
        }
        Err(e) => {
            debug!("git analysis skipped: {e}");
            if !cli.json {
                eprintln!("Git analysis skipped: {e}");
            }
            None
        }
    };

    let s = score::structural(&conn, snapshot_id)?;
    let c = score::complexity(&conn, snapshot_id)?;
    let f = if git_summary.is_some() {
        score::fragility(&conn, snapshot_id)?
    } else {
        None
    };
    let comp = score::composite(s, c, f);
    debug!("scores: structural={s} complexity={c} fragility={f:?} composite={comp}");

    db::insert_scores(&conn, snapshot_id, s, c, f, comp)?;

    let trend_data = db::get_previous_scores(&conn, &project_path_str, snapshot_id)?
        .map(|prev| trend::Trend::compare(s, c, f, comp, &prev));

    let issues = diagnose::run(&conn, snapshot_id)?;
    debug!("diagnosis complete: {} issues", issues.len());

    if cli.json {
        let report = Report {
            snapshot_id,
            scores: Scores {
                structural: s,
                complexity: c,
                fragility: f,
                composite: comp,
            },
            trend: trend_data,
            issues,
            scan: scan_summary,
            git: git_summary,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Scanned: {} files, {} dirs, max depth {}",
            scan_summary.file_count, scan_summary.dir_count, scan_summary.max_depth
        );

        if let Some(ref git) = git_summary {
            println!(
                "Git: {} commits, {} files changed (last 90 days)",
                git.total_commits, git.files_analyzed
            );
        }

        match &trend_data {
            Some(t) => println!("{}", trend::format_health_with_trend(comp, s, c, f, t)),
            None => {
                let f_display = match f {
                    Some(v) => format!("{v}"),
                    None => "N/A".to_string(),
                };
                println!(
                    "Health: {comp}/100 structural: {s} complexity: {c} fragility: {f_display}"
                );
            }
        }

        diagnose::print_issues(&issues);

        println!(
            "Snapshot #{snapshot_id} created for {}",
            project_path.display()
        );
    }

    Ok(())
}
