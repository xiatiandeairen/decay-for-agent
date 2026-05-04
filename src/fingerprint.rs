//! Function fingerprint — deterministic xxh3_64 hash of
//! (file, impl_context, cfg_context, name, param_types).
//!
//! Why: `std::DefaultHasher` is not stable across processes / Rust versions, so
//! snapshots saved on one run would not match on the next. v0.1 plan §2.10
//! mandates xxh3_64 with NUL-separated fields.
//!
//! `impl_context` distinguishes same-name methods across different `impl`
//! blocks (e.g. three `fn path_is_symlink(&self)` on different structs in the
//! same file). Without it, ripgrep alone produces 128 fingerprint collisions.
//!
//! `cfg_context` distinguishes same-signature functions split across mutually
//! exclusive `#[cfg(...)]` branches. Without it, ripgrep still has 16
//! collisions after impl-context disambiguation.

use xxhash_rust::xxh3::xxh3_64;

/// Compute a deterministic 64-bit fingerprint identifying a function across snapshots.
///
/// NUL byte (`0x00`) separates fields so that ("ab","c") and ("a","bc") never collide.
/// Order is significant: file, then impl_context, then cfg_context, then
/// name, then each param_type in given order.
///
/// Pure: no IO, no allocation beyond a single Vec, no global state.
/// Stable: same input → same output across processes, machines, Rust versions
/// (xxh3_64 is a stable spec, see xxhash-rust crate).
pub fn compute(
    file: &str,
    impl_context: &str,
    cfg_context: &str,
    name: &str,
    param_types: &[String],
) -> u64 {
    let mut bytes = Vec::with_capacity(128);
    bytes.extend_from_slice(file.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(impl_context.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(cfg_context.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(name.as_bytes());
    bytes.push(0);
    for t in param_types {
        bytes.extend_from_slice(t.as_bytes());
        bytes.push(0);
    }
    xxh3_64(&bytes)
}
