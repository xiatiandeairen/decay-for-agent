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

fn git_head(project: &Path) -> String {
    let output = StdCommand::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project)
        .output()
        .expect("git rev-parse");
    assert!(
        output.status.success(),
        "git rev-parse failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_short_head(project: &Path) -> String {
    let output = StdCommand::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .current_dir(project)
        .output()
        .expect("git rev-parse short");
    assert!(
        output.status.success(),
        "git rev-parse --short failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn db_count(db: &Path, sql: &str) -> i64 {
    let conn = rusqlite::Connection::open(db).expect("open db");
    conn.query_row(sql, [], |row| row.get(0))
        .expect("count rows")
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
fn diff_git_ref_prints_execution_context_and_top_findings() {
    let (_tmp, project, db) = base_project();
    let base = git_head(&project);
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
    if x > 0 {
        return x;
    }
    0
}
"#,
    );

    decay_cmd(&project, &db)
        .args(["diff", &base])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Diff context:"))
        .stdout(predicate::str::contains("mode=git_ref"))
        .stdout(predicate::str::contains(format!("base={base}")))
        .stdout(predicate::str::contains("head="))
        .stdout(predicate::str::contains("changed_files=1"))
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[high] src/lib.rs:target"));
}

#[test]
fn diff_short_git_ref_prefers_commit_over_baseline_version() {
    let (_tmp, project, db) = base_project();
    let base_short = git_short_head(&project);

    decay_cmd(&project, &db)
        .args(["baseline", &base_short])
        .assert()
        .success();

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
    if x > 0 {
        return x;
    }
    0
}
"#,
    );

    decay_cmd(&project, &db)
        .args(["diff", &base_short])
        .assert()
        .failure()
        .stdout(predicate::str::contains("mode=git_ref"))
        .stdout(predicate::str::contains("warning=short_git_ref_resolved"))
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("target"));
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
fn diff_treats_same_file_same_name_signature_change_as_existing_function() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn render(value: i32) -> i32 {
    if value > 0 { return 1; }
    if value > 1 { return 2; }
    if value > 2 { return 3; }
    if value > 3 { return 4; }
    if value > 4 { return 5; }
    if value > 5 { return 6; }
    if value > 6 { return 7; }
    if value > 7 { return 8; }
    if value > 8 { return 9; }
    if value > 9 { return 10; }
    if value > 10 { return 11; }
    if value > 11 { return 12; }
    0
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "base");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn render(value: i32, fallback: i32) -> i32 {
    if value > 0 { return 1; }
    if value > 1 { return 2; }
    if value > 2 { return 3; }
    if value > 3 { return 4; }
    if value > 4 { return 5; }
    if value > 5 { return 6; }
    if value > 6 { return 7; }
    if value > 7 { return 8; }
    if value > 8 { return 9; }
    if value > 9 { return 10; }
    if value > 10 { return 11; }
    if value > 11 { return 12; }
    fallback
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Complexity impact:"))
        .stdout(predicate::str::contains("kind=new_high_risk").not())
        .stdout(predicate::str::contains("kind=touched_existing_risky"))
        .stdout(predicate::str::contains("render"));
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
fn diff_reports_default_value_delta_as_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub const DEFAULT_MAX_HEADER_LIST_SIZE: usize = 1024 * 16;

pub fn limit() -> usize {
    DEFAULT_MAX_HEADER_LIST_SIZE
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed default header limit");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub const DEFAULT_MAX_HEADER_LIST_SIZE: usize = 16 << 20;

pub fn limit() -> usize {
    DEFAULT_MAX_HEADER_LIST_SIZE
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[high] src/lib.rs"))
        .stdout(predicate::str::contains("Semantic impact:"))
        .stdout(predicate::str::contains("category=semantic"))
        .stdout(predicate::str::contains("kind=default_value_changed"))
        .stdout(predicate::str::contains("DEFAULT_MAX_HEADER_LIST_SIZE"))
        .stdout(predicate::str::contains("1024 * 16 -> 16 << 20"));
}

#[test]
fn diff_reports_sensitive_dependency_feature_as_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub fn ok() -> bool { true }\n").expect("write lib");
    commit_all(&project, "feat: seed crate");

    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='impact-fixture'\nversion='0.1.0'\nedition='2021'\n\n[dependencies]\nrustls = { version = \"0.21\", features = [\"dangerous_configuration\"] }\n",
    )
    .expect("write cargo");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[high] Cargo.toml"))
        .stdout(predicate::str::contains("Semantic impact:"))
        .stdout(predicate::str::contains("category=semantic"))
        .stdout(predicate::str::contains(
            "kind=security_sensitive_dependency_changed",
        ))
        .stdout(predicate::str::contains(
            "rustls feature dangerous_configuration",
        ))
        .stdout(predicate::str::contains("kind=dependency_changed"));
}

#[test]
fn diff_reports_config_value_delta_as_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::create_dir_all(project.join("config")).expect("create config");
    fs::write(project.join("src/lib.rs"), "pub fn ok() -> bool { true }\n").expect("write lib");
    fs::write(
        project.join("config/default.toml"),
        "connect_timeout_ms = 1000\nmax_inflight_requests = 64\n",
    )
    .expect("write base config");
    commit_all(&project, "feat: seed default config");

    fs::write(
        project.join("config/default.toml"),
        "connect_timeout_ms = 30000\nmax_inflight_requests = 64\n",
    )
    .expect("write changed config");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[high] config/default.toml"))
        .stdout(predicate::str::contains("Semantic impact:"))
        .stdout(predicate::str::contains("kind=config_value_changed"))
        .stdout(predicate::str::contains(
            "connect_timeout_ms: 1000 -> 30000",
        ))
        .stdout(predicate::str::contains("kind=test_gap").not());
}

#[test]
fn diff_promotes_public_api_signature_change_to_top_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        "pub fn send_message(message: String) -> bool { !message.is_empty() }\n",
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed public api");

    fs::write(
        project.join("src/lib.rs"),
        "pub fn send_message(message: String, retry: bool) -> bool { retry || !message.is_empty() }\n",
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[high] src/lib.rs"))
        .stdout(predicate::str::contains("public API signature changed"))
        .stdout(predicate::str::contains("Stability impact:"))
        .stdout(predicate::str::contains(
            "kind=public_api_signature_changed",
        ));
}

#[test]
fn diff_does_not_treat_docs_only_changes_as_test_gap_or_stability_evidence() {
    let (_tmp, project, db) = base_project();
    fs::create_dir_all(project.join("docs")).expect("create docs");
    fs::write(project.join("docs/note.md"), "# base note\n").expect("write base docs");
    commit_all(&project, "docs: seed note");
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
        .stdout(predicate::str::contains("kind=unsafe_added").not())
        .stdout(predicate::str::contains("recorded=true"))
        .stdout(predicate::str::contains("evidence_saved=0"));

    assert_eq!(db_count(&db, "SELECT COUNT(*) FROM changesets"), 1);
    assert_eq!(db_count(&db, "SELECT COUNT(*) FROM file_changes"), 1);
    assert_eq!(db_count(&db, "SELECT COUNT(*) FROM evidence_items"), 0);
}

#[test]
fn diff_reports_history_evidence_for_churned_bugfixed_code() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);

    for idx in 0..3 {
        fs::write(
            project.join("src/parser.rs"),
            format!("pub fn parse() -> i32 {{ {idx} }}\n"),
        )
        .expect("write parser");
        commit_all(
            &project,
            if idx == 1 {
                "fix: prevent parser panic"
            } else {
                "refactor: parser iteration"
            },
        );
    }

    fs::write(
        project.join("src/parser.rs"),
        "pub fn parse() -> i32 { 4 }\n",
    )
    .expect("write parser change");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("History impact:"))
        .stdout(predicate::str::contains("category=history"))
        .stdout(predicate::str::contains("kind=churn_hotspot"))
        .stdout(predicate::str::contains("kind=recently_bugfixed_area"))
        .stdout(predicate::str::contains("source=git_history"))
        .stdout(predicate::str::contains("risk=high").not())
        .stdout(predicate::str::contains("recorded=true"))
        .stdout(predicate::str::contains("evidence_saved=3"));

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("recorded=true"))
        .stdout(predicate::str::contains("evidence_saved=3"));

    assert_eq!(db_count(&db, "SELECT COUNT(*) FROM changesets"), 1);
    assert_eq!(db_count(&db, "SELECT COUNT(*) FROM file_changes"), 1);
    assert_eq!(db_count(&db, "SELECT COUNT(*) FROM evidence_items"), 3);
}

#[test]
fn diff_history_does_not_count_current_pr_commit_as_prior_history() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);

    fs::write(
        project.join("src/lib.rs"),
        "pub fn message() -> &'static str { \"immediately quick\" }\n",
    )
    .expect("write lib");
    commit_all(&project, "feat: add message");
    let base = git_head(&project);

    fs::write(
        project.join("src/lib.rs"),
        "pub fn message() -> &'static str { \"immediately quit\" }\n",
    )
    .expect("write lib");
    commit_all(&project, "fix typo in message docs");

    decay_cmd(&project, &db)
        .args(["diff", &base])
        .assert()
        .success()
        .stdout(predicate::str::contains("History impact:"))
        .stdout(predicate::str::contains("kind=recently_bugfixed_area").not());
}

#[test]
fn diff_does_not_report_build_script_complexity_as_function_debt() {
    let (_tmp, project, db) = base_project();
    fs::write(project.join("build.rs"), "fn main() {}\n").expect("write build");
    commit_all(&project, "base build script");

    fs::write(
        project.join("build.rs"),
        r#"
fn main() {
    if option_env!("A").is_some() {}
    if option_env!("B").is_some() {}
    if option_env!("C").is_some() {}
    if option_env!("D").is_some() {}
    if option_env!("E").is_some() {}
    if option_env!("F").is_some() {}
    if option_env!("G").is_some() {}
    if option_env!("H").is_some() {}
    if option_env!("I").is_some() {}
    if option_env!("J").is_some() {}
    if option_env!("K").is_some() {}
    if option_env!("L").is_some() {}
}
"#,
    )
    .expect("write build");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("function=build.rs:main").not())
        .stdout(predicate::str::contains("kind=build_or_ci_changed"));
}

#[test]
fn diff_does_not_report_inline_test_complexity_as_function_debt() {
    let (_tmp, project, db) = base_project();
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn prod() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_large_fixture() {
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
    }
}
"#,
    )
    .expect("write lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test_large_fixture").not())
        .stdout(predicate::str::contains("test_gap").not());
}
