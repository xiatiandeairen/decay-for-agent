//! Cyclomatic-complexity tests per v0.1 plan §5 T6.
//!
//! Each fixture is parsed via the project parser and the first extracted
//! `ParsedFunc` is fed to `metric::cyclomatic::compute`. Expected values are
//! hand-computed in each test's comment; the spec is in the T6 brief.

use std::fs;
use std::path::PathBuf;

use decay::metric::cyclomatic;
use tempfile::TempDir;

/// Parse `src` as a single-file project and compute cyclomatic on the first
/// extracted function. Panics on parse failure (these fixtures are valid Rust).
fn cyclo(src: &str) -> u32 {
    let dir = TempDir::new().expect("tempdir");
    let path: PathBuf = dir.path().join("src.rs");
    fs::write(&path, src).expect("write fixture");
    let parsed = decay::parser::parse_file(&path, dir.path()).expect("parse");
    let func = parsed
        .funcs
        .first()
        .expect("at least one function in fixture");
    cyclomatic::compute(&parsed.tree, &parsed.source, func.body_range)
}

// 1) plain: base path only -> 1
#[test]
fn plain_function_is_one() {
    let v = cyclo("fn f() {}\n");
    assert_eq!(v, 1, "plain fn body should score 1, got {v}");
}

// 2) single_if: 1 + 1 if -> 2
#[test]
fn single_if_is_two() {
    let v = cyclo("fn f() { if true {} }\n");
    assert_eq!(v, 2, "single if should score 2, got {v}");
}

// 3) match with 3 arms: 1 + (3 - 1) -> 3
#[test]
fn match_three_arms_is_three() {
    let src = "fn f() { let x = 0; match x { 1 => {}, 2 => {}, _ => {} } }\n";
    let v = cyclo(src);
    assert_eq!(v, 3, "match with 3 arms should score 3, got {v}");
}

// 4) try_chain: 1 + 3 `?` -> 4
#[test]
fn three_try_operators_is_four() {
    // Use opaque types so the file parses without external context.
    let src = "fn f() -> Result<(), ()> { a()?; b()?; c()?; Ok(()) }\n\
               fn a() -> Result<(), ()> { Ok(()) }\n\
               fn b() -> Result<(), ()> { Ok(()) }\n\
               fn c() -> Result<(), ()> { Ok(()) }\n";
    // The first function is `f` (top-down order in tree-sitter).
    let v = cyclo(src);
    assert_eq!(v, 4, "fn with three `?` operators should score 4, got {v}");
}

// 5) nested_if_with_and: 1 + 2 if + 1 `&&` -> 4
#[test]
fn nested_if_with_logical_and_is_four() {
    let src = "fn f() { let a = true; let b = true; let c = true; \
               if a && b { if c {} } }\n";
    let v = cyclo(src);
    assert_eq!(
        v, 4,
        "nested if with `&&` should score 4 (1 base + 2 if + 1 &&), got {v}"
    );
}
