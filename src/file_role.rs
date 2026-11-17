#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    Prod,
    Test,
    Docs,
    Generated,
    Build,
    Dependency,
    Unknown,
}

impl FileRole {
    pub fn is_prod(self) -> bool {
        matches!(self, Self::Prod)
    }

    pub fn is_test(self) -> bool {
        matches!(self, Self::Test)
    }

    pub fn is_docs(self) -> bool {
        matches!(self, Self::Docs)
    }
}

pub fn classify(path: &str) -> FileRole {
    let path = normalize(path);
    if path.is_empty() {
        return FileRole::Unknown;
    }

    let file_name = path.rsplit('/').next().unwrap_or(path);

    if is_generated_path(path) {
        return FileRole::Generated;
    }
    if is_docs_path(path, file_name) {
        return FileRole::Docs;
    }
    if is_test_path(path, file_name) {
        return FileRole::Test;
    }
    if is_dependency_path(path) {
        return FileRole::Dependency;
    }
    if is_build_or_ci_path(path) {
        return FileRole::Build;
    }
    if path.starts_with("src/")
        || path.starts_with("crates/")
        || path.starts_with("bindings/")
        || path.contains("/src/")
    {
        return FileRole::Prod;
    }

    FileRole::Unknown
}

fn normalize(path: &str) -> &str {
    path.trim().trim_start_matches("./")
}

fn is_generated_path(path: &str) -> bool {
    path.split('/').any(|part| {
        part == "target"
            || part == "generated"
            || part == ".git"
            || part == "node_modules"
            || part == "dist"
    })
}

fn is_docs_path(path: &str, file_name: &str) -> bool {
    path.starts_with("docs/")
        || file_name == "README.md"
        || file_name == "README.zh.md"
        || file_name.ends_with(".md")
        || file_name.ends_with(".adoc")
        || file_name.ends_with(".rst")
}

fn is_test_path(path: &str, file_name: &str) -> bool {
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.starts_with("fixtures/")
        || path.contains("/fixtures/")
        || path.starts_with("examples/")
        || path.contains("/examples/")
        || file_name.ends_with("_test.rs")
        || file_name.ends_with(".test.rs")
        || file_name.ends_with("_spec.rs")
}

fn is_dependency_path(path: &str) -> bool {
    path == "Cargo.toml" || path.ends_with("/Cargo.toml") || path == "Cargo.lock"
}

fn is_build_or_ci_path(path: &str) -> bool {
    path == "build.rs"
        || path.ends_with("/build.rs")
        || path.starts_with(".github/workflows/")
        || path == ".gitlab-ci.yml"
        || path == "azure-pipelines.yml"
        || path == "Jenkinsfile"
}
