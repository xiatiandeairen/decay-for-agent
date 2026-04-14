use log::debug;

use crate::filter_pipeline;
use crate::git::GitChange;

/// Context for git change filtering.
pub struct GitFilterContext {
    /// Primary language groups detected from file scan.
    pub primary_languages: Vec<String>,
}

/// A stage in the git filter pipeline.
pub trait GitFilterStage: Send + Sync {
    fn name(&self) -> &'static str;
    fn filter(&self, changes: Vec<GitChange>, ctx: &GitFilterContext) -> Vec<GitChange>;
}

/// Run all git filter stages in sequence.
pub fn run_pipeline(changes: Vec<GitChange>, ctx: &GitFilterContext) -> Vec<GitChange> {
    let stages: Vec<Box<dyn GitFilterStage>> = vec![
        Box::new(LanguageFilter),
        Box::new(LockFileFilter),
        Box::new(GeneratedFileFilter),
    ];

    let mut result = changes;
    for stage in &stages {
        let before = result.len();
        result = stage.filter(result, ctx);
        let after = result.len();
        if before != after {
            debug!("git filter {}: {} → {} changes", stage.name(), before, after);
        }
    }
    result
}

// ============================================================
// LanguageFilter: only keep changes to primary language files
// ============================================================

struct LanguageFilter;

impl GitFilterStage for LanguageFilter {
    fn name(&self) -> &'static str {
        "language"
    }

    fn filter(&self, changes: Vec<GitChange>, ctx: &GitFilterContext) -> Vec<GitChange> {
        if ctx.primary_languages.is_empty() {
            return changes;
        }

        changes
            .into_iter()
            .filter(|c| {
                let ext = std::path::Path::new(&c.path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                // Check if this extension belongs to any primary language group
                for group in filter_pipeline::LANGUAGE_GROUPS {
                    if ctx.primary_languages.iter().any(|l| l == group.name)
                        && group.extensions.contains(&ext.as_str())
                    {
                        return true;
                    }
                }
                false
            })
            .collect()
    }
}

// ============================================================
// LockFileFilter: exclude lock files
// ============================================================

struct LockFileFilter;

impl GitFilterStage for LockFileFilter {
    fn name(&self) -> &'static str {
        "lock_file"
    }

    fn filter(&self, changes: Vec<GitChange>, _ctx: &GitFilterContext) -> Vec<GitChange> {
        changes
            .into_iter()
            .filter(|c| {
                let lower = c.path.to_lowercase();
                !lower.ends_with(".lock") && !lower.ends_with("lock.json")
            })
            .collect()
    }
}

// ============================================================
// GeneratedFileFilter: exclude generated/config files
// ============================================================

struct GeneratedFileFilter;

impl GitFilterStage for GeneratedFileFilter {
    fn name(&self) -> &'static str {
        "generated"
    }

    fn filter(&self, changes: Vec<GitChange>, _ctx: &GitFilterContext) -> Vec<GitChange> {
        let generated_patterns = [
            ".pbxproj", ".xcassets", "Contents.json",
            ".min.js", ".min.css",
            "package-lock.json", "yarn.lock",
        ];

        changes
            .into_iter()
            .filter(|c| {
                !generated_patterns.iter().any(|p| c.path.contains(p))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_change(path: &str, churn: i64) -> GitChange {
        GitChange {
            path: path.to_string(),
            change_count: 1,
            lines_added: churn / 2,
            lines_deleted: churn / 2,
            last_modified: "2026-04-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_language_filter() {
        let changes = vec![
            make_change("src/main.swift", 100),
            make_change("src/Helper.m", 50),
            make_change("emoji.json", 620000),
            make_change("style.css", 200),
        ];
        let ctx = GitFilterContext {
            primary_languages: vec!["swift".to_string(), "objc".to_string()],
        };
        let result = LanguageFilter.filter(changes, &ctx);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "src/main.swift");
        assert_eq!(result[1].path, "src/Helper.m");
    }

    #[test]
    fn test_lock_file_filter() {
        let changes = vec![
            make_change("src/main.rs", 100),
            make_change("Cargo.lock", 5000),
            make_change("package-lock.json", 3000),
        ];
        let ctx = GitFilterContext { primary_languages: vec![] };
        let result = LockFileFilter.filter(changes, &ctx);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/main.rs");
    }

    #[test]
    fn test_generated_file_filter() {
        let changes = vec![
            make_change("src/main.swift", 100),
            make_change("App.xcodeproj/project.pbxproj", 500),
            make_change("Assets.xcassets/image/Contents.json", 30),
        ];
        let ctx = GitFilterContext { primary_languages: vec![] };
        let result = GeneratedFileFilter.filter(changes, &ctx);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/main.swift");
    }

    #[test]
    fn test_full_pipeline() {
        let changes = vec![
            make_change("src/main.swift", 100),
            make_change("src/Helper.m", 50),
            make_change("emoji.json", 620000),
            make_change("Cargo.lock", 5000),
            make_change("project.pbxproj", 500),
        ];
        let ctx = GitFilterContext {
            primary_languages: vec!["swift".to_string(), "objc".to_string()],
        };
        let result = run_pipeline(changes, &ctx);
        assert_eq!(result.len(), 2);
    }
}
