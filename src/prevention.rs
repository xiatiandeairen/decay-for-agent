/// Prevention configuration generator.
///
/// Recommends lint rules, CI checks, and pre-commit hooks based on
/// detected issues to prevent recurrence at the source.

use serde::Serialize;

use crate::diagnose::{Issue, IssueCategory};

/// A prevention recommendation.
#[derive(Debug, Clone, Serialize)]
pub struct Prevention {
    pub tool: String,
    pub config_file: String,
    pub description: String,
    pub config_snippet: String,
}

/// Detect if the project is Rust-based from issue file paths.
fn is_rust_project(issues: &[Issue]) -> bool {
    issues.iter().any(|i| {
        i.actions.iter().any(|a| a.target.file.ends_with(".rs"))
    })
}

/// Detect if the project is Node-based from issue file paths.
fn is_node_project(issues: &[Issue]) -> bool {
    issues.iter().any(|i| {
        i.actions.iter().any(|a| {
            a.target.file.ends_with(".js") || a.target.file.ends_with(".ts")
        })
    })
}

/// Generate prevention recommendations based on detected issues.
pub fn generate_preventions(issues: &[Issue]) -> Vec<Prevention> {
    let mut result = Vec::new();
    let mut seen_tools: Vec<String> = Vec::new();
    let is_rust = is_rust_project(issues);
    let is_node = is_node_project(issues);

    for issue in issues {
        let candidates = match_preventions(issue, is_rust, is_node);
        for p in candidates {
            if !seen_tools.contains(&p.tool) {
                seen_tools.push(p.tool.clone());
                result.push(p);
            }
        }
    }

    result
}

fn match_preventions(issue: &Issue, is_rust: bool, is_node: bool) -> Vec<Prevention> {
    let msg = issue.message.to_lowercase();
    let cat = issue.classification;
    let mut result = Vec::new();

    // Dependency management
    if cat == Some(IssueCategory::Prevention) && msg.contains("direct dependencies") {
        if is_rust {
            result.push(Prevention {
                tool: "cargo-deny".into(),
                config_file: "deny.toml".into(),
                description: "audit dependencies for security, duplicates, and license compliance".into(),
                config_snippet: "[advisories]\nvulnerability = \"deny\"\n\n[licenses]\nunlicensed = \"deny\"\n\n[bans]\nmultiple-versions = \"warn\"\n".into(),
            });
        } else if is_node {
            result.push(Prevention {
                tool: "depcheck".into(),
                config_file: ".depcheckrc.json".into(),
                description: "detect unused dependencies".into(),
                config_snippet: "{\n  \"ignoreMatches\": [\"@types/*\"]\n}\n".into(),
            });
        } else {
            result.push(Prevention {
                tool: "dependency-audit".into(),
                config_file: "CI".into(),
                description: "add dependency audit to CI pipeline".into(),
                config_snippet: "# Add dependency audit step to your CI configuration\n".into(),
            });
        }
    }

    // Unwrap/panic prevention (Rust clippy)
    if cat == Some(IssueCategory::MechanicalFix) && msg.contains("unwrap/panic") && is_rust {
        result.push(Prevention {
            tool: "clippy".into(),
            config_file: "clippy.toml".into(),
            description: "deny unwrap and expect in production code".into(),
            config_snippet: "# In Cargo.toml or .cargo/config.toml:\n# [lints.clippy]\n# unwrap_used = \"deny\"\n# expect_used = \"warn\"\n".into(),
        });
    }

    // SQL injection prevention
    if cat == Some(IssueCategory::SecurityCritical) && (msg.contains("sql") || msg.contains("injection")) {
        result.push(Prevention {
            tool: "sql-lint".into(),
            config_file: "CI".into(),
            description: "use compile-time checked SQL queries or add SQL lint to CI".into(),
            config_snippet: "# Consider sqlx (Rust) or knex (Node) for safe query building\n# Or add a CI check:\n# grep -rn 'format!.*SELECT\\|INSERT\\|DELETE\\|UPDATE' src/ && exit 1\n".into(),
        });
    }

    // Credential leak prevention
    if cat == Some(IssueCategory::SecurityCritical) && msg.contains("credential") {
        result.push(Prevention {
            tool: "git-secrets".into(),
            config_file: ".pre-commit-config.yaml".into(),
            description: "prevent committing credentials with pre-commit hook".into(),
            config_snippet: "repos:\n  - repo: https://github.com/awslabs/git-secrets\n    rev: master\n    hooks:\n      - id: git-secrets\n".into(),
        });
    }

    // File size prevention (Rust)
    if cat == Some(IssueCategory::ArchitecturalDecision) && (msg.contains("kb)") || msg.contains("lines")) && is_rust {
        result.push(Prevention {
            tool: "clippy-file-lines".into(),
            config_file: "clippy.toml".into(),
            description: "enforce maximum file and function length".into(),
            config_snippet: "too-many-lines-threshold = 300\ntoo-large-for-stack = 200\n".into(),
        });
    }

    // Blocking call prevention (Rust)
    if cat == Some(IssueCategory::Prevention) && msg.contains("blocking call") && is_rust {
        result.push(Prevention {
            tool: "clippy-async".into(),
            config_file: "clippy.toml".into(),
            description: "detect blocking calls in async context".into(),
            config_snippet: "# [lints.clippy]\n# await_holding_lock = \"warn\"\n".into(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionType, Effort, Priority, Target};
    use crate::diagnose::Level;

    fn make_issue(dim: &str, msg: &str, cat: IssueCategory, file: &str) -> Issue {
        Issue {
            level: Level::Warning,
            category: dim.into(),
            message: msg.into(),
            classification: Some(cat),
            actions: vec![Action {
                dimension: dim.into(),
                action_type: ActionType::Replace,
                target: Target::file(file),
                suggestion: "fix".into(),
                reason: "broken".into(),
                priority: Priority::High,
                effort: Effort::Small,
            }],
        }
    }

    #[test]
    fn test_dependency_prevention_rust() {
        let issues = vec![make_issue("reliability", "80 direct dependencies", IssueCategory::Prevention, "src/main.rs")];
        let preventions = generate_preventions(&issues);
        assert_eq!(preventions.len(), 1);
        assert_eq!(preventions[0].tool, "cargo-deny");
    }

    #[test]
    fn test_dependency_prevention_node() {
        let issues = vec![make_issue("reliability", "80 direct dependencies", IssueCategory::Prevention, "src/app.ts")];
        let preventions = generate_preventions(&issues);
        assert_eq!(preventions.len(), 1);
        assert_eq!(preventions[0].tool, "depcheck");
    }

    #[test]
    fn test_unwrap_prevention() {
        let issues = vec![make_issue("observability", "src/a.rs has 10 unwrap/panic calls", IssueCategory::MechanicalFix, "src/a.rs")];
        let preventions = generate_preventions(&issues);
        assert_eq!(preventions.len(), 1);
        assert_eq!(preventions[0].tool, "clippy");
    }

    #[test]
    fn test_credential_prevention() {
        let issues = vec![make_issue("reliability", "src/config.rs:5: hardcoded credential detected", IssueCategory::SecurityCritical, "src/config.rs")];
        let preventions = generate_preventions(&issues);
        assert_eq!(preventions.len(), 1);
        assert_eq!(preventions[0].tool, "git-secrets");
    }

    #[test]
    fn test_no_duplicate_tools() {
        let issues = vec![
            make_issue("observability", "src/a.rs has 10 unwrap/panic calls", IssueCategory::MechanicalFix, "src/a.rs"),
            make_issue("observability", "src/b.rs has 8 unwrap/panic calls", IssueCategory::MechanicalFix, "src/b.rs"),
        ];
        let preventions = generate_preventions(&issues);
        assert_eq!(preventions.len(), 1);
    }

    #[test]
    fn test_no_preventions_for_non_matching() {
        let issues = vec![make_issue("structural", "max directory depth is 7", IssueCategory::ArchitecturalDecision, ".")];
        let preventions = generate_preventions(&issues);
        assert!(preventions.is_empty());
    }
}
