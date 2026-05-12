//! End-to-end integration tests covering `decay doctor/baseline/diff`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::Command;
use predicates::prelude::*;

fn copy_fixture_to(dest: &Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample_project");
    copy_dir_recursive(&src, dest);
}

fn copy_dir_recursive(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).expect("create dest dir");
    for entry in fs::read_dir(src).expect("read source dir") {
        let entry = entry.expect("dir entry");
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let ft = entry.file_type().expect("file type");
        if ft.is_dir() {
            copy_dir_recursive(&from, &to);
        } else if ft.is_file() {
            fs::copy(&from, &to).expect("copy file");
        }
    }
}

fn decay_cmd(project: &Path, db: &Path) -> Command {
    let mut cmd = Command::cargo_bin("decay").expect("cargo bin");
    cmd.current_dir(project)
        .env("DECAY_DB_PATH", db)
        .env("RUST_LOG", "off");
    cmd
}

fn fresh_workspace() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    copy_fixture_to(&project);
    (tmp, project, db)
}

fn fresh_healthy_workspace() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    fs::create_dir_all(project.join("src")).expect("create src");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='healthy'\nversion='0.1.0'\nedition='2021'\n",
    )
    .expect("write Cargo.toml");
    fs::write(
        project.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .expect("write lib.rs");
    (tmp, project, db)
}

fn init_git_repo(project: &Path) {
    fs::create_dir_all(project.join("src")).expect("create src");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='doctor-debt-fixture'\nversion='0.1.0'\nedition='2021'\n",
    )
    .expect("write Cargo.toml");
    run(project, &["git", "init"]);
    run(
        project,
        &["git", "config", "user.email", "decay@example.test"],
    );
    run(project, &["git", "config", "user.name", "Decay Test"]);
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

fn commit_all(project: &Path, message: &str) {
    run(project, &["git", "add", "."]);
    run(project, &["git", "commit", "--no-verify", "-m", message]);
}

#[test]
fn bare_decay_prints_concise_commands_without_scanning() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .assert()
        .success()
        .stdout(predicate::str::contains("decay commands:"))
        .stdout(predicate::str::contains("decay doctor"))
        .stdout(predicate::str::contains("decay baseline <version>"))
        .stdout(predicate::str::contains("decay diff <from> [to]"))
        .stdout(predicate::str::contains("decay --help"))
        .stderr(predicate::str::is_empty());

    assert!(!db.exists(), "bare decay should not scan or create storage");
}

#[test]
fn help_prints_detailed_usage() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("--scope"))
        .stdout(predicate::str::contains("--exclude"))
        .stderr(predicate::str::is_empty());

    assert!(!db.exists(), "help should not scan or create storage");
}

#[test]
fn doctor_reports_current_risks_without_baseline() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("status=attention"))
        .stdout(predicate::str::contains("[hard-to-follow logic]"))
        .stdout(predicate::str::contains("complex_logic"))
        .stdout(predicate::str::contains("problem="))
        .stdout(predicate::str::contains("Branching complexity"));
}

#[test]
fn doctor_reports_ok_for_healthy_project() {
    let (_tmp, project, db) = fresh_healthy_workspace();

    decay_cmd(&project, &db)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("status=ok findings=0 scope=prod"))
        .stdout(predicate::str::contains("No complexity risks found"));
}

#[test]
fn doctor_honors_scope_and_exclude() {
    let (_tmp, project, db) = fresh_workspace();

    let examples_dir = project.join("examples");
    fs::create_dir_all(&examples_dir).expect("create examples dir");
    fs::write(
        examples_dir.join("noise.rs"),
        r#"
pub fn example_noise(x: i32) -> i32 {
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    if x > 4 {
                        return x;
                    }
                }
            }
        }
    }
    0
}
"#,
    )
    .expect("write examples/noise.rs");

    decay_cmd(&project, &db)
        .args(["doctor", "--scope", "all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example_noise"));

    decay_cmd(&project, &db)
        .args(["doctor", "--scope", "all", "--exclude", "examples"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example_noise").not());
}

#[test]
fn doctor_default_prod_scope_excludes_tests_examples_and_target() {
    let (_tmp, project, db) = fresh_workspace();

    fs::create_dir_all(project.join("examples")).expect("create examples");
    fs::write(project.join("examples/noise.rs"), noisy_fn("example_noise"))
        .expect("write example noise");
    fs::create_dir_all(project.join("tests")).expect("create tests");
    fs::write(
        project.join("tests/noise.rs"),
        noisy_fn("integration_noise"),
    )
    .expect("write test noise");
    fs::write(
        project.join("target/debug/junk.rs"),
        noisy_fn("target_noise"),
    )
    .expect("write target noise");

    decay_cmd(&project, &db)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("example_noise").not())
        .stdout(predicate::str::contains("integration_noise").not())
        .stdout(predicate::str::contains("target_noise").not());
}

#[test]
fn doctor_reports_hot_debt_with_history_and_boundary_tags() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_git_repo(&project);

    fs::write(project.join("src/stable.rs"), noisy_fn("stable_debt")).expect("write stable debt");
    fs::write(project.join("src/ffi.rs"), noisy_fn("ffi_hot_debt")).expect("write ffi debt");
    fs::write(
        project.join("src/lib.rs"),
        "pub mod stable;\npub mod ffi;\n",
    )
    .expect("write lib");
    commit_all(&project, "base");
    for idx in 0..3 {
        fs::write(
            project.join("src/ffi.rs"),
            format!("// churn {idx}\n{}", noisy_fn("ffi_hot_debt")),
        )
        .expect("write ffi churn");
        commit_all(&project, &format!("fix: ffi churn {idx}"));
    }

    decay_cmd(&project, &db)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hot complexity debt:"))
        .stdout(predicate::str::contains("kind=hot_complexity_debt"))
        .stdout(predicate::str::contains("file=src/ffi.rs"))
        .stdout(predicate::str::contains("tag=boundary:ffi"))
        .stdout(predicate::str::contains("tag=history:churn_hotspot"))
        .stdout(predicate::str::contains("source=doctor+git_history"));
}

#[test]
fn doctor_hot_debt_ignores_test_only_complexity_even_with_scope_all() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    init_git_repo(&project);
    fs::write(project.join("src/lib.rs"), "pub fn ok() -> i32 { 1 }\n").expect("write lib");
    fs::create_dir_all(project.join("tests")).expect("create tests");
    fs::write(project.join("tests/noise.rs"), noisy_fn("test_noise")).expect("write test noise");
    commit_all(&project, "base");

    decay_cmd(&project, &db)
        .args(["doctor", "--scope", "all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test_noise"))
        .stdout(predicate::str::contains("Hot complexity debt:"))
        .stdout(predicate::str::contains("evidence=0"));
}

#[test]
fn doctor_hot_debt_history_failure_is_diagnostic_not_failure() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hot complexity debt:"))
        .stdout(predicate::str::contains("diagnostic="));
}

#[test]
fn baseline_creates_named_version_and_does_not_dump_current_risks() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status=created baseline=v1"))
        .stdout(predicate::str::contains("functions="))
        .stdout(predicate::str::contains("complex_logic").not());
}

#[test]
fn baseline_repeated_same_version_is_unchanged() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();
    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status=unchanged baseline=v1"));
}

#[test]
fn baseline_same_version_different_content_requires_replace() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();
    make_nested_worse(&project);

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "reason=baseline_already_exists_with_different_content",
        ))
        .stdout(predicate::str::contains("hint=use_--replace"));

    decay_cmd(&project, &db)
        .args(["baseline", "v1", "--replace"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status=replaced baseline=v1"));
}

#[test]
fn diff_from_baseline_to_current_reports_regression() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();
    make_nested_worse(&project);

    decay_cmd(&project, &db)
        .args(["diff", "v1"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "status=degraded from=v1 to=current",
        ))
        .stdout(
            predicate::str::contains("[risks that got worse]").or(predicate::str::contains(
                "[functions that crossed a risk boundary]",
            )),
        )
        .stdout(predicate::str::contains("deeply_nested"))
        .stdout(predicate::str::contains("change="));
}

#[test]
fn diff_between_two_baselines_reports_regression() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();
    make_nested_worse(&project);
    decay_cmd(&project, &db)
        .args(["baseline", "v2"])
        .assert()
        .success();

    decay_cmd(&project, &db)
        .args(["diff", "v1", "v2"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("status=degraded from=v1 to=v2"))
        .stdout(predicate::str::contains("deeply_nested"));
}

#[test]
fn diff_reports_added_high_risk_function() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();
    fs::write(
        project.join("src").join("new_complex.rs"),
        noisy_fn("new_complex"),
    )
    .expect("write new complex function");

    decay_cmd(&project, &db)
        .args(["diff", "v1"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "status=degraded from=v1 to=current",
        ))
        .stdout(predicate::str::contains("[new high-risk functions]"))
        .stdout(predicate::str::contains("new_complex"));
}

#[test]
fn diff_clean_reports_single_ok_line() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();
    decay_cmd(&project, &db)
        .args(["diff", "v1"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "status=ok from=v1 to=current degradations=0",
        ))
        .stdout(predicate::str::contains("[new high-risk functions]").not());
}

#[test]
fn diff_missing_baseline_is_error() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["diff", "missing"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains(
            "status=error reason=baseline_not_found version=missing",
        ));
}

#[test]
fn parse_failure_warns_continues() {
    let (_tmp, project, db) = fresh_workspace();

    let bad = project.join("src").join("invalid.rs");
    fs::write(&bad, "pub fn broken( { not valid rust at all").expect("write invalid.rs");

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status=created baseline=v1"))
        .stdout(predicate::str::contains("partial=true"))
        .stdout(predicate::str::contains("diagnostics=1"));
}

#[test]
fn db_in_temp_dir_via_env() {
    let (_tmp, project, db) = fresh_workspace();
    assert!(!db.exists(), "db should not exist before first run");

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();

    assert!(db.exists(), "DECAY_DB_PATH should have been honored");
    let meta = fs::metadata(&db).expect("db metadata");
    assert!(meta.len() > 0, "db file should be non-empty");
}

fn make_nested_worse(project: &Path) {
    let nested_path = project.join("src").join("nested.rs");
    let deeper = r#"
pub fn deeply_nested(x: i32) -> i32 {
    let mut r = 0;
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    if x > 4 {
                        if x > 5 {
                            if x > 6 {
                                if x > 7 {
                                    r = x;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    r
}
"#;
    fs::write(&nested_path, deeper).expect("write deeper nested.rs");
}

fn noisy_fn(name: &str) -> String {
    format!(
        r#"
pub fn {name}(x: i32) -> i32 {{
    let mut r = 0;
    if x > 0 {{
        if x > 1 {{
            if x > 2 {{
                if x > 3 {{
                    if x > 4 {{
                        if x > 5 {{
                            if x > 6 {{
                                r = x;
                            }}
                        }}
                    }}
                }}
            }}
        }}
    }}
    r
}}
"#
    )
}
