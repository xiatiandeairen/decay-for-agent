//! Tests for `metric::cognitive::compute` per v0.1 plan §5 T7.
//!
//! Tolerance per brief: hand-computed expected values, deviation ≤ 1 accepted.
//! Each case carries the hand-trace in its leading comment so future readers
//! can audit the SonarSource interpretation we encoded in `cognitive.rs`.

use decay::metric::cognitive;
use decay::parser::parse_file;
use std::fs;
use tempfile::TempDir;

/// Parse `content` as `src.rs` in a tempdir, return the cognitive score for
/// the first extracted function. `parser_test.rs` covers the extraction path
/// itself; here we trust it and focus on the metric.
fn cognitive_of(content: &str) -> u32 {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("src.rs");
    fs::write(&path, content).expect("write");
    let parsed = parse_file(&path, dir.path()).expect("parse");
    let func = parsed.funcs.first().expect("at least one function");
    cognitive::compute(&parsed.tree, &parsed.source, func.body_range)
}

/// Assert `actual` is within ±1 of `expected` (brief allows that tolerance).
#[track_caller]
fn assert_near(actual: u32, expected: u32, label: &str) {
    let lo = expected.saturating_sub(1);
    let hi = expected.saturating_add(1);
    assert!(
        actual >= lo && actual <= hi,
        "{label}: got {actual}, expected {expected} (±1, range {lo}..={hi})"
    );
}

// ---------- 5 hand-crafted samples ----------

/// Sample 1 — plain function.
/// Hand-trace: empty body, no control flow → 0.
#[test]
fn cognitive_plain_is_zero() {
    let actual = cognitive_of("fn f() {}");
    assert_eq!(actual, 0, "plain fn should be 0, got {actual}");
}

/// Sample 2 — single `if` at top level.
/// Hand-trace: if at nesting=0 → +1+0 = 1. Empty body → 0. Total = 1.
#[test]
fn cognitive_single_if_is_one() {
    let actual = cognitive_of("fn f() { if a {} }");
    assert_near(actual, 1, "single_if");
}

/// Sample 3 — nested if-if (two levels deep).
/// Hand-trace:
///   outer if at nesting=0 → +1+0 = 1; consequence walked at nesting=1
///     inner if at nesting=1 → +1+1 = 2
/// Total = 1 + 2 = 3.
#[test]
fn cognitive_nested_if_if_is_three() {
    let actual = cognitive_of("fn f() { if a { if b {} } }");
    assert_near(actual, 3, "nested_if_if");
}

/// Sample 4 — match with closure inside an arm.
/// Hand-trace (per T7 brief, expected = 5):
///   match at nesting=0      → +1+0 = 1
///   match arm #2 (`_`)      → +1     (first arm not counted)
///   closure at nesting=0    → +1+0 = 1   (arm contents inherit outer nesting)
///   inner if at nesting=1   → +1+1 = 2   (closure body bumps nesting)
/// Total = 1 + 1 + 1 + 2 = 5. (±1 acceptable.)
#[test]
fn cognitive_match_with_closure_is_about_five() {
    let actual = cognitive_of("fn f() { match x { 1 => || { if a {} }(), _ => () } }");
    assert_near(actual, 5, "match_with_closure");
}

/// Sample 5 — logical chain in `if` condition.
/// Hand-trace:
///   if at nesting=0 → +1
///   `a && b && c` parses as `(a && b) && c` — outer `&&` is the chain head,
///     inner `&&` shares the operator → only +1 for the whole chain.
/// Total = 2.
#[test]
fn cognitive_logical_chain_is_two() {
    let actual = cognitive_of("fn f() { if a && b && c {} }");
    assert_near(actual, 2, "logical_chain");
}
