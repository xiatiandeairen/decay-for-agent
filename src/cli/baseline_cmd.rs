use crate::cli::common;
use crate::error::Result;
use crate::store::{self, SaveBaselineOutcome};

/// Run `decay baseline <version>`: save current tree as a named baseline.
pub fn run(args: &common::ScanArgs, version: &str, replace: bool) -> Result<i32> {
    let project = common::resolve_project()?;
    let scan = common::scan_current(&project.root, args)?;
    let conn = store::open_db()?;
    let outcome = store::save_baseline(
        &conn,
        &project.project_id,
        args.scope.as_str(),
        version,
        scan.funcs.clone(),
        scan.diagnostic_count,
        replace,
    )?;

    if args.verbose {
        print_verbose(args, version, replace, &scan, outcome);
    } else {
        print_concise(args, version, &scan, outcome);
    }

    let code = match outcome {
        SaveBaselineOutcome::ExistsDifferent { .. } => 1,
        _ => 0,
    };
    Ok(code)
}

fn print_concise(
    args: &common::ScanArgs,
    version: &str,
    scan: &common::ScanResult,
    outcome: SaveBaselineOutcome,
) {
    match outcome {
        SaveBaselineOutcome::Created { id } => println!(
            "status=created baseline={} scope={} functions={} partial={} diagnostics={} id={}",
            version,
            args.scope.as_str(),
            scan.funcs.len(),
            scan.diagnostic_count > 0,
            scan.diagnostic_count,
            id
        ),
        SaveBaselineOutcome::Unchanged { id } => println!(
            "status=unchanged baseline={} scope={} functions={} partial={} diagnostics={} id={}",
            version,
            args.scope.as_str(),
            scan.funcs.len(),
            scan.diagnostic_count > 0,
            scan.diagnostic_count,
            id
        ),
        SaveBaselineOutcome::Replaced { id } => println!(
            "status=replaced baseline={} scope={} functions={} partial={} diagnostics={} id={}",
            version,
            args.scope.as_str(),
            scan.funcs.len(),
            scan.diagnostic_count > 0,
            scan.diagnostic_count,
            id
        ),
        SaveBaselineOutcome::ExistsDifferent { id } => println!(
            "status=error reason=baseline_already_exists_with_different_content baseline={} scope={} partial={} diagnostics={} id={} hint=use_--replace",
            version,
            args.scope.as_str(),
            scan.diagnostic_count > 0,
            scan.diagnostic_count,
            id
        ),
    }
}

fn print_verbose(
    args: &common::ScanArgs,
    version: &str,
    replace: bool,
    scan: &common::ScanResult,
    outcome: SaveBaselineOutcome,
) {
    println!("decay v{}", env!("CARGO_PKG_VERSION"));
    println!("Mode: baseline");
    println!("Scope: {}", args.scope.as_str());
    println!("Version: {}", version);
    println!("Replace: {}", replace);
    println!(
        "Scanned: {} files, {} functions in {:.2}s",
        scan.file_count,
        scan.funcs.len(),
        scan.elapsed_secs
    );
    println!("Diagnostics: {}", scan.diagnostic_count);
    println!();

    match outcome {
        SaveBaselineOutcome::Created { id } => {
            println!("Baseline `{}` created with id #{}.", version, id);
        }
        SaveBaselineOutcome::Unchanged { id } => {
            println!(
                "Baseline `{}` is unchanged. Existing id #{} is already current.",
                version, id
            );
        }
        SaveBaselineOutcome::Replaced { id } => {
            println!("Baseline `{}` replaced in-place at id #{}.", version, id);
        }
        SaveBaselineOutcome::ExistsDifferent { id } => {
            println!(
                "Baseline `{}` already exists at id #{} and differs from the current tree.",
                version, id
            );
            println!(
                "Use `decay baseline {} --replace` to overwrite it.",
                version
            );
        }
    }
}
