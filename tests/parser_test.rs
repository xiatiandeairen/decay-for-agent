//! Tests for `walk` + `parser` per v0.1 plan §5 T3.
//!
//! All fixtures are inline (no shared on-disk fixture; that ships in T11).

use std::fs;
use std::path::PathBuf;

use decay::parser::parse_file;
use decay::walk::walk_rust_files;
use tempfile::TempDir;

// ---------- helpers ----------

/// Write `content` to `<dir>/<rel>`, creating parent dirs as needed.
fn write_file(dir: &std::path::Path, rel: &str, content: &str) -> PathBuf {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir -p");
    }
    fs::write(&path, content).expect("write fixture");
    path
}

/// Convenience: write `content` to `src.rs` under a fresh tempdir and parse it
/// against that tempdir as project root. Returns the parsed file.
fn parse_inline(content: &str) -> (TempDir, decay::parser::ParsedFile) {
    let dir = TempDir::new().expect("tempdir");
    let path = write_file(dir.path(), "src.rs", content);
    let parsed = parse_file(&path, dir.path()).expect("parse");
    (dir, parsed)
}

/// Returned function names sorted, for order-insensitive assertions.
fn names(funcs: &[decay::parser::ParsedFunc]) -> Vec<&str> {
    funcs.iter().map(|f| f.function.name.as_str()).collect()
}

// ---------- walk tests ----------

#[test]
fn walk_returns_rust_files_skipping_target_and_git() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "src/a.rs", "fn a() {}");
    write_file(dir.path(), "src/b/c.rs", "fn c() {}");
    write_file(dir.path(), "target/skip.rs", "fn skip() {}");
    write_file(dir.path(), ".git/x.rs", "fn x() {}");
    write_file(dir.path(), "README.md", "not rust");

    let mut found: Vec<String> = walk_rust_files(dir.path())
        .unwrap()
        .into_iter()
        .map(|p| {
            p.strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    found.sort();

    assert_eq!(found, vec!["src/a.rs", "src/b/c.rs"]);
}

#[test]
fn walk_excludes_nested_target_at_any_depth() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "a/keep.rs", "fn k() {}");
    write_file(dir.path(), "a/target/skip.rs", "fn s() {}");
    write_file(dir.path(), "a/b/.git/skip.rs", "fn s() {}");

    let found: Vec<String> = walk_rust_files(dir.path())
        .unwrap()
        .into_iter()
        .map(|p| {
            p.strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    assert_eq!(found, vec!["a/keep.rs"]);
}

// ---------- parser tests ----------

#[test]
fn parses_top_level_fn() {
    let (_d, p) = parse_inline("fn foo() {}\n");
    assert_eq!(p.funcs.len(), 1);
    let f = &p.funcs[0].function;
    assert_eq!(f.name, "foo");
    assert!(f.param_types.is_empty());
    assert_eq!(f.start_line, 1);
}

#[test]
fn parses_impl_method_with_param() {
    let src = "struct Foo;\nimpl Foo { fn new(x: i32) -> Self { Foo } }\n";
    let (_d, p) = parse_inline(src);
    let new_fn = p
        .funcs
        .iter()
        .find(|f| f.function.name == "new")
        .expect("new fn");
    assert_eq!(new_fn.function.param_types, vec!["i32".to_string()]);
}

#[test]
fn parses_trait_default_method() {
    let src = "trait T { fn def(&self) {} }\n";
    let (_d, p) = parse_inline(src);
    assert_eq!(names(&p.funcs), vec!["def"]);
    assert_eq!(p.funcs[0].function.param_types, vec!["&self".to_string()]);
}

#[test]
fn skips_function_signature_item() {
    // Trait method without body must not be extracted.
    let src = "trait T { fn sig(&self); }\n";
    let (_d, p) = parse_inline(src);
    assert!(
        p.funcs.is_empty(),
        "expected no function_item, got {:?}",
        names(&p.funcs)
    );
}

#[test]
fn closures_are_not_independent_functions() {
    let src = "fn outer() { let f = |x| x + 1; let _ = f(2); }\n";
    let (_d, p) = parse_inline(src);
    assert_eq!(names(&p.funcs), vec!["outer"]);
}

#[test]
fn parse_error_returns_err() {
    let dir = TempDir::new().unwrap();
    let path = write_file(dir.path(), "broken.rs", "fn ?? !! {\n");
    match parse_file(&path, dir.path()) {
        Err(decay::error::DecayError::Parse { .. }) => {}
        Err(other) => panic!("expected Parse error, got {other:?}"),
        Ok(_) => panic!("expected Parse error, got Ok"),
    }
}

#[test]
fn lifetimes_are_stripped_from_param_types() {
    let src = "fn f<'a>(x: &'a str) {}\n";
    let (_d, p) = parse_inline(src);
    let f = &p.funcs[0];
    assert_eq!(f.function.param_types, vec!["&str".to_string()]);
}

#[test]
fn self_three_forms_are_canonicalized() {
    let src = "
struct S;
impl S {
    fn a(self) {}
    fn b(&self) {}
    fn c(&mut self) {}
}
";
    let (_d, p) = parse_inline(src);
    let a = p.funcs.iter().find(|f| f.function.name == "a").unwrap();
    let b = p.funcs.iter().find(|f| f.function.name == "b").unwrap();
    let c = p.funcs.iter().find(|f| f.function.name == "c").unwrap();
    assert_eq!(a.function.param_types, vec!["self".to_string()]);
    assert_eq!(b.function.param_types, vec!["&self".to_string()]);
    assert_eq!(c.function.param_types, vec!["&mut self".to_string()]);
}
