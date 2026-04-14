mod db;
mod git;
mod scan;

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

            match git::collect(&conn, snapshot_id, &project_path, 90) {
                Ok(git_summary) => {
                    println!(
                        "Git: {} commits, {} files changed (last 90 days)",
                        git_summary.total_commits, git_summary.files_analyzed
                    );
                }
                Err(e) => {
                    eprintln!("Git analysis skipped: {e}");
                }
            }

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
