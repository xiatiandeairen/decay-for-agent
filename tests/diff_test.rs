use decay::config::{Thresholds, DEFAULT_THRESHOLDS};
use decay::diff::diff;
use decay::types::{DiffKind, Function, Metrics, Snapshot};

fn make_function(signature_hash: u64, name: &str, metrics: Metrics) -> Function {
    Function {
        file: "src/lib.rs".to_string(),
        name: name.to_string(),
        start_line: 1,
        end_line: 10,
        param_types: Vec::new(),
        signature_hash,
        metrics,
    }
}

fn snapshot(id: i64, functions: Vec<Function>) -> Snapshot {
    Snapshot {
        id,
        project_id: "/tmp/proj".to_string(),
        created_at: 0,
        functions,
    }
}

fn metrics(nesting: u32, cyclomatic: u32, cognitive: u32, params: u32) -> Metrics {
    Metrics {
        nesting,
        cyclomatic,
        cognitive,
        params,
    }
}

fn thresholds() -> Thresholds {
    Thresholds {
        nesting: DEFAULT_THRESHOLDS.nesting,
        cyclomatic: DEFAULT_THRESHOLDS.cyclomatic,
        cognitive: DEFAULT_THRESHOLDS.cognitive,
        params: DEFAULT_THRESHOLDS.params,
    }
}

#[test]
fn added_above_threshold() {
    let prev = snapshot(1, Vec::new());
    // cognitive=20 exceeds default threshold (15)
    let f = make_function(0xAAAA, "complex_fn", metrics(0, 0, 20, 0));
    let curr = snapshot(2, vec![f]);

    let entries = diff(&prev, &curr, &thresholds());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, DiffKind::Added);
    assert!(entries[0].previous.is_none());
    assert_eq!(entries[0].function.name, "complex_fn");
}

#[test]
fn added_below_threshold_filtered() {
    let prev = snapshot(1, Vec::new());
    // cognitive=10 is below default threshold (15) — must not be reported.
    let f = make_function(0xBBBB, "small_fn", metrics(0, 0, 10, 0));
    let curr = snapshot(2, vec![f]);

    let entries = diff(&prev, &curr, &thresholds());

    assert!(entries.is_empty());
}

#[test]
fn crossed_threshold() {
    // Same signature_hash; cognitive 10 → 20 crosses threshold (15).
    let prev_f = make_function(0xCCCC, "fn_a", metrics(0, 0, 10, 0));
    let curr_f = make_function(0xCCCC, "fn_a", metrics(0, 0, 20, 0));

    let prev = snapshot(1, vec![prev_f]);
    let curr = snapshot(2, vec![curr_f]);

    let entries = diff(&prev, &curr, &thresholds());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, DiffKind::CrossedThreshold);
    assert_eq!(entries[0].previous.unwrap().cognitive, 10);
    assert_eq!(entries[0].function.metrics.cognitive, 20);
}

#[test]
fn worsened() {
    // cognitive 20 → 25, both above threshold.
    let prev_f = make_function(0xDDDD, "fn_b", metrics(0, 0, 20, 0));
    let curr_f = make_function(0xDDDD, "fn_b", metrics(0, 0, 25, 0));

    let prev = snapshot(1, vec![prev_f]);
    let curr = snapshot(2, vec![curr_f]);

    let entries = diff(&prev, &curr, &thresholds());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, DiffKind::Worsened);
    assert_eq!(entries[0].previous.unwrap().cognitive, 20);
    assert_eq!(entries[0].function.metrics.cognitive, 25);
}

#[test]
fn below_threshold_change_filtered() {
    // 5 → 8, both below threshold — not reported.
    let prev_f = make_function(0xEEEE, "fn_c", metrics(0, 0, 5, 0));
    let curr_f = make_function(0xEEEE, "fn_c", metrics(0, 0, 8, 0));

    let prev = snapshot(1, vec![prev_f]);
    let curr = snapshot(2, vec![curr_f]);

    let entries = diff(&prev, &curr, &thresholds());

    assert!(entries.is_empty());
}

#[test]
fn dropped_not_reported() {
    // cognitive 20 → 10 (decrease, even crossing threshold downward) — not reported.
    let prev_f = make_function(0xFFFF, "fn_d", metrics(0, 0, 20, 0));
    let curr_f = make_function(0xFFFF, "fn_d", metrics(0, 0, 10, 0));

    let prev = snapshot(1, vec![prev_f]);
    let curr = snapshot(2, vec![curr_f]);

    let entries = diff(&prev, &curr, &thresholds());

    assert!(entries.is_empty());
}

#[test]
fn sort_by_max_excess() {
    // Three functions with different excess over threshold:
    //   - fn_low:  cognitive=16  → excess = 16 - 15 = 1
    //   - fn_high: cognitive=30  → excess = 30 - 15 = 15
    //   - fn_mid:  cognitive=22  → excess = 22 - 15 = 7
    // Expected order: fn_high, fn_mid, fn_low.
    let prev = snapshot(1, Vec::new());

    let f_low = make_function(0x1111, "fn_low", metrics(0, 0, 16, 0));
    let f_high = make_function(0x2222, "fn_high", metrics(0, 0, 30, 0));
    let f_mid = make_function(0x3333, "fn_mid", metrics(0, 0, 22, 0));

    let curr = snapshot(2, vec![f_low, f_high, f_mid]);

    let entries = diff(&prev, &curr, &thresholds());

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].function.name, "fn_high");
    assert_eq!(entries[1].function.name, "fn_mid");
    assert_eq!(entries[2].function.name, "fn_low");
}
