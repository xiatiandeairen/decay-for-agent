//! Tests for fingerprint::compute (v0.1 plan §2.10).
//!
//! Coverage:
//! 1. idempotent — same input → same output
//! 2. field order sensitive — file/name not interchangeable
//! 3. cross-process stable — known input → frozen hex value
//! 4. param order sensitive — [A,B] != [B,A]
//! 5. empty vs single-empty-string param_types — NUL separator distinguishes

use decay::fingerprint;

#[test]
fn idempotent_same_input_same_output() {
    let a = fingerprint::compute("foo.rs", "bar", &["i32".to_string(), "&str".to_string()]);
    let b = fingerprint::compute("foo.rs", "bar", &["i32".to_string(), "&str".to_string()]);
    assert_eq!(a, b);
}

#[test]
fn field_order_sensitive_file_vs_name() {
    // ("a","b") and ("b","a") must differ — NUL separator + ordered fields.
    let h1 = fingerprint::compute("a", "b", &[]);
    let h2 = fingerprint::compute("b", "a", &[]);
    assert_ne!(h1, h2);
}

#[test]
fn deterministic_known_value() {
    // Frozen value from first run. Any change (algorithm, byte layout, separator)
    // breaks this on purpose — snapshot DB depends on cross-process stability.
    let h = fingerprint::compute("foo.rs", "bar", &["i32".to_string()]);
    // Frozen on 2026-05-02 with xxhash-rust 0.8 / xxh3_64. If this assertion
    // fails, snapshot DBs from prior runs become unreadable — investigate
    // before bumping the value.
    assert_eq!(h, 0xb80b_b048_1f46_1d18);
}

#[test]
fn param_order_sensitive() {
    let ab = fingerprint::compute("f", "n", &["A".to_string(), "B".to_string()]);
    let ba = fingerprint::compute("f", "n", &["B".to_string(), "A".to_string()]);
    assert_ne!(ab, ba);
}

#[test]
fn empty_vs_single_empty_string_param() {
    // [] hashes name+\0 (no param bytes), while [""] hashes name+\0+\0 (one empty
    // param then its NUL terminator). NUL boundary makes them distinct.
    let none = fingerprint::compute("f", "n", &[]);
    let one_empty = fingerprint::compute("f", "n", &["".to_string()]);
    assert_ne!(none, one_empty);
}
