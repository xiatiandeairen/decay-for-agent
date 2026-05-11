//! v0.2 evidence-first `decay diff` behavior.
//!
//! These tests intentionally use real git repositories. The product value
//! depends on diff-aware evidence, so parser-only mocks are not enough.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::Command;
use predicates::prelude::*;

fn decay_cmd(project: &Path, db: &Path) -> Command {
    let mut cmd = Command::cargo_bin("decay").expect("cargo bin");
    cmd.current_dir(project)
        .env("DECAY_DB_PATH", db)
        .env("RUST_LOG", "off");
    cmd
}

fn init_repo(project: &Path) {
    fs::create_dir_all(project.join("src")).expect("create src");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='impact-fixture'\nversion='0.1.0'\nedition='2021'\n",
    )
    .expect("write Cargo.toml");
    run(project, &["git", "init"]);
    run(
        project,
        &["git", "config", "user.email", "decay@example.test"],
    );
    run(project, &["git", "config", "user.name", "Decay Test"]);
}

fn commit_all(project: &Path, message: &str) {
    run(project, &["git", "add", "."]);
    run(project, &["git", "commit", "--no-verify", "-m", message]);
}

fn run(project: &Path, args: &[&str]) {
    let (bin, rest) = args.split_first().expect("command");
    let output = StdCommand::new(bin)
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

fn write_lib(project: &Path, body: &str) {
    fs::write(project.join("src/lib.rs"), body).expect("write lib.rs");
}

fn base_project() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    write_lib(
        &project,
        r#"
pub fn old_debt(x: i32) -> i32 {
    let mut out = 0;
    if x > 0 { out += 1; }
    if x > 1 { out += 1; }
    if x > 2 { out += 1; }
    if x > 3 { out += 1; }
    if x > 4 { out += 1; }
    if x > 5 { out += 1; }
    if x > 6 { out += 1; }
    if x > 7 { out += 1; }
    if x > 8 { out += 1; }
    if x > 9 { out += 1; }
    if x > 10 { out += 1; }
    if x > 11 { out += 1; }
    if x > 12 { out += 1; }
    if x > 13 { out += 1; }
    if x > 14 { out += 1; }
    out
}

pub fn target(x: i32) -> i32 {
    if x > 0 {
        return x;
    }
    0
}

pub fn improves(x: i32) -> i32 {
    if x > 0 { return 1; }
    if x > 1 { return 2; }
    if x > 2 { return 3; }
    if x > 3 { return 4; }
    if x > 4 { return 5; }
    if x > 5 { return 6; }
    if x > 6 { return 7; }
    if x > 7 { return 8; }
    if x > 8 { return 9; }
    if x > 9 { return 10; }
    if x > 10 { return 11; }
    if x > 11 { return 12; }
    0
}
"#,
    );
    commit_all(&project, "base");
    (tmp, project, db)
}

#[test]
fn diff_ignores_untouched_old_complexity_debt() {
    let (_tmp, project, db) = base_project();
    write_lib(
        &project,
        r#"
pub fn old_debt(x: i32) -> i32 {
    let mut out = 0;
    if x > 0 { out += 1; }
    if x > 1 { out += 1; }
    if x > 2 { out += 1; }
    if x > 3 { out += 1; }
    if x > 4 { out += 1; }
    if x > 5 { out += 1; }
    if x > 6 { out += 1; }
    if x > 7 { out += 1; }
    if x > 8 { out += 1; }
    if x > 9 { out += 1; }
    if x > 10 { out += 1; }
    if x > 11 { out += 1; }
    if x > 12 { out += 1; }
    if x > 13 { out += 1; }
    if x > 14 { out += 1; }
    out
}

pub fn target(x: i32) -> i32 {
    if x > 0 { return 1; }
    if x > 1 { return 2; }
    if x > 2 { return 3; }
    if x > 3 { return 4; }
    if x > 4 { return 5; }
    if x > 5 { return 6; }
    if x > 6 { return 7; }
    if x > 7 { return 8; }
    if x > 8 { return 9; }
    if x > 9 { return 10; }
    if x > 10 { return 11; }
    if x > 11 { return 12; }
    if x > 12 { return 13; }
    if x > 13 { return 14; }
    if x > 14 { return 15; }
    0
}

pub fn improves(x: i32) -> i32 {
    if x > 0 { return 1; }
    if x > 1 { return 2; }
    if x > 2 { return 3; }
    if x > 3 { return 4; }
    if x > 4 { return 5; }
    if x > 5 { return 6; }
    if x > 6 { return 7; }
    if x > 7 { return 8; }
    if x > 8 { return 9; }
    if x > 9 { return 10; }
    if x > 10 { return 11; }
    if x > 11 { return 12; }
    0
}
"#,
    );

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Complexity impact:"))
        .stdout(predicate::str::contains("kind=worsened"))
        .stdout(predicate::str::contains("subject=function"))
        .stdout(predicate::str::contains("source=diff_hunk+ast_snapshot"))
        .stdout(predicate::str::contains("target"))
        .stdout(predicate::str::contains("old_debt").not());
}

#[test]
fn diff_reports_improved_complexity_as_positive_evidence() {
    let (_tmp, project, db) = base_project();
    write_lib(
        &project,
        r#"
pub fn old_debt(x: i32) -> i32 {
    let mut out = 0;
    if x > 0 { out += 1; }
    if x > 1 { out += 1; }
    if x > 2 { out += 1; }
    if x > 3 { out += 1; }
    if x > 4 { out += 1; }
    if x > 5 { out += 1; }
    if x > 6 { out += 1; }
    if x > 7 { out += 1; }
    if x > 8 { out += 1; }
    if x > 9 { out += 1; }
    if x > 10 { out += 1; }
    if x > 11 { out += 1; }
    if x > 12 { out += 1; }
    if x > 13 { out += 1; }
    if x > 14 { out += 1; }
    out
}

pub fn target(x: i32) -> i32 {
    if x > 0 {
        return x;
    }
    0
}

pub fn improves(x: i32) -> i32 {
    if x > 0 {
        return x;
    }
    0
}
"#,
    );

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Complexity impact:"))
        .stdout(predicate::str::contains("kind=improved"))
        .stdout(predicate::str::contains("improves"))
        .stdout(predicate::str::contains("old_debt").not());
}

#[test]
fn diff_does_not_report_unchanged_moved_function_as_new_debt() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/old.rs"),
        "pub fn moved(x: i32) -> i32 { if x > 0 { x } else { 0 } }\n",
    )
    .expect("write old.rs");
    fs::write(project.join("src/lib.rs"), "pub mod old;\n").expect("write lib.rs");
    commit_all(&project, "base");

    run(&project, &["git", "mv", "src/old.rs", "src/new.rs"]);
    fs::write(project.join("src/lib.rs"), "pub mod new;\n").expect("write lib.rs");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Complexity impact:"))
        .stdout(predicate::str::contains("kind=new_high_risk").not())
        .stdout(predicate::str::contains("moved").not());
}

#[test]
fn diff_ignores_non_rust_files_for_complexity_evidence() {
    let (_tmp, project, db) = base_project();
    fs::create_dir_all(project.join("docs")).expect("create docs");
    fs::write(project.join("docs/note.md"), "# note\nnot rust\n").expect("write doc");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Complexity impact:"))
        .stdout(predicate::str::contains("changed_functions=0"))
        .stdout(predicate::str::contains("evidence=0"))
        .stdout(predicate::str::contains("diagnostics=0"));
}

#[test]
fn diff_reports_stability_and_quality_evidence_without_claiming_bugs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
#[no_mangle]
pub extern "C" fn mx_send(value: i32) -> i32 {
    value
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "test(base): seed ffi");

    fs::write(
        project.join("src/lib.rs"),
        r#"
#[no_mangle]
pub extern "C" fn mx_send(value: i32) -> i32 {
    let parsed = Some(value).unwrap();
    parsed
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stability impact:"))
        .stdout(predicate::str::contains("kind=ffi_boundary_touched"))
        .stdout(predicate::str::contains("kind=panic_like_added"))
        .stdout(predicate::str::contains("Quality impact:"))
        .stdout(predicate::str::contains("kind=sensitive_test_gap"))
        .stdout(predicate::str::contains("bug").not())
        .stdout(predicate::str::contains("risk=high").not());
}

#[test]
fn diff_does_not_treat_docs_only_changes_as_test_gap_or_stability_evidence() {
    let (_tmp, project, db) = base_project();
    fs::create_dir_all(project.join("docs")).expect("create docs");
    fs::write(
        project.join("docs/note.md"),
        "# unsafe unwrap panic\nThese are words in docs.\n",
    )
    .expect("write docs");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Complexity impact:"))
        .stdout(predicate::str::contains("Stability impact:"))
        .stdout(predicate::str::contains("evidence=0"))
        .stdout(predicate::str::contains("Quality impact:"))
        .stdout(predicate::str::contains("kind=test_gap").not())
        .stdout(predicate::str::contains("kind=panic_like_added").not())
        .stdout(predicate::str::contains("kind=unsafe_added").not());
}
