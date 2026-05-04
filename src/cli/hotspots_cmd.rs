use crate::cli::common;
use crate::error::Result;

/// Run `decay hotspots`: scan current tree and list current threshold breaches.
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
    common::print_exceeded(&scan.funcs);

    Ok(0)
}
