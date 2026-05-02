//! Hand-crafted samples for `metric::nesting::compute` per v0.1 plan §5 T5.
//!
//! Each test parses an inline Rust snippet via the project's `parser`, takes
//! the first extracted function's `body_range`, calls `nesting::compute`, and
//! asserts the expected depth. Manual derivations are in each test's comment.

use std::fs;
use std::path::PathBuf;

use decay::metric::nesting;
use decay::parser::{parse_file, ParsedFile};
use tempfile::TempDir;

/// Parse `content` as `src.rs` inside a fresh tempdir and return the
/// `ParsedFile`. Tempdir is returned to keep the file alive.
fn parse_inline(content: &str) -> (TempDir, ParsedFile) {
    let dir = TempDir::new().expect("tempdir");
    let path: PathBuf = dir.path().join("src.rs");
    fs::write(&path, content).expect("write fixture");
    let parsed = parse_file(&path, dir.path()).expect("parse");
    (dir, parsed)
}

/// Compute nesting for the first function in `content`.
fn nesting_of(content: &str) -> u32 {
    let (_dir, parsed) = parse_inline(content);
    let first = parsed.funcs.first().expect("at least one function");
    nesting::compute(&parsed.tree, &parsed.source, first.body_range)
}

#[test]
fn plain_function_has_zero_depth() {
    // No control flow at all → max depth observed inside body is 0.
    let src = "fn f() {}";
    assert_eq!(nesting_of(src), 0);
}

#[test]
fn single_if_has_depth_one() {
    // Body { if true {} }
    //   - body is at depth 0
    //   - if's consequence block sits one level deeper → depth 1
    let src = "fn f() { if true {} }";
    assert_eq!(nesting_of(src), 1);
}

#[test]
fn triple_nested_if_while_for_has_depth_three() {
    // Body
    //   if true {        // consequence depth 1
    //     while x() {    // while body depth 2
    //       for y in z {} // for body depth 3
    //     }
    //   }
    let src = "fn f() { if true { while x() { for y in z {} } } }";
    assert_eq!(nesting_of(src), 3);
}

#[test]
fn match_then_if_has_depth_two() {
    // Body
    //   match x {           // match body depth 1
    //     _ => { if y {} }  // arm value lives at depth 1
    //                       // if's consequence then at depth 2
    //   }
    let src = "fn f() { match x { _ => { if y {} } } }";
    assert_eq!(nesting_of(src), 2);
}

#[test]
fn closure_with_inner_if_has_depth_two() {
    // Body
    //   let g = || {       // closure body depth 1
    //     if true {}       // consequence depth 2
    //   };
    // Closure adds one nesting level (per T5 brief, matches cognitive metric).
    let src = "fn f() { let g = || { if true {} }; }";
    assert_eq!(nesting_of(src), 2);
}
