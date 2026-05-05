//! End-to-end integration tests covering `decay doctor/baseline/diff`.

use std::fs;
use std::path::{Path, PathBuf};

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
fn doctor_verbose_explains_problem_groups() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["doctor", "--verbose"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: doctor"))
        .stdout(predicate::str::contains("What this means:"))
        .stdout(predicate::str::contains("Why it matters:"))
        .stdout(predicate::str::contains("Bad points:"));
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
fn diff_verbose_explains_change_groups() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .args(["baseline", "v1"])
        .assert()
        .success();
    make_nested_worse(&project);

    decay_cmd(&project, &db)
        .args(["diff", "v1", "--verbose"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Mode: diff"))
        .stdout(predicate::str::contains("What changed:"))
        .stdout(predicate::str::contains("Why it matters:"))
        .stdout(predicate::str::contains("Bad points:"));
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
        .stdout(predicate::str::contains("status=created baseline=v1"));
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
