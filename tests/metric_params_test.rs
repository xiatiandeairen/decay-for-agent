//! Tests for `metric::params::compute` per v0.1 plan §5 T8.
//!
//! Each test parses an inline Rust snippet, locates the target function by
//! name, and asserts the parameter count.

use std::fs;
use std::path::PathBuf;

use decay::metric::params;
use decay::parser::{parse_file, ParsedFile, ParsedFunc};
use tempfile::TempDir;

fn write_file(dir: &std::path::Path, rel: &str, content: &str) -> PathBuf {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir -p");
    }
    fs::write(&path, content).expect("write fixture");
    path
}

fn parse_inline(content: &str) -> (TempDir, ParsedFile) {
    let dir = TempDir::new().expect("tempdir");
    let path = write_file(dir.path(), "src.rs", content);
    let parsed = parse_file(&path, dir.path()).expect("parse");
    (dir, parsed)
}

fn find<'a>(p: &'a ParsedFile, name: &str) -> &'a ParsedFunc {
    p.funcs
        .iter()
        .find(|f| f.function.name == name)
        .unwrap_or_else(|| panic!("function `{name}` not found"))
}

fn count(p: &ParsedFile, name: &str) -> u32 {
    let f = find(p, name);
    params::compute(&p.tree, &p.source, f.body_range)
}

// ---------- 5 spec samples ----------

#[test]
fn zero_params() {
    // fn f() {} → 0
    let (_d, p) = parse_inline("fn f() {}\n");
    assert_eq!(count(&p, "f"), 0);
}

#[test]
fn one_param() {
    // fn f(x: i32) {} → 1
    let (_d, p) = parse_inline("fn f(x: i32) {}\n");
    assert_eq!(count(&p, "f"), 1);
}

#[test]
fn five_params() {
    // fn f(a, b, c, d, e) {} → 5
    let (_d, p) = parse_inline("fn f(a: i32, b: i32, c: i32, d: i32, e: i32) {}\n");
    assert_eq!(count(&p, "f"), 5);
}

#[test]
fn self_plus_args() {
    // impl Foo { fn m(&self, a, b) } → 3 (self + a + b)
    let src = "struct Foo;\nimpl Foo { fn m(&self, a: i32, b: String) {} }\n";
    let (_d, p) = parse_inline(src);
    assert_eq!(count(&p, "m"), 3);
}

#[test]
fn generics_do_not_count() {
    // fn f<T: Clone>(x: T) {} → 1 (generics excluded)
    let (_d, p) = parse_inline("fn f<T: Clone>(x: T) {}\n");
    assert_eq!(count(&p, "f"), 1);
}
