use std::cmp::Ordering;
use std::collections::HashMap;

use crate::metric;
use crate::types::{DiffEntry, DiffKind, Function, FunctionSet, Metrics};

/// Compare two snapshots and return the functions that warrant a degradation report.
///
/// Reporting policy:
/// - `Added`: function present in `curr` but not in `prev`, with at least one metric ≥ threshold.
/// - `CrossedThreshold`: same `signature_hash`, some metric was below threshold and is now ≥ threshold.
/// - `Worsened`: same `signature_hash`, some metric was already ≥ threshold and increased further.
///
/// Functions whose metrics changed only below threshold, decreased, were removed, or are unchanged
/// are not reported. When a function qualifies under multiple kinds simultaneously, `CrossedThreshold`
/// wins over `Worsened` (a fresh threshold crossing is more salient than further degradation).
///
/// Result is sorted by `max(metric_value - threshold)` descending — the most over-budget function first.
pub fn diff(prev: &FunctionSet, curr: &FunctionSet) -> Vec<DiffEntry> {
    let prev_index: HashMap<u64, &Function> = prev
        .functions
        .iter()
        .map(|f| (f.signature_hash, f))
        .collect();

    let mut entries: Vec<DiffEntry> = Vec::new();

    for func in &curr.functions {
        match prev_index.get(&func.signature_hash) {
            None => {
                if exceeds_any_threshold(func.metrics) {
                    entries.push(DiffEntry {
                        function: func.clone(),
                        previous: None,
                        kind: DiffKind::Added,
                    });
                }
            }
            Some(prev_func) => {
                if let Some(kind) = classify_change(prev_func.metrics, func.metrics) {
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
        let ea = max_excess(a.function.metrics);
        let eb = max_excess(b.function.metrics);
        eb.cmp(&ea).then(Ordering::Equal)
    });

    entries
}

/// Returns true when at least one metric is over its threshold.
fn exceeds_any_threshold(m: Metrics) -> bool {
    metric::active_values(m).any(|(def, value)| metric::breaches_threshold(value, def.threshold))
}

/// Decide which `DiffKind` (if any) applies for a function present in both snapshots.
///
/// Crossed wins over Worsened: a metric that just crossed is the more important signal.
fn classify_change(prev: Metrics, curr: Metrics) -> Option<DiffKind> {
    let mut crossed = false;
    let mut worsened = false;

    for def in metric::ACTIVE_METRICS {
        let p = prev.value(def.id);
        let c = curr.value(def.id);
        if metric::crossed_threshold(p, c, def.threshold) {
            crossed = true;
        } else if metric::worsened_over_threshold(p, c, def.threshold) {
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
fn max_excess(m: Metrics) -> i64 {
    metric::active_values(m)
        .map(|(def, value)| value as i64 - def.threshold as i64)
        .max()
        .unwrap_or(0)
}
