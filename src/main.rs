mod db;
mod diagnose;
mod git;
mod scan;
mod score;

use std::env;

use anyhow::Result;
use clap::Parser;

/// Project health monitoring for AI agents
#[derive(Parser)]
#[command(version, about)]
struct Cli {}

fn main() -> Result<()> {
    let cli = Cli::try_parse();

    match cli {
        Ok(_) => {
            let conn = db::init()?;
            let project_path = env::current_dir()?;
            let snapshot_id = db::create_snapshot(&conn, &project_path.to_string_lossy())?;

            let scan_summary = scan::collect(&conn, snapshot_id, &project_path)?;
            println!(
                "Scanned: {} files, {} dirs, max depth {}",
                scan_summary.file_count, scan_summary.dir_count, scan_summary.max_depth
            );

            let has_git = match git::collect(&conn, snapshot_id, &project_path, 90) {
                Ok(git_summary) => {
                    println!(
                        "Git: {} commits, {} files changed (last 90 days)",
                        git_summary.total_commits, git_summary.files_analyzed
                    );
                    true
                }
                Err(e) => {
                    eprintln!("Git analysis skipped: {e}");
                    false
                }
            };

            let s = score::structural(&conn, snapshot_id)?;
            let c = score::complexity(&conn, snapshot_id)?;
            let f = if has_git {
                score::fragility(&conn, snapshot_id)?
            } else {
                None
            };
            let comp = score::composite(s, c, f);

            db::insert_scores(&conn, snapshot_id, s, c, f, comp)?;

            let f_display = match f {
                Some(v) => format!("{v}"),
                None => "N/A".to_string(),
            };
            println!(
                "Health: {comp}/100 (structural: {s}, complexity: {c}, fragility: {f_display})"
            );

            let issues = diagnose::run(&conn, snapshot_id)?;
            diagnose::print_issues(&issues);

            println!(
                "Snapshot #{snapshot_id} created for {}",
                project_path.display()
            );
        }
        Err(e) => {
            e.exit();
        }
    }

    Ok(())
}
