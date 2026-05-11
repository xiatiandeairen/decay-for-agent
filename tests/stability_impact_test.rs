use std::collections::BTreeSet;

use decay::evidence::stability::detect_stability;
use decay::evidence::EvidenceRecord;

fn lines_containing(text: &str, needles: &[&str]) -> BTreeSet<u32> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            needles
                .iter()
                .any(|needle| line.contains(needle))
                .then_some(index as u32 + 1)
        })
        .collect()
}

fn detect(
    path: &str,
    before: Option<&str>,
    after: Option<&str>,
    added_needles: &[&str],
    touched_needles: &[&str],
) -> Vec<EvidenceRecord> {
    let after_text = after.unwrap_or_default();
    let added_lines = lines_containing(after_text, added_needles);
    let touched_lines = lines_containing(after_text, touched_needles);

    detect_stability(path, before, after, &added_lines, &touched_lines)
}

#[test]
fn touched_ffi_declaration_does_not_claim_bug() {
    let before = r#"
extern "C" {
    pub fn sqlite3_close(handle: *mut std::ffi::c_void) -> i32;
}
"#;
    let after = r#"
extern "C" {
    pub fn sqlite3_close(handle: *mut std::ffi::c_void) -> i32;
}
"#;

    let records = detect(
        "src/ffi.rs",
        Some(before),
        Some(after),
        &[],
        &["pub fn sqlite3_close"],
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, "ffi_boundary_touched");
    assert!(!records[0].kind.contains("bug"));
}

#[test]
fn private_function_signature_change_is_not_public_api_evidence() {
    let before = r#"
fn normalize(input: &str) -> String {
    input.trim().to_string()
}
"#;
    let after = r#"
fn normalize(input: &str, strict: bool) -> String {
    if strict { input.trim().to_string() } else { input.to_string() }
}
"#;

    let records = detect(
        "src/lib.rs",
        Some(before),
        Some(after),
        &[],
        &["fn normalize"],
    );

    assert!(records.is_empty());
}

#[test]
fn public_function_signature_change_is_stability_evidence() {
    let before = r#"
pub fn parse(input: &str) -> usize {
    input.len()
}
"#;
    let after = r#"
pub fn parse(input: &str, strict: bool) -> usize {
    if strict { input.trim().len() } else { input.len() }
}
"#;

    let records = detect(
        "src/lib.rs",
        Some(before),
        Some(after),
        &[],
        &["pub fn parse"],
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, "public_api_signature_changed");
    assert_eq!(records[0].file, "src/lib.rs");
    assert_eq!(records[0].subject, "parse");
    assert_eq!(records[0].source, "stability");
}

#[test]
fn unsafe_evidence_only_reports_added_unsafe_not_untouched_old_unsafe() {
    let before = r#"
pub fn old_path(ptr: *const u8) -> u8 {
    unsafe { *ptr }
}
"#;
    let after = r#"
pub fn old_path(ptr: *const u8) -> u8 {
    unsafe { *ptr }
}

pub fn new_path(ptr: *const u8) -> u8 {
    unsafe { *ptr.add(1) }
}
"#;

    let records = detect(
        "src/lib.rs",
        Some(before),
        Some(after),
        &["unsafe { *ptr.add"],
        &["unsafe { *ptr", "unsafe { *ptr.add"],
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, "unsafe_added");
    assert_eq!(records[0].subject, "line 7");
}

#[test]
fn unwrap_expect_and_panic_evidence_only_reports_added_calls() {
    let before = r#"
pub fn old_parse(input: Option<&str>) -> &str {
    input.unwrap()
}
"#;
    let after = r#"
pub fn old_parse(input: Option<&str>) -> &str {
    input.unwrap()
}

pub fn new_parse(input: Option<&str>) -> &str {
    let value = input.expect("value");
    if value.is_empty() {
        panic!("empty");
    }
    value
}
"#;

    let records = detect(
        "src/lib.rs",
        Some(before),
        Some(after),
        &["expect(\"value\")", "panic!(\"empty\")"],
        &["unwrap()", "expect(\"value\")", "panic!(\"empty\")"],
    );

    assert_eq!(
        records
            .iter()
            .map(|record| record.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["panic_like_added", "panic_like_added"]
    );
    assert_eq!(
        records
            .iter()
            .map(|record| record.subject.as_str())
            .collect::<Vec<_>>(),
        vec!["line 7", "line 9"]
    );
}

#[test]
fn storage_migration_sql_is_stability_evidence() {
    let after = r#"
pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "ALTER TABLE users ADD COLUMN status TEXT NOT NULL DEFAULT 'active'",
        [],
    )?;
    Ok(())
}
"#;

    let records = detect(
        "src/store.rs",
        None,
        Some(after),
        &["ALTER TABLE users"],
        &["ALTER TABLE users"],
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, "storage_migration_touched");
    assert_eq!(records[0].subject, "line 4");
}

#[test]
fn serde_public_struct_field_change_is_stability_evidence() {
    let before = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub name: String,
}
"#;
    let after = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub name: String,
    pub timeout_ms: u64,
}
"#;

    let records = detect(
        "src/config.rs",
        Some(before),
        Some(after),
        &["pub timeout_ms"],
        &["pub timeout_ms"],
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, "protocol_shape_changed");
    assert_eq!(records[0].subject, "Config::timeout_ms");
}

#[test]
fn markdown_mentions_of_unsafe_do_not_trigger_stability_evidence() {
    let after = r#"
# Notes

```rust
unsafe { call_ffi() }
```
"#;

    let records = detect(
        "docs/notes.md",
        None,
        Some(after),
        &["unsafe { call_ffi"],
        &["unsafe { call_ffi"],
    );

    assert!(records.is_empty());
}
