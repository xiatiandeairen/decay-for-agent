//! `decay` (default command): scan cwd → save snapshot → list current
//! threshold-exceeding functions per §2.8.

use std::time::Instant;

use crate::config::{DEFAULT_THRESHOLDS, Thresholds};
use crate::error::{DecayError, Result};
use crate::pipeline;
use crate::store;
use crate::types::{Function, Metrics};

/// Run the default `decay` command.
///
/// Side effects: walks cwd, parses every `.rs` file, writes one row to
/// `snapshots` + N rows to `functions` in the SQLite db. Output goes to
/// stdout (program output, per server.md §4.2 stderr/stdout split — the
/// snapshot summary is the user-visible product, not a log).
pub fn run() -> Result<i32> {
    let project_root = std::env::current_dir().map_err(|source| DecayError::Io {
        path: ".".to_string(),
        source,
    })?;

    // canonicalize to anchor project_id to a stable absolute path (cwd
    // expressed via different relative paths must yield the same project_id).
    let canonical = project_root
        .canonicalize()
        .map_err(|source| DecayError::Io {
            path: project_root.display().to_string(),
            source,
        })?;
    let project_id = canonical.to_string_lossy().to_string();

    let started = Instant::now();
    let funcs = pipeline::scan(&project_root)?;
    let elapsed = started.elapsed();

    // Distinct file count = unique `Function.file` values (project-relative).
    let file_count = {
        let mut files: Vec<&str> = funcs.iter().map(|f| f.file.as_str()).collect();
        files.sort_unstable();
        files.dedup();
        files.len()
    };
    let func_count = funcs.len();

    println!("decay v{}", env!("CARGO_PKG_VERSION"));

    if file_count == 0 {
        // Per §2.8: explicit message when there's nothing to scan. We still
        // exit 0 — friendly notice, not an error.
        println!("No .rs files found in the current directory.");
        return Ok(0);
    }

    println!(
        "Scanned {} files, {} functions in {:.2}s",
        file_count,
        func_count,
        elapsed.as_secs_f64(),
    );
    println!();

    let conn = store::open_db()?;
    let snapshot_id = store::save_snapshot(&conn, &project_id, funcs.clone())?;

    // First snapshot = no other snapshot exists for this project id after the
    // save (load_latest with n=2 returns just the one we just saved).
    let recent = store::load_latest_snapshots(&conn, &project_id, 2)?;
    let is_first = recent.len() == 1;

    if is_first {
        println!(
            "Snapshot #{snapshot_id} saved [first snapshot — run `decay diff` after next change]"
        );
    } else {
        println!("Snapshot #{snapshot_id} saved");
    }
    println!();

    print_exceeded(&funcs);

    Ok(0)
}

/// Print the "K functions exceed threshold" section per §2.8.
///
/// Each function lists every metric that exceeds its threshold, sorted within
/// the function by `value - threshold` descending. Functions are sorted by
/// their single largest overage (same key) descending so the worst offenders
/// surface first.
fn print_exceeded(funcs: &[Function]) {
    let exceeded = collect_exceeded(funcs, &DEFAULT_THRESHOLDS);
    if exceeded.is_empty() {
        println!("\u{2713} All functions within threshold.");
        return;
    }
    print_breach_list(&exceeded);
}

/// Build the (function, breaches) list, sorted by max overage descending so
/// the worst offenders surface first; breaches within a function are likewise
/// sorted by overage so the dominant metric leads its block.
fn collect_exceeded<'a>(
    funcs: &'a [Function],
    thresholds: &Thresholds,
) -> Vec<(&'a Function, Vec<MetricBreach>)> {
    let mut exceeded: Vec<(&Function, Vec<MetricBreach>)> = funcs
        .iter()
        .filter_map(|f| {
            let breaches = collect_breaches(&f.metrics, thresholds);
            (!breaches.is_empty()).then_some((f, breaches))
        })
        .collect();

    for (_, breaches) in exceeded.iter_mut() {
        breaches.sort_by(|a, b| b.overage.cmp(&a.overage));
    }
    exceeded.sort_by(|a, b| {
        let am = a.1.iter().map(|m| m.overage).max().unwrap_or(0);
        let bm = b.1.iter().map(|m| m.overage).max().unwrap_or(0);
        bm.cmp(&am)
    });
    exceeded
}

fn print_breach_list(exceeded: &[(&Function, Vec<MetricBreach>)]) {
    println!("{} functions exceed threshold:", exceeded.len());
    println!();
    for (f, breaches) in exceeded {
        println!("  {}:{}  {}", f.file, f.start_line, f.name);
        for b in breaches {
            // \u{26A0} = ⚠ (warning sign). Kept as escape so file stays ASCII.
            println!(
                "    {}: {} \u{26a0} (>{})",
                b.metric, b.value, b.threshold
            );
        }
    }
}

struct MetricBreach {
    metric: &'static str,
    value: u32,
    threshold: u32,
    overage: u32,
}

fn collect_breaches(m: &Metrics, t: &crate::config::Thresholds) -> Vec<MetricBreach> {
    let mut out = Vec::with_capacity(4);
    if m.nesting > t.nesting {
        out.push(MetricBreach {
            metric: "nesting",
            value: m.nesting,
            threshold: t.nesting,
            overage: m.nesting - t.nesting,
        });
    }
    if m.cyclomatic > t.cyclomatic {
        out.push(MetricBreach {
            metric: "cyclomatic",
            value: m.cyclomatic,
            threshold: t.cyclomatic,
            overage: m.cyclomatic - t.cyclomatic,
        });
    }
    if m.cognitive > t.cognitive {
        out.push(MetricBreach {
            metric: "cognitive",
            value: m.cognitive,
            threshold: t.cognitive,
            overage: m.cognitive - t.cognitive,
        });
    }
    if m.params > t.params {
        out.push(MetricBreach {
            metric: "params",
            value: m.params,
            threshold: t.params,
            overage: m.params - t.params,
        });
    }
    out
}
