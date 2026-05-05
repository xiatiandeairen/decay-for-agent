//! Tests for `walk` + `parser` per v0.1 plan §5 T3.
//!
//! All fixtures are inline (no shared on-disk fixture; that ships in T11).

use std::fs;
use std::path::PathBuf;

use decay::parser::parse_file;
use decay::walk::{walk_rust_files, walk_rust_files_with_excludes};
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

#[test]
fn walk_respects_custom_basename_and_path_excludes() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "src/keep.rs", "fn keep() {}");
    write_file(dir.path(), "examples/demo.rs", "fn demo() {}");
    write_file(dir.path(), "src/generated/skip.rs", "fn skip() {}");

    let excludes = vec!["examples".to_string(), "src/generated".to_string()];
    let found: Vec<String> = walk_rust_files_with_excludes(dir.path(), &excludes)
        .unwrap()
        .into_iter()
        .map(|p| {
            p.strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    assert_eq!(found, vec!["src/keep.rs"]);
}

#[test]
fn walk_respects_custom_glob_file_excludes() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "src/keep.rs", "fn keep() {}");
    write_file(
        dir.path(),
        "src/generated_test.rs",
        "fn generated_test() {}",
    );
    write_file(dir.path(), "src/nested/also_keep.rs", "fn also_keep() {}");

    let excludes = vec!["src/*_test.rs".to_string()];
    let mut found: Vec<String> = walk_rust_files_with_excludes(dir.path(), &excludes)
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

    assert_eq!(found, vec!["src/keep.rs", "src/nested/also_keep.rs"]);
}

#[test]
fn walk_respects_root_gitignore() {
    let dir = TempDir::new().unwrap();
    write_file(
        dir.path(),
        ".gitignore",
        "generated/\nignored.rs\n/build/*.rs\n",
    );
    write_file(dir.path(), "src/keep.rs", "fn keep() {}");
    write_file(dir.path(), "generated/skip.rs", "fn skip() {}");
    write_file(dir.path(), "src/ignored.rs", "fn ignored() {}");
    write_file(dir.path(), "build/skip.rs", "fn build_skip() {}");

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

    assert_eq!(found, vec!["src/keep.rs"]);
}

#[test]
fn walk_gitignore_negation_reincludes_root_file() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), ".gitignore", "*.rs\n!src/keep.rs\n");
    write_file(dir.path(), "src/keep.rs", "fn keep() {}");
    write_file(dir.path(), "src/skip.rs", "fn skip() {}");

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

    assert_eq!(found, vec!["src/keep.rs"]);
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
    assert_eq!(f.impl_context, "", "free fn carries empty impl_context");
    assert_eq!(f.cfg_context, "", "non-cfg fn carries empty cfg_context");
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
    assert_eq!(new_fn.function.impl_context, "Foo");
}

#[test]
fn parses_trait_default_method() {
    let src = "trait T { fn def(&self) {} }\n";
    let (_d, p) = parse_inline(src);
    assert_eq!(names(&p.funcs), vec!["def"]);
    assert_eq!(p.funcs[0].function.param_types, vec!["&self".to_string()]);
    // trait_item is not an impl block; v0.1 leaves impl_context empty for
    // default methods. Same-name defaults across two traits in one file are
    // a known v0.1 collision (rare in practice).
    assert_eq!(p.funcs[0].function.impl_context, "");
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
fn impl_context_strips_generics() {
    let src = "struct Foo<T>(T);\nimpl<T> Foo<T> { fn id(&self) {} }\n";
    let (_d, p) = parse_inline(src);
    let id = p.funcs.iter().find(|f| f.function.name == "id").unwrap();
    assert_eq!(id.function.impl_context, "Foo");
}

#[test]
fn impl_context_includes_trait() {
    let src = "
struct Foo;
impl std::fmt::Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { Ok(()) }
}
";
    let (_d, p) = parse_inline(src);
    let fmt = p.funcs.iter().find(|f| f.function.name == "fmt").unwrap();
    assert_eq!(fmt.function.impl_context, "std::fmt::Display for Foo");
}

#[test]
fn same_name_methods_in_different_impls_get_distinct_contexts() {
    // The ripgpep collision regression: same name, same params, different
    // impl blocks must surface as distinct impl_context strings.
    let src = "
struct A; struct B; struct C;
impl A { fn ping(&self) {} }
impl B { fn ping(&self) {} }
impl C { fn ping(&self) {} }
";
    let (_d, p) = parse_inline(src);
    let pings: Vec<&str> = p
        .funcs
        .iter()
        .filter(|f| f.function.name == "ping")
        .map(|f| f.function.impl_context.as_str())
        .collect();
    assert_eq!(pings.len(), 3);
    let mut sorted = pings.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["A", "B", "C"]);
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

#[test]
fn extracts_cfg_context_for_free_function() {
    let src = "
#[cfg(any(unix, target_os = \"wasi\"))]
fn imp() {}
";
    let (_d, p) = parse_inline(src);
    let imp = &p.funcs[0].function;
    assert_eq!(imp.cfg_context, "#[cfg(any(unix,target_os=\"wasi\"))]");
}

#[test]
fn extracts_cfg_context_for_impl_method() {
    let src = "
struct Foo;
impl Foo {
    #[cfg(windows)]
    fn from_path(&self) {}
}
";
    let (_d, p) = parse_inline(src);
    let f = &p.funcs[0].function;
    assert_eq!(f.impl_context, "Foo");
    assert_eq!(f.cfg_context, "#[cfg(windows)]");
}

#[test]
fn cfg_context_ignores_non_cfg_attributes() {
    let src = "
#[inline]
#[cfg(unix)]
#[allow(dead_code)]
fn imp() {}
";
    let (_d, p) = parse_inline(src);
    let imp = &p.funcs[0].function;
    assert_eq!(imp.cfg_context, "#[cfg(unix)]");
}

#[test]
fn cfg_context_preserves_multiple_cfg_attributes_in_source_order() {
    let src = "
#[cfg(unix)]
#[cfg(feature = \"cli\")]
fn imp() {}
";
    let (_d, p) = parse_inline(src);
    let imp = &p.funcs[0].function;
    assert_eq!(imp.cfg_context, "#[cfg(unix)]\n#[cfg(feature=\"cli\")]");
}

#[test]
fn test_attribute_marks_function_as_test_like() {
    let src = "
#[test]
fn smoke() {}
";
    let (_d, p) = parse_inline(src);
    assert!(p.funcs[0].is_test_like);
}

#[test]
fn cfg_test_marks_function_as_test_like() {
    let src = "
#[cfg(test)]
fn helper() {}
";
    let (_d, p) = parse_inline(src);
    assert!(p.funcs[0].is_test_like);
}

#[test]
fn cfg_test_module_marks_nested_helpers_as_test_like() {
    let src = "
#[cfg(test)]
mod tests {
    fn helper() {}

    #[test]
    fn smoke() {}
}
";
    let (_d, p) = parse_inline(src);
    assert_eq!(names(&p.funcs), vec!["helper", "smoke"]);
    assert!(p.funcs.iter().all(|f| f.is_test_like));
}

#[test]
fn tests_module_marks_nested_helpers_as_test_like() {
    let src = "
mod tests {
    fn helper() {}
}
";
    let (_d, p) = parse_inline(src);
    assert_eq!(names(&p.funcs), vec!["helper"]);
    assert!(p.funcs[0].is_test_like);
}
