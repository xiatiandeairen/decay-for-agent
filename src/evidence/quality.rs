use crate::evidence::EvidenceRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityFileChange {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub added_lines: u32,
    pub deleted_lines: u32,
    pub sensitive: bool,
}

impl QualityFileChange {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            old_path: None,
            new_path: Some(path.into()),
            added_lines: 0,
            deleted_lines: 0,
            sensitive: false,
        }
    }

    fn path(&self) -> Option<&str> {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .filter(|path| !path.trim().is_empty())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QualityReport {
    pub records: Vec<EvidenceRecord>,
    pub evidence: Vec<EvidenceRecord>,
    pub prod_files_changed: usize,
    pub test_files_changed: usize,
    pub docs_files_changed: usize,
}

pub fn detect_quality(files: &[QualityFileChange]) -> QualityReport {
    let mut report = QualityReport::default();
    let mut prod_domains = Vec::new();
    let mut crates = Vec::new();
    let mut has_sensitive_prod = false;
    let mut first_prod_file = None;

    for file in files {
        let Some(path) = file.path() else {
            continue;
        };

        if is_ignored_generated_path(path) {
            continue;
        }

        if is_docs_path(path) {
            report.docs_files_changed += 1;
            continue;
        }

        if is_test_path(path) {
            report.test_files_changed += 1;
            continue;
        }

        if is_dependency_path(path) {
            push_record(
                &mut report,
                "dependency_changed",
                path,
                "dependency manifest",
            );
            continue;
        }

        if is_build_or_ci_path(path) {
            push_record(&mut report, "build_or_ci_changed", path, "build or ci");
            continue;
        }

        if is_prod_path(path) {
            report.prod_files_changed += 1;
            if first_prod_file.is_none() {
                first_prod_file = Some(path.to_string());
            }
            push_unique(&mut prod_domains, domain_for(path));
            push_unique(&mut crates, crate_for(path));

            if file.sensitive || is_sensitive_path(path) {
                has_sensitive_prod = true;
            }
        }
    }

    if report.prod_files_changed > 0 && report.test_files_changed == 0 {
        let file = first_prod_file.as_deref().unwrap_or("");
        push_record(
            &mut report,
            "test_gap",
            file,
            "production change without tests",
        );

        if has_sensitive_prod {
            push_record(
                &mut report,
                "sensitive_test_gap",
                file,
                "sensitive production change without tests",
            );
        }
    }

    if crates.len() > 1 || prod_domains.len() > 2 {
        let file = first_prod_file.as_deref().unwrap_or("");
        push_record(
            &mut report,
            "blast_radius",
            file,
            "multiple crates or domains",
        );
    }

    if prod_domains.len() > 1 {
        let file = first_prod_file.as_deref().unwrap_or("");
        push_record(
            &mut report,
            "mixed_concerns",
            file,
            "multiple production domains",
        );
    }

    report.evidence = report.records.clone();
    report
}

fn push_record(report: &mut QualityReport, kind: &str, file: &str, subject: &str) {
    report
        .records
        .push(EvidenceRecord::new(kind, file, subject, "quality"));
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|item| item == &value) {
        items.push(value);
    }
}

fn is_docs_path(path: &str) -> bool {
    path.starts_with("docs/")
        || path == "README.md"
        || path == "README.zh.md"
        || path.ends_with(".md")
        || path.ends_with(".adoc")
        || path.ends_with(".rst")
}

fn is_test_path(path: &str) -> bool {
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with(".test.rs")
        || path.ends_with("_spec.rs")
}

fn is_dependency_path(path: &str) -> bool {
    path == "Cargo.toml" || path.ends_with("/Cargo.toml")
}

fn is_build_or_ci_path(path: &str) -> bool {
    path == "build.rs"
        || path.ends_with("/build.rs")
        || path.starts_with(".github/workflows/")
        || path == ".gitlab-ci.yml"
        || path == "azure-pipelines.yml"
        || path == "Jenkinsfile"
}

fn is_prod_path(path: &str) -> bool {
    path.starts_with("src/") || path.starts_with("crates/")
}

fn is_ignored_generated_path(path: &str) -> bool {
    path.split('/')
        .any(|part| part == "target" || part == "generated")
}

fn is_sensitive_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        "auth",
        "token",
        "permission",
        "security",
        "crypto",
        "payment",
        "billing",
        "secret",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn crate_for(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"crates") && parts.len() > 1 {
        parts[1].to_string()
    } else {
        "root".to_string()
    }
}

fn domain_for(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"crates") && parts.len() > 3 {
        return parts[1].to_string();
    }
    if parts.first() == Some(&"src") && parts.len() > 1 {
        return parts[1].strip_suffix(".rs").unwrap_or(parts[1]).to_string();
    }
    path.to_string()
}
