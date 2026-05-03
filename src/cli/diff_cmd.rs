//! `decay diff` — compare the two most recent snapshots without writing a new one.
//!
//! Side effects: opens (and may create) the SQLite db, but does not insert rows.
//! Output goes to stdout (program output, mirrors `scan::run`); friendly notices
//! when there is no baseline. Exit code follows §2.8: 0 for any normal run.

use crate::config::{DEFAULT_THRESHOLDS, Thresholds};
use crate::diff;
use crate::error::{DecayError, Result};
use crate::store;
use crate::types::{DiffEntry, DiffKind, Metrics};

/// Run the `decay diff` command.
///
/// Loads the two most recent snapshots for the cwd's canonical path. Returns
/// exit code 0 in every normal case (no baseline, no changes, changes found).
/// DB / IO errors propagate.
pub fn run() -> Result<i32> {
    let project_root = std::env::current_dir().map_err(|source| DecayError::Io {
        path: ".".to_string(),
        source,
    })?;
    let canonical = project_root
        .canonicalize()
        .map_err(|source| DecayError::Io {
            path: project_root.display().to_string(),
            source,
        })?;
    let project_id = canonical.to_string_lossy().to_string();

    let conn = store::open_db()?;
    let snaps = store::load_latest_snapshots(&conn, &project_id, 2)?;

    println!("decay v{}", env!("CARGO_PKG_VERSION"));

    // §2.8: no baseline → friendly notice, exit 0. Less than 2 snapshots means
    // we cannot diff, so both 0 and 1 collapse to the same message.
    if snaps.len() < 2 {
        println!("No previous snapshot for this project.");
        println!("Run `decay` to create a baseline snapshot.");
        return Ok(0);
    }

    // load_latest_snapshots returns id DESC, so [0] is current, [1] is previous.
    let curr = &snaps[0];
    let prev = &snaps[1];
    let diffs = diff::diff(prev, curr, &DEFAULT_THRESHOLDS);

    let elapsed_minutes = ((curr.created_at - prev.created_at).max(0)) / 60;
    println!(
        "Diff: snapshot #{} vs #{} ({} minutes ago)",
        curr.id, prev.id, elapsed_minutes,
    );

    if diffs.is_empty() {
        println!();
        println!("\u{2713} No functions degraded since last snapshot.");
        return Ok(0);
    }

    println!();
    println!("{} functions degraded:", diffs.len());
    println!();

    for entry in &diffs {
        print_entry(entry, &DEFAULT_THRESHOLDS);
    }

    Ok(0)
}

/// Print one diff entry: header line + one line per metric that changed (or
/// every threshold-exceeding metric for `Added`).
fn print_entry(entry: &DiffEntry, thresholds: &Thresholds) {
    let label = match entry.kind {
        DiffKind::Added => "  [new]",
        DiffKind::Worsened => "  [worsened]",
        DiffKind::CrossedThreshold => "",
    };
    let f = &entry.function;
    println!("  {}:{}  {}{}", f.file, f.start_line, f.name, label);

    let breaches = collect_metric_lines(&entry.kind, entry.previous.as_ref(), &f.metrics, thresholds);
    for line in breaches {
        println!("{}", line);
    }
}

/// Build per-metric output lines according to §2.8 + §2.9:
/// - `Added`: list every metric ≥ threshold (no prev value).
/// - `CrossedThreshold` / `Worsened`: list every metric whose value increased.
fn collect_metric_lines(
    kind: &DiffKind,
    prev: Option<&Metrics>,
    curr: &Metrics,
    t: &Thresholds,
) -> Vec<String> {
    let mut out = Vec::new();
    let metrics = [
        ("nesting", curr.nesting, t.nesting, prev.map(|m| m.nesting)),
        (
            "cyclomatic",
            curr.cyclomatic,
            t.cyclomatic,
            prev.map(|m| m.cyclomatic),
        ),
        (
            "cognitive",
            curr.cognitive,
            t.cognitive,
            prev.map(|m| m.cognitive),
        ),
        ("params", curr.params, t.params, prev.map(|m| m.params)),
    ];

    match kind {
        DiffKind::Added => {
            for (name, value, threshold, _) in metrics {
                if value >= threshold {
                    // \u{26a0} = ⚠
                    out.push(format!(
                        "    {}: {} \u{26a0} over (>{})",
                        name, value, threshold
                    ));
                }
            }
        }
        DiffKind::CrossedThreshold | DiffKind::Worsened => {
            for (name, value, threshold, prev_value) in metrics {
                let prev_value = match prev_value {
                    Some(v) => v,
                    None => continue,
                };
                if value <= prev_value {
                    continue;
                }
                let delta = value - prev_value;
                let marker = if prev_value < threshold && value >= threshold {
                    format!(" \u{26a0} crossed (>{})", threshold)
                } else if prev_value >= threshold {
                    format!(" \u{26a0} already over (>{})", threshold)
                } else {
                    // Increase but still under threshold — don't flag, but still show line.
                    String::new()
                };
                out.push(format!(
                    "    {}: {}\u{2192}{}  (+{}){}",
                    name, prev_value, value, delta, marker
                ));
            }
        }
    }

    out
}
