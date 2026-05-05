//! Tests for fingerprint::compute (v0.1 plan §2.10).
//!
//! Coverage:
//! 1. idempotent — same input → same output
//! 2. param order sensitive — [A,B] != [B,A]
//! 3. impl_context disambiguates same-name methods (regression for ripgrep
//!    collision: 128 fingerprint clashes from same-name methods on different
//!    structs in the same file).
//! 4. cfg_context disambiguates same-signature functions split across
//!    mutually exclusive `#[cfg(...)]` branches.

use decay::fingerprint;

#[test]
fn idempotent_same_input_same_output() {
    let a = fingerprint::compute(
        "foo.rs",
        "Foo",
        "",
        "bar",
        &["i32".to_string(), "&str".to_string()],
    );
    let b = fingerprint::compute(
        "foo.rs",
        "Foo",
        "",
        "bar",
        &["i32".to_string(), "&str".to_string()],
    );
    assert_eq!(a, b);
}

#[test]
fn param_order_sensitive() {
    let ab = fingerprint::compute("f", "", "", "n", &["A".to_string(), "B".to_string()]);
    let ba = fingerprint::compute("f", "", "", "n", &["B".to_string(), "A".to_string()]);
    assert_ne!(ab, ba);
}

#[test]
fn impl_context_disambiguates_same_name_methods() {
    // Three `fn path_is_symlink(&self)` in the same file but different impl
    // blocks must hash to three distinct values. Without impl_context all
    // three collide and a single snapshot save fails (ripgrep regression).
    let on_a = fingerprint::compute(
        "walk.rs",
        "FileType",
        "",
        "path_is_symlink",
        &["&self".to_string()],
    );
    let on_b = fingerprint::compute(
        "walk.rs",
        "DirEntry",
        "",
        "path_is_symlink",
        &["&self".to_string()],
    );
    let on_c = fingerprint::compute(
        "walk.rs",
        "WalkBuilder",
        "",
        "path_is_symlink",
        &["&self".to_string()],
    );
    assert_ne!(on_a, on_b);
    assert_ne!(on_b, on_c);
    assert_ne!(on_a, on_c);
}

#[test]
fn impl_context_distinguishes_trait_implementations() {
    // `fn fmt(&self, ...)` in `impl Display for X` and `impl Debug for X` must
    // differ — same self type, same fn name, same params.
    let display = fingerprint::compute(
        "x.rs",
        "Display for X",
        "",
        "fmt",
        &["&self".to_string(), "&mutFormatter".to_string()],
    );
    let debug = fingerprint::compute(
        "x.rs",
        "Debug for X",
        "",
        "fmt",
        &["&self".to_string(), "&mutFormatter".to_string()],
    );
    assert_ne!(display, debug);
}

#[test]
fn free_function_uses_empty_impl_context() {
    // Free functions (not in any impl block) carry empty impl_context. The
    // hash is well-defined and stable; this asserts the empty-context path
    // does not collide with a same-name method on some struct named "" — the
    // NUL separator after impl_context guarantees distinctness.
    let free = fingerprint::compute("m.rs", "", "", "helper", &[]);
    let method = fingerprint::compute("m.rs", "Foo", "", "helper", &[]);
    assert_ne!(free, method);
}

#[test]
fn cfg_context_disambiguates_same_signature_functions() {
    let windows = fingerprint::compute(
        "walk.rs",
        "DirEntryRaw",
        "#[cfg(windows)]",
        "from_path",
        &["&Path".to_string()],
    );
    let unix = fingerprint::compute(
        "walk.rs",
        "DirEntryRaw",
        "#[cfg(unix)]",
        "from_path",
        &["&Path".to_string()],
    );
    let fallback = fingerprint::compute(
        "walk.rs",
        "DirEntryRaw",
        "#[cfg(not(any(windows,unix)))]",
        "from_path",
        &["&Path".to_string()],
    );
    assert_ne!(windows, unix);
    assert_ne!(unix, fallback);
    assert_ne!(windows, fallback);
}
