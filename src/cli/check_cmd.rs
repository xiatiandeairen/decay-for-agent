use crate::cli::common;
use crate::config::DEFAULT_THRESHOLDS;
use crate::diff;
use crate::error::Result;
use crate::store;
use crate::types::Snapshot;

/// Run `decay check`: compare the current tree against the latest baseline
/// snapshot without saving a new snapshot.
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
    let snaps = store::load_latest_snapshots(&conn, &project.project_id, args.scope.as_str(), 1)?;
    if snaps.is_empty() {
        println!("No baseline snapshot for this project.");
        println!("Run `decay init` to create one for scope `{}`.", args.scope.as_str());
        return Ok(0);
    }

    let baseline = &snaps[0];
    let current = Snapshot {
        id: 0,
        project_id: project.project_id,
        scope: args.scope.as_str().to_string(),
        created_at: 0,
        functions: scan.funcs,
    };
    let diffs = diff::diff(baseline, &current, &DEFAULT_THRESHOLDS);

    println!("Check: current tree vs snapshot #{}", baseline.id);

    if diffs.is_empty() {
        println!();
        println!("\u{2713} No functions degraded compared to the latest baseline.");
        return Ok(0);
    }

    println!();
    println!("{} functions degraded:", diffs.len());
    println!();
    for entry in &diffs {
        crate::cli::diff_cmd::print_entry(entry, &DEFAULT_THRESHOLDS);
    }

    Ok(0)
}
