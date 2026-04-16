mod action;
mod aggregate;
mod chronic;
mod classify;
mod compare;
mod collector;
mod config;
mod data_store;
mod db;
mod diagnose;
mod dimension;
mod filter;
mod filter_pipeline;
mod git;
mod patch;
mod prevention;
mod report;
mod git_pipeline;
mod profile;
mod render;
mod run;
mod scan;
mod summary;
mod trend;
mod util;

#[cfg(test)]
mod test_helpers;

use clap::Parser;

/// Project health monitoring for AI agents
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Output results as JSON
    #[arg(long, conflicts_with_all = ["markdown", "quiet"])]
    json: bool,

    /// Output results as Markdown report
    #[arg(long, conflicts_with_all = ["json", "quiet"])]
    markdown: bool,

    /// Output one-line summary; exit 1 if critical issues exist
    #[arg(long, conflicts_with_all = ["json", "markdown"])]
    quiet: bool,

    /// Compare current snapshot against a previous snapshot ID
    #[arg(long)]
    compare: Option<i64>,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut log_builder = env_logger::Builder::from_default_env();
    if cli.debug {
        log_builder.filter_level(log::LevelFilter::Debug);
    }
    log_builder.init();

    // Compare mode: compare current against a previous snapshot
    if let Some(before_id) = cli.compare {
        let conn = db::init()?;
        let project_path = std::env::current_dir()?;
        let project_path_str = project_path.to_string_lossy().to_string();
        let snapshot_id = db::create_snapshot(&conn, &project_path_str)?;

        // Run current scan to populate the new snapshot
        let has_critical = run::run(cli.json, cli.markdown, cli.quiet)?;

        // Then compare
        let report = compare::compare_snapshots(&conn, before_id, snapshot_id)?;
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
        } else {
            compare::print_comparison(&report);
        }

        if has_critical {
            std::process::exit(1);
        }
        return Ok(());
    }

    let has_critical = run::run(cli.json, cli.markdown, cli.quiet)?;

    if has_critical {
        std::process::exit(1);
    }

    Ok(())
}
