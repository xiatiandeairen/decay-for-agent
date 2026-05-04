use crate::cli::common;
use crate::error::Result;
use crate::store;

/// Run `decay init`: scan current tree and save a fresh baseline snapshot.
pub fn run(args: &common::ScanArgs) -> Result<i32> {
    let project = common::resolve_project()?;
    let scan = common::scan_current(&project.root, args)?;

    println!("decay v{}", env!("CARGO_PKG_VERSION"));

    if scan.file_count == 0 {
        println!("No .rs files found in the current directory.");
        return Ok(0);
    }

    common::print_scan_summary(&scan);
    println!();

    let conn = store::open_db()?;
    let snapshot_id = store::save_snapshot(&conn, &project.project_id, scan.funcs.clone())?;
    let exceeded = common::collect_exceeded(&scan.funcs, &crate::config::DEFAULT_THRESHOLDS);

    println!("Baseline snapshot #{} saved.", snapshot_id);
    println!("{} functions currently exceed threshold.", exceeded.len());
    println!("Run `decay hotspots` to inspect them.");
    println!("Run `decay check` after your next change.");

    Ok(0)
}
