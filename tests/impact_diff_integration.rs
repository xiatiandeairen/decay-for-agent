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
fn diff_reports_prod_change_with_tests_as_diff_structure_evidence() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub fn answer() -> i32 { 1 }\n").expect("write lib");
    fs::create_dir_all(project.join("tests")).expect("create tests");
    fs::write(
        project.join("tests/answer_test.rs"),
        "#[test]\nfn answer_stays_visible() {}\n",
    )
    .expect("write test");
    commit_all(&project, "feat: seed answer");

    fs::write(project.join("src/lib.rs"), "pub fn answer() -> i32 { 2 }\n").expect("change lib");
    fs::write(
        project.join("tests/answer_test.rs"),
        "#[test]\nfn answer_stays_visible() {}\n#[test]\nfn answer_changes() {}\n",
    )
    .expect("change test");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[medium] src/lib.rs"))
        .stdout(predicate::str::contains(
            "production code changed with test changes",
        ))
        .stdout(predicate::str::contains("Quality impact:"))
        .stdout(predicate::str::contains("kind=prod_change_with_tests"))
        .stdout(predicate::str::contains("confidence=low value=supporting"))
        .stdout(predicate::str::contains("kind=test_gap").not())
        .stdout(predicate::str::contains("Semantic findings:").not());
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
fn diff_reports_env_default_delta_as_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn retry_limit() -> String {
    std::env::var("MX_RETRY_LIMIT").unwrap_or_else(|_| "3".to_string())
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed env default");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn retry_limit() -> String {
    std::env::var("MX_RETRY_LIMIT").unwrap_or_else(|_| "30".to_string())
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
        .stdout(predicate::str::contains("kind=env_default_changed"))
        .stdout(predicate::str::contains("MX_RETRY_LIMIT: 3 -> 30"));
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
fn diff_reports_host_header_authority_change_as_protocol_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn host_header(url: &hyper::Uri) -> String {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let authority = url.authority().unwrap().clone();
    Request::builder()
        .header(hyper::header::HOST, authority.as_str())
        .body(())
        .unwrap();
    format!("{}:{}", host, port)
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed host header");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn host_header(url: &hyper::Uri) -> String {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    Request::builder()
        .header(hyper::header::HOST, format!("{}:{}", host, port))
        .body(())
        .unwrap();
    format!("{}:{}", host, port)
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
        .stdout(predicate::str::contains(
            "protocol host header authority changed",
        ))
        .stdout(predicate::str::contains("Semantic impact:"))
        .stdout(predicate::str::contains(
            "kind=protocol_header_authority_changed",
        ));
}

#[test]
fn diff_reports_zero_closed_condition_as_boundary_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn forget_permits(state: usize, permits: usize) -> usize {
    state - permits
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed semaphore boundary");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn forget_permits(state: usize, permits: usize) -> usize {
    if permits == 0 || state & CLOSED_BIT != 0 {
        return state;
    }
    state - permits
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
        .stdout(predicate::str::contains("kind=boundary_condition_changed"))
        .stdout(predicate::str::contains("permits == 0"))
        .stdout(predicate::str::contains("CLOSED_BIT"));
}

#[test]
fn diff_reports_usize_max_condition_as_integer_boundary_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        "pub fn read_len(len: u64) -> usize { len as usize }\n",
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed integer boundary");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn read_len(len: u64) -> usize {
    if len > usize::MAX as u64 {
        return usize::MAX;
    }
    len as usize
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Semantic impact:"))
        .stdout(predicate::str::contains("kind=boundary_condition_changed"))
        .stdout(predicate::str::contains("usize::MAX"));
}

#[test]
fn diff_reports_bitmask_assignment_as_boundary_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
const CLOSED_BIT: usize = 1;

pub fn forget_permits(state: usize, permits: usize) -> usize {
    state - permits
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed bit boundary");

    fs::write(
        project.join("src/lib.rs"),
        r#"
const CLOSED_BIT: usize = 1;

pub fn forget_permits(state: usize, permits: usize) -> usize {
    (state - permits) | (state & CLOSED_BIT)
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Semantic impact:"))
        .stdout(predicate::str::contains("kind=boundary_expression_changed"))
        .stdout(predicate::str::contains("CLOSED_BIT"));
}

#[test]
fn diff_reports_saturating_assignment_as_boundary_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        "pub fn read_len(len: u64) -> usize { len as usize }\n",
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed saturation boundary");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn read_len(len: u64) -> usize {
    len.min(usize::MAX as u64) as usize
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Semantic impact:"))
        .stdout(predicate::str::contains("kind=boundary_expression_changed"))
        .stdout(predicate::str::contains("usize::MAX"));
}

#[test]
fn diff_reports_usize_try_from_fallback_as_integer_platform_boundary_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn to_read(remaining: u64) -> usize {
    remaining as usize
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed truncating cast");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn to_read(remaining: u64) -> usize {
    usize::try_from(remaining).unwrap_or(usize::MAX)
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
        .stdout(predicate::str::contains(
            "kind=integer_platform_boundary_changed",
        ))
        .stdout(predicate::str::contains("usize::try_from"))
        .stdout(predicate::str::contains("usize::MAX"));
}

#[test]
fn diff_does_not_report_plain_usize_try_from_as_platform_boundary_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn parse_len(value: u32) -> Result<usize, std::num::TryFromIntError> {
    Ok(value as usize)
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed checked cast");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn parse_len(value: u32) -> Result<usize, std::num::TryFromIntError> {
    usize::try_from(value)
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=integer_platform_boundary_changed").not());
}

#[test]
fn diff_reports_panic_to_error_propagation_for_malformed_input_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn raw_value(raw: &[u8]) -> Result<&str, &'static str> {
    let raw = std::str::from_utf8(raw).unwrap();
    Ok(raw)
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed utf8 panic");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn raw_value(raw: &[u8]) -> Result<&str, &'static str> {
    let raw = match std::str::from_utf8(raw) {
        Ok(raw) => raw,
        Err(_) => return Err("invalid unicode code point"),
    };
    Ok(raw)
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
        .stdout(predicate::str::contains(
            "kind=panic_to_error_propagation_changed",
        ))
        .stdout(predicate::str::contains("Err(_) => return Err"))
        .stdout(predicate::str::contains("confidence=high value=direct"))
        .stdout(predicate::str::contains("Semantic findings:").not());
}

#[test]
fn diff_reports_error_propagation_change_for_error_to_non_error_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub enum Reason { NoError, Cancel, Other }

pub fn poll_body(reason: Reason) -> Result<Option<&'static str>, &'static str> {
    match reason {
        Reason::NoError => return Err("body error"),
        Reason::Cancel => return Err("body error"),
        Reason::Other => Err("body error"),
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed error propagation");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub enum Reason { NoError, Cancel, Other }

pub fn poll_body(reason: Reason) -> Result<Option<&'static str>, &'static str> {
    match reason {
        Reason::NoError => return Ok(None),
        Reason::Cancel => return Ok(None),
        Reason::Other => Err("body error"),
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=error_propagation_changed"))
        .stdout(predicate::str::contains("Reason::"))
        .stdout(predicate::str::contains("confidence=medium value=direct"));
}

#[test]
fn diff_reports_error_propagation_change_for_panic_to_internal_parse_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub enum Parse { Internal }

pub fn header_name(bytes: &[u8]) -> Result<&[u8], Parse> {
    match std::str::from_utf8(bytes) {
        Ok(_) => Ok(bytes),
        Err(_) => panic!("illegal header name"),
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed panic header");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub enum Parse { Internal }

pub fn header_name(bytes: &[u8]) -> Result<&[u8], Parse> {
    match std::str::from_utf8(bytes) {
        Ok(_) => Ok(bytes),
        Err(e) => {
            let _ = e;
            return Err(Parse::Internal);
        }
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kind=panic_to_error_propagation_changed",
        ))
        .stdout(predicate::str::contains("kind=error_propagation_changed"));
}

#[test]
fn diff_reports_readiness_error_propagation_change_for_recv_interest_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Interest;

impl Interest {
    pub const READABLE: u8 = 1;
    pub const ERROR: u8 = 2;
}

pub struct Registration;

impl Registration {
    pub async fn async_io<F, T>(&self, _interest: u8, f: F) -> Result<T, &'static str>
    where
        F: FnOnce() -> Result<T, &'static str>,
    {
        f()
    }
}

pub struct Io;

impl Io {
    pub fn registration(&self) -> Registration {
        Registration
    }

    pub fn recv(&self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Ok(1)
    }
}

pub struct Socket {
    io: Io,
}

impl Socket {
    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize, &'static str> {
        self.io
            .registration()
            .async_io(Interest::READABLE, || self.io.recv(buf))
            .await
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed recv readiness");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Interest;

impl Interest {
    pub const READABLE: u8 = 1;
    pub const ERROR: u8 = 2;
}

pub struct Registration;

impl Registration {
    pub async fn async_io<F, T>(&self, _interest: u8, f: F) -> Result<T, &'static str>
    where
        F: FnOnce() -> Result<T, &'static str>,
    {
        f()
    }
}

pub struct Io;

impl Io {
    pub fn registration(&self) -> Registration {
        Registration
    }

    pub fn recv(&self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Ok(1)
    }
}

pub struct Socket {
    io: Io,
}

impl Socket {
    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize, &'static str> {
        self.io
            .registration()
            .async_io(Interest::READABLE | Interest::ERROR, || self.io.recv(buf))
            .await
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kind=readiness_error_propagation_changed",
        ))
        .stdout(predicate::str::contains(
            "Interest::READABLE | Interest::ERROR",
        ))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_reports_drop_error_propagation_change_for_incomplete_message_guard() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Sender;

impl Sender {
    pub fn send_error(&mut self, _err: Error) {}
}

pub struct Error;

impl Error {
    pub fn new_incomplete() -> Self {
        Error
    }
}

pub struct Dispatcher {
    body_tx: Option<Sender>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self { body_tx: None }
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed dispatcher sender");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Sender;

impl Sender {
    pub fn send_error(&mut self, _err: Error) {}
}

pub struct Error;

impl Error {
    pub fn new_incomplete() -> Self {
        Error
    }
}

pub struct SenderDropGuard(Option<Sender>);

impl SenderDropGuard {
    pub fn none() -> Self {
        Self(None)
    }
}

impl Drop for SenderDropGuard {
    fn drop(&mut self) {
        if let Some(mut sender) = self.0.take() {
            sender.send_error(Error::new_incomplete());
        }
    }
}

pub struct Dispatcher {
    body_tx: SenderDropGuard,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            body_tx: SenderDropGuard::none(),
        }
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kind=drop_error_propagation_changed",
        ))
        .stdout(predicate::str::contains(
            "send_error(Error::new_incomplete())",
        ))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_does_not_report_unrelated_unwrap_refactor_as_error_propagation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn parse(raw: &str) -> usize {
    raw.parse::<usize>().unwrap()
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed unwrap");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn parse(raw: &str) -> usize {
    raw.parse::<usize>().expect("valid fixture")
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=panic_to_error_propagation_changed").not());
}

#[test]
fn diff_does_not_report_send_interest_error_as_readiness_error_propagation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Interest;

impl Interest {
    pub const WRITABLE: u8 = 1;
    pub const ERROR: u8 = 2;
}

pub struct Registration;

impl Registration {
    pub async fn async_io<F, T>(&self, _interest: u8, f: F) -> Result<T, &'static str>
    where
        F: FnOnce() -> Result<T, &'static str>,
    {
        f()
    }
}

pub struct Io;

impl Io {
    pub fn registration(&self) -> Registration {
        Registration
    }

    pub fn send(&self, _buf: &[u8]) -> Result<usize, &'static str> {
        Ok(1)
    }
}

pub struct Socket {
    io: Io,
}

impl Socket {
    pub async fn send(&self, buf: &[u8]) -> Result<usize, &'static str> {
        self.io
            .registration()
            .async_io(Interest::WRITABLE, || self.io.send(buf))
            .await
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed send readiness");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Interest;

impl Interest {
    pub const WRITABLE: u8 = 1;
    pub const ERROR: u8 = 2;
}

pub struct Registration;

impl Registration {
    pub async fn async_io<F, T>(&self, _interest: u8, f: F) -> Result<T, &'static str>
    where
        F: FnOnce() -> Result<T, &'static str>,
    {
        f()
    }
}

pub struct Io;

impl Io {
    pub fn registration(&self) -> Registration {
        Registration
    }

    pub fn send(&self, _buf: &[u8]) -> Result<usize, &'static str> {
        Ok(1)
    }
}

pub struct Socket {
    io: Io,
}

impl Socket {
    pub async fn send(&self, buf: &[u8]) -> Result<usize, &'static str> {
        self.io
            .registration()
            .async_io(Interest::WRITABLE | Interest::ERROR, || self.io.send(buf))
            .await
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=readiness_error_propagation_changed").not());
}

#[test]
fn diff_does_not_report_test_only_error_propagation_as_semantic_direct() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::create_dir_all(project.join("tests")).expect("create tests");
    fs::write(project.join("src/lib.rs"), "pub fn stable() {}\n").expect("write lib");
    fs::write(
        project.join("tests/error_test.rs"),
        r#"
#[test]
fn old_test() {
    let result: Result<(), &str> = Err("body error");
    assert!(result.is_err());
}
"#,
    )
    .expect("write base test");
    commit_all(&project, "feat: seed test");

    fs::write(
        project.join("tests/error_test.rs"),
        r#"
#[test]
fn old_test() {
    let result: Result<Option<()>, &str> = Ok(None);
    assert!(result.is_ok());
}
"#,
    )
    .expect("write changed test");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=error_propagation_changed").not())
        .stdout(predicate::str::contains("kind=panic_to_error_propagation_changed").not());
}

#[test]
fn diff_does_not_report_plain_arithmetic_assignment_as_boundary_expression() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        "pub fn total(a: usize, b: usize) -> usize { a + b }\n",
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed arithmetic");

    fs::write(
        project.join("src/lib.rs"),
        "pub fn total(a: usize, b: usize) -> usize { (a + b) * 2 }\n",
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=boundary_expression_changed").not());
}

#[test]
fn diff_does_not_report_plain_status_condition_as_boundary_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub fn ok() -> bool { true }\n").expect("write lib");
    commit_all(&project, "feat: seed status condition");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn ok() -> bool {
    let status = 200;
    if status == 201 {
        return false;
    }
    true
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=boundary_condition_changed").not());
}

#[test]
fn diff_does_not_report_test_file_boundary_condition_as_semantic_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub fn ok() -> bool { true }\n").expect("write lib");
    fs::create_dir_all(project.join("tests")).expect("create tests");
    fs::write(
        project.join("tests/state_test.rs"),
        "#[test]\nfn base() {}\n",
    )
    .expect("write test");
    commit_all(&project, "feat: seed test boundary");

    fs::write(
        project.join("tests/state_test.rs"),
        r#"
#[test]
fn closed_state() {
    let closed = true;
    if closed != false {
        assert!(true);
    }
}
"#,
    )
    .expect("write changed test");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=boundary_condition_changed").not());
}

#[test]
fn diff_does_not_flag_public_api_moved_into_new_file_as_added() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub struct Incoming;\n").expect("write base lib");
    commit_all(&project, "feat: seed moved public api");

    fs::create_dir_all(project.join("src/body/incoming")).expect("create nested module");
    fs::write(project.join("src/lib.rs"), "pub mod body;\n").expect("write changed lib");
    fs::write(project.join("src/body.rs"), "pub mod incoming;\n").expect("write body");
    fs::write(
        project.join("src/body/incoming.rs"),
        "pub struct Incoming;\n",
    )
    .expect("write incoming");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=public_api_added").not());
}

#[test]
fn diff_reports_lifecycle_ordering_change_for_upgradeable_graceful_shutdown_guard() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
use std::pin::Pin;

pub struct Inner;

impl Inner {
    pub fn graceful_shutdown(self: Pin<&mut Self>) {}
}

pub struct UpgradeableConnection {
    inner: Option<Inner>,
}

impl UpgradeableConnection {
    pub fn graceful_shutdown(mut self: Pin<&mut Self>) {
        Pin::new(self.inner.as_mut().unwrap()).graceful_shutdown()
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed upgradeable shutdown");

    fs::write(
        project.join("src/lib.rs"),
        r#"
use std::pin::Pin;

pub struct Inner;

impl Inner {
    pub fn graceful_shutdown(self: Pin<&mut Self>) {}
}

pub struct UpgradeableConnection {
    inner: Option<Inner>,
}

impl UpgradeableConnection {
    pub fn graceful_shutdown(mut self: Pin<&mut Self>) {
        if let Some(conn) = self.inner.as_mut() {
            Pin::new(conn).graceful_shutdown()
        }
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=lifecycle_ordering_changed"))
        .stdout(predicate::str::contains(
            "Pin::new(conn).graceful_shutdown()",
        ))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_reports_lifecycle_ordering_change_for_initial_state_shutdown_branch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Conn;

impl Conn {
    pub fn disable_keep_alive(&mut self) {}

    pub fn is_write_closed(&self) -> bool {
        false
    }
}

pub struct Dispatcher {
    conn: Conn,
}

impl Dispatcher {
    pub fn close(&mut self) {}

    pub fn disable_keep_alive(&mut self) {
        self.conn.disable_keep_alive();
        if self.conn.is_write_closed() {
            self.close();
        }
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed shutdown branch");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Conn;

impl Conn {
    pub fn disable_keep_alive(&mut self) {}

    pub fn is_write_closed(&self) -> bool {
        false
    }

    pub fn has_initial_read_write_state(&self) -> bool {
        true
    }
}

pub struct Dispatcher {
    conn: Conn,
}

impl Dispatcher {
    pub fn close(&mut self) {}

    pub fn disable_keep_alive(&mut self) {
        self.conn.disable_keep_alive();
        if self.conn.is_write_closed() || self.conn.has_initial_read_write_state() {
            self.close();
        }
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=lifecycle_ordering_changed"))
        .stdout(predicate::str::contains("has_initial_read_write_state()"))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_reports_lifecycle_ordering_change_for_cancel_reset_cleanup_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
use std::task::Poll;

pub mod h2 {
    pub enum Reason {
        CANCEL,
    }
}

pub struct Pipe;

impl Pipe {
    pub fn send_reset(&mut self, _reason: h2::Reason) {}
}

pub struct PipeMap {
    pipe: Pipe,
    conn_drop_ref: Option<()>,
    cancel_rx: Option<()>,
}

impl PipeMap {
    pub fn poll(&mut self) -> Poll<()> {
        Poll::Pending
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed cancel cleanup");

    fs::write(
        project.join("src/lib.rs"),
        r#"
use std::task::Poll;

pub mod h2 {
    pub enum Reason {
        CANCEL,
    }
}

pub struct Pipe;

impl Pipe {
    pub fn send_reset(&mut self, _reason: h2::Reason) {}
}

pub struct PipeMap {
    pipe: Pipe,
    conn_drop_ref: Option<()>,
    cancel_rx: Option<()>,
}

impl PipeMap {
    pub fn poll(&mut self) -> Poll<()> {
        let cancel_result = self.cancel_rx.as_mut().map(|_| Poll::Ready(()));
        match cancel_result {
            Some(Poll::Ready(())) => {
                self.pipe.send_reset(h2::Reason::CANCEL);
                drop(self.conn_drop_ref.take().expect("Future polled twice"));
                return Poll::Ready(());
            }
            Some(Poll::Pending) | None => {}
        }
        Poll::Pending
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=lifecycle_ordering_changed"))
        .stdout(predicate::str::contains("send_reset(h2::Reason::CANCEL)"))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_does_not_report_cancel_protocol_mapping_as_lifecycle_ordering() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
use std::task::Poll;

pub enum Reason {
    NO_ERROR,
    CANCEL,
}

pub fn map(reason: Reason) -> Poll<Option<Result<(), &'static str>>> {
    match reason {
        Reason::NO_ERROR => Poll::Ready(Some(Err("body"))),
        Reason::CANCEL => Poll::Ready(Some(Err("body"))),
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed cancel mapping");

    fs::write(
        project.join("src/lib.rs"),
        r#"
use std::task::Poll;

pub enum Reason {
    NO_ERROR,
    CANCEL,
}

pub fn map(reason: Reason) -> Poll<Option<Result<(), &'static str>>> {
    match reason {
        Reason::NO_ERROR | Reason::CANCEL => Poll::Ready(None),
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=lifecycle_ordering_changed").not());
}

#[test]
fn diff_reports_concurrency_ordering_change_for_blocking_pool_idle_race() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Metrics;

impl Metrics {
    pub fn inc_num_idle_threads(&self) {}
    pub fn dec_num_idle_threads(&self) -> usize { 1 }
}

pub struct Inner {
    metrics: Metrics,
}

impl Inner {
    pub fn run(&self, shutdown: bool, notified: bool) {
        self.metrics.inc_num_idle_threads();
        if notified {
            return;
        }
        if shutdown {
            let prev_idle = self.metrics.dec_num_idle_threads();
            assert_ne!(prev_idle, 0);
        }
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed blocking pool idle path");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Metrics;

impl Metrics {
    pub fn inc_num_idle_threads(&self) {}
    pub fn dec_num_idle_threads(&self) -> usize { 1 }
}

pub struct Inner {
    metrics: Metrics,
}

impl Inner {
    pub fn run(&self, shutdown: bool, notified: bool) {
        let mut is_counted_idle;
        self.metrics.inc_num_idle_threads();
        is_counted_idle = true;
        if notified {
            is_counted_idle = false;
            return;
        }
        if shutdown && is_counted_idle {
            let prev_idle = self.metrics.dec_num_idle_threads();
            assert_ne!(prev_idle, 0);
        }
    }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kind=concurrency_ordering_changed",
        ))
        .stdout(predicate::str::contains("is_counted_idle"))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_reports_concurrency_ordering_change_for_wakeup_repoll_stall() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Dispatcher;

impl Dispatcher {
    pub fn poll_loop(&mut self) -> bool {
        let _ = self.poll_write();
        let _ = self.poll_flush();
        if !self.wants_read_again() {
            return true;
        }
        false
    }

    fn poll_write(&mut self) -> bool { true }
    fn poll_flush(&mut self) -> bool { true }
    fn wants_read_again(&self) -> bool { false }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed wakeup stall path");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub struct Dispatcher;

impl Dispatcher {
    pub fn poll_loop(&mut self) -> bool {
        let _ = self.poll_write();
        let conn_ready = self.poll_flush();
        let wants_write_again = self.can_write_again() && conn_ready;
        let wants_read_again = self.wants_read_again();
        if !(wants_write_again || wants_read_again) {
            return true;
        }
        false
    }

    fn poll_write(&mut self) -> bool { true }
    fn poll_flush(&mut self) -> bool { true }
    fn wants_read_again(&self) -> bool { false }
    fn can_write_again(&mut self) -> bool { true }
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kind=concurrency_ordering_changed",
        ))
        .stdout(predicate::str::contains("wants_write_again"))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_reports_concurrency_ordering_change_for_completed_before_repoll_cleanup() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        r#"
pub enum Lifecycle {
    Waiting,
    Cancelled,
    Completed,
}

pub fn cancel_op(lifecycle: &mut Lifecycle) {
    match lifecycle {
        Lifecycle::Completed => {}
        Lifecycle::Cancelled => {}
        Lifecycle::Waiting => {}
    }
}
"#,
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed completion ordering");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub enum Lifecycle {
    Waiting,
    Cancelled(CancelData),
    Completed(Cqe),
}

pub struct Cqe;
pub struct OwnedFd;
pub struct Open;

pub enum CancelData {
    Open(Open),
}

impl OwnedFd {
    pub unsafe fn from_raw_fd(_fd: i32) -> Self { Self }
}

pub fn cancel_op(lifecycle: &mut Lifecycle, cqe: Cqe) {
    match lifecycle {
        Lifecycle::Completed(cqe) => {
            if let Lifecycle::Cancelled(CancelData::Open(_)) = lifecycle {
                unsafe { OwnedFd::from_raw_fd(1) };
            }
        }
        Lifecycle::Cancelled(_) => {}
        Lifecycle::Waiting => {}
    }
    let _ = cqe;
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kind=concurrency_ordering_changed",
        ))
        .stdout(predicate::str::contains("Lifecycle::Completed(cqe)"))
        .stdout(predicate::str::contains("confidence=high value=direct"));
}

#[test]
fn diff_does_not_report_assert_boundary_as_concurrency_ordering() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        "pub fn make(limit: usize) -> usize { limit }\n",
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed max readers");

    fs::write(
        project.join("src/lib.rs"),
        r#"
pub fn make(limit: usize) -> usize {
    assert!(limit != 0, "limit must not be zero");
    limit
}
"#,
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind=concurrency_ordering_changed").not());
}

#[test]
fn diff_promotes_public_api_addition_to_top_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub struct Existing;\n").expect("write base lib");
    commit_all(&project, "feat: seed public api");

    fs::write(
        project.join("src/lib.rs"),
        "pub struct Existing;\npub struct HeaderCaseMap;\n",
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[high] src/lib.rs"))
        .stdout(predicate::str::contains("public API added"))
        .stdout(predicate::str::contains("Stability impact:"))
        .stdout(predicate::str::contains("kind=public_api_added"));
}

#[test]
fn diff_promotes_public_api_function_addition_to_top_risk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        "pub fn keep_alive(val: bool) -> bool { val }\n",
    )
    .expect("write base lib");
    commit_all(&project, "feat: seed public api");

    fs::write(
        project.join("src/lib.rs"),
        "pub fn keep_alive(val: bool) -> bool { val }\n\
         pub fn http10_disable_keep_alive(disable: bool) -> bool { disable }\n",
    )
    .expect("write changed lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("[high] src/lib.rs"))
        .stdout(predicate::str::contains("public API function added"))
        .stdout(predicate::str::contains("Stability impact:"))
        .stdout(predicate::str::contains("kind=public_api_function_added"))
        .stdout(predicate::str::contains("http10_disable_keep_alive"));
}

#[test]
fn diff_keeps_mechanical_clippy_changes_quiet() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(
        project.join("src/lib.rs"),
        "pub mod alpha;\npub mod beta;\n",
    )
    .expect("write lib");
    fs::write(
        project.join("src/alpha.rs"),
        "pub fn alpha(values: &[String]) -> Vec<String> { values.iter().map(|s| s.clone()).collect() }\n",
    )
    .expect("write alpha");
    fs::write(
        project.join("src/beta.rs"),
        "pub fn beta(values: &[String]) -> Vec<String> { values.iter().map(|&ch| ch).collect() }\n",
    )
    .expect("write beta");
    commit_all(&project, "feat: seed clippy fixture");

    fs::write(
        project.join("src/alpha.rs"),
        "pub fn alpha(values: &[String]) -> Vec<String> { values.iter().cloned().collect() }\n",
    )
    .expect("write alpha changed");
    fs::write(
        project.join("src/beta.rs"),
        "pub fn beta(values: &[String]) -> Vec<String> { values.iter().copied().collect() }\n",
    )
    .expect("write beta changed");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top risk findings:"))
        .stdout(predicate::str::contains("none"))
        .stdout(predicate::str::contains("kind=blast_radius").not())
        .stdout(predicate::str::contains("kind=mixed_concerns").not())
        .stdout(predicate::str::contains("kind=test_gap").not());
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
fn diff_reports_parse_coverage_gap_as_structured_evidence() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub fn answer() -> i32 { 1 }\n").expect("write lib");
    commit_all(&project, "feat: seed parse fixture");

    fs::write(
        project.join("src/lib.rs"),
        "pub fn answer() -> i32 { let = 2; 2 }\n",
    )
    .expect("write invalid lib");

    decay_cmd(&project, &db)
        .args(["diff", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Coverage impact:"))
        .stdout(predicate::str::contains("kind=parse_coverage_gap"))
        .stdout(predicate::str::contains("source=scan"))
        .stdout(predicate::str::contains(
            "diagnostic=parse error in src/lib.rs",
        ))
        .stdout(predicate::str::contains("[medium] src/lib.rs"))
        .stdout(predicate::str::contains("decay scan coverage is partial"));
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
