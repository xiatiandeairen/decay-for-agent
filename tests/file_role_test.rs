use decay::file_role::{classify, FileRole};

#[test]
fn classifies_primary_project_roles() {
    assert_eq!(classify("src/lib.rs"), FileRole::Prod);
    assert_eq!(classify("crates/core/src/lib.rs"), FileRole::Prod);
    assert_eq!(classify("bindings/electron/src/lib.rs"), FileRole::Prod);
    assert_eq!(classify("tokio/src/sync/mpsc/chan.rs"), FileRole::Prod);
    assert_eq!(classify("hyper/src/proto/h1/dispatch.rs"), FileRole::Prod);
    assert_eq!(classify("tests/integration.rs"), FileRole::Test);
    assert_eq!(classify("tokio/tests/sync_mpsc.rs"), FileRole::Test);
    assert_eq!(classify("src/foo_test.rs"), FileRole::Test);
    assert_eq!(classify("examples/demo.rs"), FileRole::Test);
    assert_eq!(classify("docs/roadmap.md"), FileRole::Docs);
    assert_eq!(classify("README.md"), FileRole::Docs);
    assert_eq!(classify("target/debug/out.rs"), FileRole::Generated);
    assert_eq!(classify("src/generated/schema.rs"), FileRole::Generated);
    assert_eq!(classify("Cargo.toml"), FileRole::Dependency);
    assert_eq!(classify(".github/workflows/test.yml"), FileRole::Build);
}
