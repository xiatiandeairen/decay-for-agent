/// Structured diagnostic report combining classification, aggregation, patches, and preventions.
///
/// This is the v6 "intelligent report" — instead of a flat issue list,
/// issues are organized by category with appropriate handling strategies.

use std::collections::HashMap;

use serde::Serialize;

use crate::aggregate::AggregatedIssue;
use crate::diagnose::{Issue, IssueCategory};
use crate::patch::Patch;
use crate::prevention::Prevention;

/// A category section within the diagnostic report.
#[derive(Debug, Clone, Serialize)]
pub struct CategorySection {
    pub category: IssueCategory,
    pub label: String,
    pub strategy: String,
    pub issue_count: usize,
    pub issues: Vec<IssueSummary>,
}

/// Compact issue summary for report sections.
#[derive(Debug, Clone, Serialize)]
pub struct IssueSummary {
    pub dimension: String,
    pub message: String,
    pub level: String,
}

/// The unified diagnostic report.
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticReport {
    pub total_issues: usize,
    pub sections: Vec<CategorySection>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub root_causes: Vec<AggregatedIssue>,
    pub patch_count: usize,
    pub prevention_count: usize,
}

const CATEGORY_INFO: &[(IssueCategory, &str, &str)] = &[
    (IssueCategory::SecurityCritical, "Security Critical", "precise fix, mandatory review"),
    (IssueCategory::MechanicalFix, "Mechanical Fix", "pattern-match, generate patch"),
    (IssueCategory::PatternProblem, "Pattern Problem", "aggregate similar, unified solution"),
    (IssueCategory::ArchitecturalDecision, "Architectural Decision", "analyze trade-offs, output options"),
    (IssueCategory::ConventionDrift, "Convention Drift", "detect inconsistency, alignment plan"),
    (IssueCategory::ContextualException, "Contextual Exception", "may be legitimate, review context"),
    (IssueCategory::ChronicDecay, "Chronic Decay", "trending worse, preventive action"),
    (IssueCategory::Prevention, "Prevention", "recommend toolchain/CI config"),
];

/// Build a diagnostic report from classified issues and v6 outputs.
pub fn build_diagnostic_report(
    issues: &[Issue],
    aggregated: &[AggregatedIssue],
    patches: &[Patch],
    preventions: &[Prevention],
) -> DiagnosticReport {
    // Group issues by category
    let mut by_category: HashMap<IssueCategory, Vec<&Issue>> = HashMap::new();
    for issue in issues {
        if let Some(cat) = issue.classification {
            by_category.entry(cat).or_default().push(issue);
        }
    }

    // Build sections in priority order
    let mut sections = Vec::new();
    for (cat, label, strategy) in CATEGORY_INFO {
        if let Some(cat_issues) = by_category.get(cat) {
            let summaries: Vec<IssueSummary> = cat_issues
                .iter()
                .map(|i| IssueSummary {
                    dimension: i.category.clone(),
                    message: i.message.clone(),
                    level: format!("{}", i.level),
                })
                .collect();

            sections.push(CategorySection {
                category: *cat,
                label: label.to_string(),
                strategy: strategy.to_string(),
                issue_count: summaries.len(),
                issues: summaries,
            });
        }
    }

    DiagnosticReport {
        total_issues: issues.len(),
        sections,
        root_causes: aggregated.to_vec(),
        patch_count: patches.len(),
        prevention_count: preventions.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionType, Effort, Priority, Target};
    use crate::diagnose::Level;

    fn make_issue(dim: &str, msg: &str, cat: IssueCategory) -> Issue {
        Issue {
            level: Level::Warning,
            category: dim.into(),
            message: msg.into(),
            classification: Some(cat),
            actions: vec![Action {
                dimension: dim.into(),
                action_type: ActionType::Replace,
                target: Target::file("test.rs"),
                suggestion: "fix".into(),
                reason: "reason".into(),
                priority: Priority::High,
                effort: Effort::Small,
            }],
        }
    }

    #[test]
    fn test_build_report_groups_by_category() {
        let issues = vec![
            make_issue("reliability", "SQL injection", IssueCategory::SecurityCritical),
            make_issue("reliability", "hardcoded credential", IssueCategory::SecurityCritical),
            make_issue("observability", "unwrap calls", IssueCategory::MechanicalFix),
            make_issue("structural", "deep nesting", IssueCategory::ArchitecturalDecision),
        ];

        let report = build_diagnostic_report(&issues, &[], &[], &[]);
        assert_eq!(report.total_issues, 4);
        assert_eq!(report.sections.len(), 3); // 3 categories
        // Security first in order
        assert_eq!(report.sections[0].category, IssueCategory::SecurityCritical);
        assert_eq!(report.sections[0].issue_count, 2);
        assert_eq!(report.sections[1].category, IssueCategory::MechanicalFix);
        assert_eq!(report.sections[2].category, IssueCategory::ArchitecturalDecision);
    }

    #[test]
    fn test_build_report_empty() {
        let report = build_diagnostic_report(&[], &[], &[], &[]);
        assert_eq!(report.total_issues, 0);
        assert!(report.sections.is_empty());
    }

    #[test]
    fn test_build_report_counts() {
        let issues = vec![make_issue("obs", "unwrap", IssueCategory::MechanicalFix)];
        let patches = vec![Patch {
            file: "a.rs".into(), line: 1,
            original: "x.unwrap()".into(), replacement: "x?".into(),
            description: "fix".into(),
        }];
        let preventions = vec![Prevention {
            tool: "clippy".into(), config_file: "clippy.toml".into(),
            description: "deny unwrap".into(), config_snippet: "...".into(),
        }];

        let report = build_diagnostic_report(&issues, &[], &patches, &preventions);
        assert_eq!(report.patch_count, 1);
        assert_eq!(report.prevention_count, 1);
    }
}
