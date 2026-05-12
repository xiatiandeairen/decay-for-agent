//! Git history evidence contract tests.
//!
//! History evidence should be production-scoped, rename-aware, and non-blocking
//! when git history collection fails.

use std::fs;
use std::path::Path;
use std::process::Command;

use decay::evidence::history::{detect_history, HistoryReport};

fn init_repo(project: &Path) {
    fs::create_dir_all(project.join("src")).expect("create src");
    run(project, &["git", "init"]);
    run(
        project,
        &["git", "config", "user.email", "decay@example.test"],
    );
    run(project, &["git", "config", "user.name", "Decay Test"]);
}

fn run(project: &Path, args: &[&str]) {
    let (bin, rest) = args.split_first().expect("command");
    let output = Command::new(bin)
        .args(rest)
        .current_dir(project)
        .output()
        .expect("run command");
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit_file(project: &Path, path: &str, body: &str, message: &str) {
    let abs = project.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(abs, body).expect("write file");
    run(project, &["git", "add", path]);
    run(project, &["git", "commit", "--no-verify", "-m", message]);
}

fn commit_all(project: &Path, message: &str) {
    run(project, &["git", "add", "."]);
    run(project, &["git", "commit", "--no-verify", "-m", message]);
}

fn write_file(project: &Path, path: &str, body: &str) {
    let abs = project.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(abs, body).expect("write file");
}

fn rust_body(version: usize) -> String {
    format!(
        r#"
pub fn value(input: i32) -> i32 {{
    if input > {version} {{
        input + {version}
    }} else {{
        input
    }}
}}
"#
    )
}

fn history(project: &Path, paths: &[&str]) -> HistoryReport {
    detect_history(project, paths)
}

fn kinds(report: &HistoryReport) -> Vec<&str> {
    report
        .records
        .iter()
        .map(|record| record.kind.as_str())
        .collect()
}

fn assert_has(report: &HistoryReport, expected: &str) {
    let actual = kinds(report);
    assert!(
        actual.iter().any(|kind| *kind == expected),
        "expected {expected:?}, got {:#?}",
        report
    );
}

fn assert_lacks(report: &HistoryReport, unexpected: &str) {
    let actual = kinds(report);
    assert!(
        actual.iter().all(|kind| *kind != unexpected),
        "unexpected {unexpected:?}, got {:#?}",
        report
    );
}

#[test]
fn new_production_file_without_history_does_not_report_history_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    init_repo(&project);
    commit_file(
        &project,
        "src/lib.rs",
        "pub fn seed() -> i32 { 1 }\n",
        "base",
    );

    write_file(&project, "src/new_area.rs", &rust_body(1));
    let report = history(&project, &["src/new_area.rs"]);

    assert!(report.records.is_empty(), "got {report:#?}");
    assert!(report.diagnostics.is_empty(), "got {report:#?}");
}

#[test]
fn frequent_docs_changes_do_not_pollute_code_hotspot_history() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    init_repo(&project);
    commit_file(&project, "src/lib.rs", &rust_body(0), "base");

    for idx in 1..=10 {
        commit_file(
            &project,
            "docs/roadmap.md",
            &format!("# Roadmap\n\nrevision {idx}\n"),
            &format!("docs: update roadmap {idx}"),
        );
    }

    let docs_report = history(&project, &["docs/roadmap.md"]);
    let code_report = history(&project, &["src/lib.rs"]);

    assert!(docs_report.records.is_empty(), "got {docs_report:#?}");
    assert_lacks(&code_report, "churn_hotspot");
    assert_lacks(&code_report, "frequent_rework");
}

#[test]
fn renamed_or_moved_current_path_still_uses_previous_file_history() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    init_repo(&project);

    for idx in 1..=8 {
        commit_file(
            &project,
            "src/old_area.rs",
            &rust_body(idx),
            &format!("refactor: touch old area {idx}"),
        );
    }

    run(
        &project,
        &["git", "mv", "src/old_area.rs", "src/new_area.rs"],
    );
    commit_all(&project, "refactor: move old area");
    write_file(&project, "src/new_area.rs", &rust_body(99));

    let report = history(&project, &["src/new_area.rs"]);

    assert!(
        report.records.iter().any(|record| {
            record.kind == "churn_hotspot"
                && record.file == "src/new_area.rs"
                && record.source.contains("history")
        }),
        "expected current path churn evidence, got {report:#?}"
    );
}

#[test]
fn repeated_recent_changes_to_same_production_file_trigger_churn_hotspot() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    init_repo(&project);

    for idx in 1..=8 {
        commit_file(
            &project,
            "src/lib.rs",
            &rust_body(idx),
            &format!("refactor: tune lib {idx}"),
        );
    }

    write_file(&project, "src/lib.rs", &rust_body(99));
    let report = history(&project, &["src/lib.rs"]);

    assert_has(&report, "churn_hotspot");
    assert!(
        report
            .records
            .iter()
            .any(|record| record.file == "src/lib.rs" && record.source.contains("history")),
        "got {report:#?}"
    );
}

#[test]
fn single_large_change_is_churn_not_frequent_rework() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    init_repo(&project);

    commit_file(&project, "src/lib.rs", &rust_body(0), "feat: add lib");
    let large_body = (0..100)
        .map(|idx| format!("pub fn value_{idx}() -> i32 {{ {idx} }}\n"))
        .collect::<String>();
    commit_file(
        &project,
        "src/lib.rs",
        &large_body,
        "refactor: reshape module once",
    );

    write_file(
        &project,
        "src/lib.rs",
        &(large_body + "\npub fn next() -> i32 { 1 }\n"),
    );
    let report = history(&project, &["src/lib.rs"]);

    assert_has(&report, "churn_hotspot");
    assert_lacks(&report, "frequent_rework");
}

#[test]
fn recently_bugfixed_production_file_touched_again_triggers_recently_bugfixed_area() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    init_repo(&project);

    commit_file(&project, "src/lib.rs", &rust_body(0), "feat: add lib");
    commit_file(
        &project,
        "src/lib.rs",
        &rust_body(1),
        "fix: handle value edge case",
    );
    write_file(&project, "src/lib.rs", &rust_body(2));

    let report = history(&project, &["src/lib.rs"]);

    assert_has(&report, "recently_bugfixed_area");
    assert!(
        report.records.iter().any(|record| {
            record.kind == "recently_bugfixed_area"
                && record.file == "src/lib.rs"
                && record.subject.contains("fix: handle value edge case")
        }),
        "got {report:#?}"
    );
}

#[test]
fn test_only_change_does_not_trigger_production_history_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    init_repo(&project);

    for idx in 1..=10 {
        commit_file(
            &project,
            "tests/lib_test.rs",
            &format!("#[test]\nfn case_{idx}() {{ assert!(true); }}\n"),
            &format!("test: add case {idx}"),
        );
    }

    write_file(
        &project,
        "tests/lib_test.rs",
        "#[test]\nfn changed_case() { assert!(true); }\n",
    );
    let report = history(&project, &["tests/lib_test.rs"]);

    assert!(report.records.is_empty(), "got {report:#?}");
    assert!(report.diagnostics.is_empty(), "got {report:#?}");
}

#[test]
fn git_history_collection_failure_does_not_block_and_emits_diagnostic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let report = history(tmp.path(), &["src/lib.rs"]);

    assert!(report.records.is_empty(), "got {report:#?}");
    assert_eq!(report.diagnostics.len(), 1, "got {report:#?}");
    assert!(
        report.diagnostics[0].contains("failed for src/lib.rs")
            && report.diagnostics[0].contains("not a git repository"),
        "got {report:#?}"
    );
}
