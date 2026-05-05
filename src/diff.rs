use std::cmp::Ordering;
use std::collections::HashMap;

use crate::config::Thresholds;
use crate::types::{DiffEntry, DiffKind, Function, Metrics, Snapshot};

/// Compare two snapshots and return the functions that warrant a degradation report.
///
/// Reporting policy (see plans/v0.1.md §2.9):
/// - `Added`: function present in `curr` but not in `prev`, with at least one metric ≥ threshold.
/// - `CrossedThreshold`: same `signature_hash`, some metric was below threshold and is now ≥ threshold.
/// - `Worsened`: same `signature_hash`, some metric was already ≥ threshold and increased further.
///
/// Functions whose metrics changed only below threshold, decreased, were removed, or are unchanged
/// are not reported. When a function qualifies under multiple kinds simultaneously, `CrossedThreshold`
/// wins over `Worsened` (a fresh threshold crossing is more salient than further degradation).
///
/// Result is sorted by `max(metric_value - threshold)` descending — the most over-budget function first.
pub fn diff(prev: &Snapshot, curr: &Snapshot, thresholds: &Thresholds) -> Vec<DiffEntry> {
    let prev_index: HashMap<u64, &Function> = prev
        .functions
        .iter()
        .map(|f| (f.signature_hash, f))
        .collect();

    let mut entries: Vec<DiffEntry> = Vec::new();

    for func in &curr.functions {
        match prev_index.get(&func.signature_hash) {
            None => {
                if exceeds_any_threshold(&func.metrics, thresholds) {
                    entries.push(DiffEntry {
                        function: func.clone(),
                        previous: None,
                        kind: DiffKind::Added,
                    });
                }
            }
            Some(prev_func) => {
                if let Some(kind) = classify_change(&prev_func.metrics, &func.metrics, thresholds) {
                    entries.push(DiffEntry {
                        function: func.clone(),
                        previous: Some(prev_func.metrics),
                        kind,
                    });
                }
            }
        }
    }

    entries.sort_by(|a, b| {
        let ea = max_excess(&a.function.metrics, thresholds);
        let eb = max_excess(&b.function.metrics, thresholds);
        eb.cmp(&ea).then(Ordering::Equal)
    });

    entries
}

/// Returns true when at least one metric is at or above its threshold.
fn exceeds_any_threshold(m: &Metrics, t: &Thresholds) -> bool {
    m.nesting >= t.nesting
        || m.cyclomatic >= t.cyclomatic
        || m.cognitive >= t.cognitive
        || m.params >= t.params
        || m.statement_count >= t.statement_count
        || m.max_condition_ops >= t.max_condition_ops
        || m.mutable_bindings >= t.mutable_bindings
}

/// Decide which `DiffKind` (if any) applies for a function present in both snapshots.
///
/// Crossed wins over Worsened: a metric that just crossed is the more important signal.
fn classify_change(prev: &Metrics, curr: &Metrics, t: &Thresholds) -> Option<DiffKind> {
    let mut crossed = false;
    let mut worsened = false;

    let pairs = [
        (prev.nesting, curr.nesting, t.nesting),
        (prev.cyclomatic, curr.cyclomatic, t.cyclomatic),
        (prev.cognitive, curr.cognitive, t.cognitive),
        (prev.params, curr.params, t.params),
        (
            prev.statement_count,
            curr.statement_count,
            t.statement_count,
        ),
        (
            prev.max_condition_ops,
            curr.max_condition_ops,
            t.max_condition_ops,
        ),
        (
            prev.mutable_bindings,
            curr.mutable_bindings,
            t.mutable_bindings,
        ),
    ];

    for (p, c, th) in pairs {
        if p < th && c >= th {
            crossed = true;
        } else if p >= th && c > p {
            worsened = true;
        }
    }

    if crossed {
        Some(DiffKind::CrossedThreshold)
    } else if worsened {
        Some(DiffKind::Worsened)
    } else {
        None
    }
}

/// Compute `max(metric_value - threshold)` across all metrics, saturating at 0.
/// Used as the sort key for descending ordering.
fn max_excess(m: &Metrics, t: &Thresholds) -> i64 {
    let candidates = [
        m.nesting as i64 - t.nesting as i64,
        m.cyclomatic as i64 - t.cyclomatic as i64,
        m.cognitive as i64 - t.cognitive as i64,
        m.params as i64 - t.params as i64,
        m.statement_count as i64 - t.statement_count as i64,
        m.max_condition_ops as i64 - t.max_condition_ops as i64,
        m.mutable_bindings as i64 - t.mutable_bindings as i64,
    ];
    candidates.into_iter().max().unwrap_or(0)
}
