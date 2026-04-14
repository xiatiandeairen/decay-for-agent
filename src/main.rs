mod db;

use std::env;

use anyhow::Result;
use clap::Parser;
use db::{create_snapshot, init};

/// Project health monitoring for AI agents
#[derive(Parser)]
#[command(version, about)]
struct Cli {}

fn main() -> Result<()> {
    let cli = Cli::try_parse();

    match cli {
        Ok(_) => {
            let conn = init()?;
            let project_path = env::current_dir()?;
            let snapshot_id = create_snapshot(&conn, &project_path.to_string_lossy())?;
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
