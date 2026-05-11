use decay::evidence::quality::{detect_quality, QualityFileChange};

fn change(path: &str) -> QualityFileChange {
    QualityFileChange::new(path)
}

fn kinds(paths: &[&str]) -> Vec<String> {
    detect_quality(
        &paths
            .iter()
            .map(|path| change(path))
            .collect::<Vec<QualityFileChange>>(),
    )
    .records
    .into_iter()
    .map(|record| record.kind)
    .collect()
}

fn assert_has_kind(paths: &[&str], expected: &str) {
    let actual = kinds(paths);
    assert!(
        actual.iter().any(|kind| kind == expected),
        "expected kind {expected:?}, got {actual:?}"
    );
}

fn assert_lacks_kind(paths: &[&str], unexpected: &str) {
    let actual = kinds(paths);
    assert!(
        actual.iter().all(|kind| kind != unexpected),
        "unexpected kind {unexpected:?}, got {actual:?}"
    );
}

#[test]
fn docs_only_change_does_not_trigger_test_gap() {
    assert_lacks_kind(&["docs/roadmap.md", "README.md"], "test_gap");
}

#[test]
fn production_change_without_tests_triggers_test_gap() {
    assert_has_kind(&["src/diff.rs"], "test_gap");
}

#[test]
fn production_and_test_change_does_not_trigger_general_test_gap() {
    assert_lacks_kind(&["src/diff.rs", "tests/diff_test.rs"], "test_gap");
}

#[test]
fn sensitive_production_change_without_tests_triggers_sensitive_test_gap() {
    assert_has_kind(&["src/auth/token.rs"], "sensitive_test_gap");
}

#[test]
fn multiple_crates_and_domains_trigger_blast_radius_and_mixed_concerns() {
    let paths = [
        "crates/cli/src/main.rs",
        "crates/store/src/lib.rs",
        "src/parser.rs",
    ];

    assert_has_kind(&paths, "blast_radius");
    assert_has_kind(&paths, "mixed_concerns");
}

#[test]
fn cargo_toml_dependency_change_is_reported() {
    assert_has_kind(&["Cargo.toml"], "dependency_changed");
}

#[test]
fn build_script_or_ci_change_is_reported() {
    assert_has_kind(&["build.rs"], "build_or_ci_changed");
    assert_has_kind(&[".github/workflows/test.yml"], "build_or_ci_changed");
}

#[test]
fn target_and_generated_paths_do_not_count_toward_main_blast_radius() {
    assert_lacks_kind(
        &[
            "src/diff.rs",
            "target/debug/build/out.rs",
            "src/generated/schema.rs",
        ],
        "blast_radius",
    );
}
