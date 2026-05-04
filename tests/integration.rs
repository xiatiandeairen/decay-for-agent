//! End-to-end integration tests covering `decay init/check/diff/hotspots`
//! against the shared fixture under `tests/fixtures/sample_project`.
//!
//! Each test copies the fixture into a fresh tempdir and points
//! `DECAY_DB_PATH` at a sqlite file inside that tempdir, so tests are
//! hermetic and parallel-safe.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

/// Copy the read-only fixture project into `dest`, recursively. We can't run
/// directly inside the source fixture because (a) tests need to mutate it for
/// the `worsened` case and (b) absolute db paths must live next to the project.
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

/// Build a `Command` for the binary under test with the project root as cwd
/// and `DECAY_DB_PATH` pointing at a tempdir-scoped sqlite file.
fn decay_cmd(project: &Path, db: &Path) -> Command {
    let mut cmd = Command::cargo_bin("decay").expect("cargo bin");
    cmd.current_dir(project)
        .env("DECAY_DB_PATH", db)
        // Suppress logger output so stdout assertions stay focused on the
        // user-visible product (per server.md §4.2 stdout vs stderr split).
        .env("RUST_LOG", "off");
    cmd
}

/// Helper: bootstrap a fresh fixture + db pair in a tempdir.
fn fresh_workspace() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let db = tmp.path().join("decay.db");
    copy_fixture_to(&project);
    (tmp, project, db)
}

// 1. `decay init` on a fresh db creates snapshot #1.
#[test]
fn init_creates_snapshot() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline snapshot #1 saved."));
}

// 2. `decay init` includes baseline guidance, not a giant hot-spot dump.
#[test]
fn init_prints_baseline_guidance() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline snapshot #1 saved."))
        .stdout(predicate::str::contains("Run `decay hotspots`"))
        .stdout(predicate::str::contains("Run `decay check`"))
        .stdout(predicate::str::contains("complex_logic").not());
}

// 3. `decay hotspots` shows the intentionally-complex function.
#[test]
fn hotspots_reports_known_complex_function() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("hotspots")
        .assert()
        .success()
        .stdout(predicate::str::contains("complex_logic"))
        .stdout(predicate::str::contains("\u{26a0}"));
}

// 4. Files under target/ and .git/ never appear in scan output, even if they
//    contain functions that would otherwise breach a threshold.
#[test]
fn excluded_dirs_skipped() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("hotspots")
        .assert()
        .success()
        .stdout(predicate::str::contains("junk_complex").not())
        .stdout(predicate::str::contains("target/debug").not());
}

// 5. Custom `--exclude` accepts multiple entries and reaches the walker for
//    both basename directory skips and glob file skips.
#[test]
fn hotspots_honors_multiple_excludes() {
    let (_tmp, project, db) = fresh_workspace();

    let extra_dir = project.join("examples");
    fs::create_dir_all(&extra_dir).expect("create examples dir");
    fs::write(
        extra_dir.join("noise.rs"),
        r#"
pub fn example_noise(x: i32) -> i32 {
    let mut r = 0;
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    if x > 4 {
                        if x > 5 {
                            r = x;
                        }
                    }
                }
            }
        }
    }
    r
}
"#,
    )
    .expect("write examples/noise.rs");

    decay_cmd(&project, &db)
        .arg("hotspots")
        .assert()
        .success()
        .stdout(predicate::str::contains("example_noise"))
        .stdout(predicate::str::contains("complex_logic"))
        .stdout(predicate::str::contains("deeply_nested"));

    decay_cmd(&project, &db)
        .args([
            "hotspots",
            "--exclude",
            "examples",
            "--exclude",
            "src/comp*.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("example_noise").not())
        .stdout(predicate::str::contains("complex_logic").not())
        .stdout(predicate::str::contains("deeply_nested"));
}

// 6. Root `.gitignore` is respected without needing explicit `--exclude`.
#[test]
fn hotspots_respects_root_gitignore() {
    let (_tmp, project, db) = fresh_workspace();

    fs::write(project.join(".gitignore"), "examples/\nsrc/complex.rs\n")
        .expect("write .gitignore");
    let extra_dir = project.join("examples");
    fs::create_dir_all(&extra_dir).expect("create examples dir");
    fs::write(
        extra_dir.join("noise.rs"),
        "pub fn example_noise() { if true { if true { if true { if true { if true {} }}}}}\n",
    )
    .expect("write examples/noise.rs");

    decay_cmd(&project, &db)
        .arg("hotspots")
        .assert()
        .success()
        .stdout(predicate::str::contains("example_noise").not())
        .stdout(predicate::str::contains("complex_logic").not())
        .stdout(predicate::str::contains("deeply_nested"));
}

// 7. `decay check` without a baseline gives a friendly init hint.
#[test]
fn check_without_baseline_prompts_init() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db)
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("No baseline snapshot"))
        .stdout(predicate::str::contains("decay init"));
}

// 8. Two consecutive baseline snapshots with no source change → diff reports clean.
#[test]
fn diff_no_change_reports_clean() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db).arg("init").assert().success();
    decay_cmd(&project, &db).arg("init").assert().success();

    decay_cmd(&project, &db)
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::str::contains("No functions degraded"));
}

// 9. Mutating nested.rs to deepen its nesting after baseline → `decay check`
//    flags the function as worsened (or as a fresh threshold crossing,
//    depending on whether it was already over).
#[test]
fn check_added_nesting_reports_worsened() {
    let (_tmp, project, db) = fresh_workspace();

    decay_cmd(&project, &db).arg("init").assert().success();

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

    decay_cmd(&project, &db)
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("[worsened]").or(predicate::str::contains("crossed")))
        .stdout(predicate::str::contains("deeply_nested"));
}

// 10. An invalid .rs file produces a parse warning but does not abort the
//    scan; the rest of the project is still processed and a baseline is saved.
#[test]
fn parse_failure_warns_continues() {
    let (_tmp, project, db) = fresh_workspace();

    let bad = project.join("src").join("invalid.rs");
    fs::write(&bad, "pub fn broken( { not valid rust at all").expect("write invalid.rs");

    decay_cmd(&project, &db)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline snapshot #1 saved."));
}

// 11. DECAY_DB_PATH actually directs db writes to the requested path.
#[test]
fn db_in_temp_dir_via_env() {
    let (_tmp, project, db) = fresh_workspace();
    assert!(!db.exists(), "db should not exist before first run");

    decay_cmd(&project, &db).arg("init").assert().success();

    assert!(db.exists(), "DECAY_DB_PATH should have been honored");
    let meta = fs::metadata(&db).expect("db metadata");
    assert!(meta.len() > 0, "db file should be non-empty");
}
