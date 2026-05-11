use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::metric;
use crate::types::{DiffEntry, DiffKind, Function, FunctionSet, Metrics};

/// Compare two snapshots and return the functions that warrant a degradation report.
///
/// Reporting policy:
/// - `AddedNew`: function present in `curr` but not in `prev`, with at least one metric ≥ threshold.
/// - `AddedMaybeMoved`: function was not matched, but has a path-insensitive identity collision.
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
    let stable_prev_index = unique_stable_index(&prev.functions);
    let stable_prev_presence = stable_presence(&prev.functions);
    let weak_prev_index = unique_weak_index(&prev.functions);
    let weak_prev_presence = weak_presence(&prev.functions);

    let mut entries: Vec<DiffEntry> = Vec::new();

    for func in &curr.functions {
        match prev_index.get(&func.signature_hash) {
            None => {
                if let Some(prev_func) = stable_prev_index.get(&func.stable_signature_hash) {
                    if let Some(kind) = classify_change(prev_func.metrics, func.metrics) {
                        entries.push(DiffEntry {
                            function: func.clone(),
                            previous: Some(prev_func.metrics),
                            kind,
                        });
                    }
                    continue;
                }
                let weak_key = WeakFunctionKey::from(func);
                if let Some(prev_func) = weak_prev_index.get(&weak_key) {
                    if let Some(kind) = classify_change(prev_func.metrics, func.metrics) {
                        entries.push(DiffEntry {
                            function: func.clone(),
                            previous: Some(prev_func.metrics),
                            kind,
                        });
                    }
                    continue;
                }

                if exceeds_any_threshold(func.metrics) {
                    let kind = if func.stable_signature_hash != 0
                        && stable_prev_presence.contains(&func.stable_signature_hash)
                        || weak_prev_presence.contains(&weak_key)
                    {
                        DiffKind::AddedMaybeMoved
                    } else {
                        DiffKind::AddedNew
                    };
                    entries.push(DiffEntry {
                        function: func.clone(),
                        previous: None,
                        kind,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WeakFunctionKey {
    impl_context: String,
    cfg_context: String,
    name: String,
    params: u32,
}

impl From<&Function> for WeakFunctionKey {
    fn from(f: &Function) -> Self {
        Self {
            impl_context: f.impl_context.clone(),
            cfg_context: f.cfg_context.clone(),
            name: f.name.clone(),
            params: f.metrics.params,
        }
    }
}

fn unique_stable_index(functions: &[Function]) -> HashMap<u64, &Function> {
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for f in functions {
        if f.stable_signature_hash != 0 {
            *counts.entry(f.stable_signature_hash).or_insert(0) += 1;
        }
    }

    functions
        .iter()
        .filter(|f| f.stable_signature_hash != 0)
        .filter(|f| counts.get(&f.stable_signature_hash) == Some(&1))
        .map(|f| (f.stable_signature_hash, f))
        .collect()
}

fn stable_presence(functions: &[Function]) -> HashSet<u64> {
    functions
        .iter()
        .filter(|f| f.stable_signature_hash != 0)
        .map(|f| f.stable_signature_hash)
        .collect()
}

fn unique_weak_index(functions: &[Function]) -> HashMap<WeakFunctionKey, &Function> {
    let mut counts: HashMap<WeakFunctionKey, usize> = HashMap::new();
    for f in functions {
        *counts.entry(WeakFunctionKey::from(f)).or_insert(0) += 1;
    }

    functions
        .iter()
        .filter(|f| counts.get(&WeakFunctionKey::from(*f)) == Some(&1))
        .map(|f| (WeakFunctionKey::from(f), f))
        .collect()
}

fn weak_presence(functions: &[Function]) -> HashSet<WeakFunctionKey> {
    functions.iter().map(WeakFunctionKey::from).collect()
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
