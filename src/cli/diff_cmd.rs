//! `decay diff` — compare the two most recent snapshots without writing a new one.
//!
//! Side effects: opens (and may create) the SQLite db, but does not insert rows.
//! Output goes to stdout (program output, mirrors `scan::run`); friendly notices
//! when there is no baseline. Exit code follows §2.8: 0 for any normal run.

use crate::config::{Thresholds, DEFAULT_THRESHOLDS};
use crate::diff;
use crate::error::Result;
use crate::store;
use crate::types::{DiffEntry, DiffKind, Metrics};

/// Run the `decay diff` command.
///
/// Loads the two most recent snapshots for the cwd's canonical path. Returns
/// exit code 0 in every normal case (no baseline, no changes, changes found).
/// DB / IO errors propagate.
pub fn run() -> Result<i32> {
    let project = crate::cli::common::resolve_project()?;

    let conn = store::open_db()?;
    let snaps = store::load_latest_snapshots(&conn, &project.project_id, 2)?;

    println!("decay v{}", env!("CARGO_PKG_VERSION"));

    // §2.8: no baseline → friendly notice, exit 0. Less than 2 snapshots means
    // we cannot diff, so both 0 and 1 collapse to the same message.
    if snaps.len() < 2 {
        println!("No previous snapshot for this project.");
        println!("Run `decay init` to create a baseline snapshot.");
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
pub(crate) fn print_entry(entry: &DiffEntry, thresholds: &Thresholds) {
    let label = match entry.kind {
        DiffKind::Added => "  [new]",
        DiffKind::Worsened => "  [worsened]",
        DiffKind::CrossedThreshold => "",
    };
    let f = &entry.function;
    println!("  {}:{}  {}{}", f.file, f.start_line, f.name, label);

    let breaches =
        collect_metric_lines(&entry.kind, entry.previous.as_ref(), &f.metrics, thresholds);
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
    match (kind, prev) {
        (DiffKind::Added, _) => lines_for_added(curr, t),
        (_, Some(prev)) => lines_for_change(prev, curr, t),
        // Crossed/Worsened without prev shouldn't happen — diff::diff only
        // emits those kinds when prev exists. Defensive empty if it does.
        (_, None) => Vec::new(),
    }
}

/// Per-metric tuple shared by both Added and Change paths.
fn metric_tuples<'a>(curr: &'a Metrics, t: &'a Thresholds) -> [(&'static str, u32, u32); 4] {
    [
        ("nesting", curr.nesting, t.nesting),
        ("cyclomatic", curr.cyclomatic, t.cyclomatic),
        ("cognitive", curr.cognitive, t.cognitive),
        ("params", curr.params, t.params),
    ]
}

/// Newly added function: list every metric currently ≥ threshold.
fn lines_for_added(curr: &Metrics, t: &Thresholds) -> Vec<String> {
    metric_tuples(curr, t)
        .into_iter()
        .filter(|(_, value, threshold)| value >= threshold)
        // \u{26a0} = ⚠
        .map(|(name, value, threshold)| {
            format!("    {}: {} \u{26a0} over (>{})", name, value, threshold)
        })
        .collect()
}

/// Existing function whose metrics rose: list every metric whose value
/// strictly increased, with a marker if the new value crosses or sits over
/// the threshold.
fn lines_for_change(prev: &Metrics, curr: &Metrics, t: &Thresholds) -> Vec<String> {
    let prevs = [prev.nesting, prev.cyclomatic, prev.cognitive, prev.params];
    metric_tuples(curr, t)
        .into_iter()
        .zip(prevs)
        .filter(|((_, value, _), prev_value)| value > prev_value)
        .map(|((name, value, threshold), prev_value)| {
            let delta = value - prev_value;
            let marker = change_marker(prev_value, value, threshold);
            format!(
                "    {}: {}\u{2192}{}  (+{}){}",
                name, prev_value, value, delta, marker
            )
        })
        .collect()
}

fn change_marker(prev_value: u32, value: u32, threshold: u32) -> String {
    if prev_value < threshold && value >= threshold {
        format!(" \u{26a0} crossed (>{})", threshold)
    } else if prev_value >= threshold {
        format!(" \u{26a0} already over (>{})", threshold)
    } else {
        // Increase but still under threshold — don't flag, but still show line.
        String::new()
    }
}
