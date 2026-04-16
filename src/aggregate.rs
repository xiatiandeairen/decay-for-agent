/// Pattern aggregation engine.
///
/// Groups similar issues by root cause, replacing N individual issues
/// with a single aggregated diagnosis + unified solution approach.

use serde::Serialize;

use crate::action::ActionType;
use crate::diagnose::{Issue, IssueCategory};

/// An aggregated diagnosis grouping multiple related issues.
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedIssue {
    pub root_cause: String,
    pub category: IssueCategory,
    pub issue_count: usize,
    pub affected_files: Vec<String>,
    pub suggested_approach: String,
}

/// Aggregation rule: defines when and how to group issues.
struct AggRule {
    category: IssueCategory,
    dimension: &'static str,
    action_type: Option<ActionType>,
    message_contains: Option<&'static str>,
    min_count: usize,
    root_cause: &'static str,
    approach: &'static str,
}

const RULES: &[AggRule] = &[
    AggRule {
        category: IssueCategory::PatternProblem,
        dimension: "maintainability",
        action_type: Some(ActionType::Extract),
        message_contains: Some("duplicate"),
        min_count: 2,
        root_cause: "missing shared abstraction",
        approach: "extract duplicated logic into a common module",
    },
    AggRule {
        category: IssueCategory::MechanicalFix,
        dimension: "observability",
        action_type: Some(ActionType::Replace),
        message_contains: Some("unwrap/panic"),
        min_count: 3,
        root_cause: "missing unified error type",
        approach: "design a project Error enum and replace unwrap/panic with ? operator",
    },
    AggRule {
        category: IssueCategory::PatternProblem,
        dimension: "performance",
        action_type: Some(ActionType::Refactor),
        message_contains: Some("clone/copy"),
        min_count: 2,
        root_cause: "ownership design issues",
        approach: "introduce borrowing, references, or Cow to reduce cloning",
    },
    AggRule {
        category: IssueCategory::PatternProblem,
        dimension: "maintainability",
        action_type: Some(ActionType::Extract),
        message_contains: Some("lines long"),
        min_count: 3,
        root_cause: "functions exceeding size threshold",
        approach: "break functions into smaller, focused units",
    },
    AggRule {
        category: IssueCategory::ArchitecturalDecision,
        dimension: "performance",
        action_type: Some(ActionType::Extract),
        message_contains: Some("nested loop"),
        min_count: 3,
        root_cause: "deeply nested iteration patterns",
        approach: "extract inner loops into iterators or helper functions",
    },
    AggRule {
        category: IssueCategory::ArchitecturalDecision,
        dimension: "fragility",
        action_type: Some(ActionType::Split),
        message_contains: None,
        min_count: 2,
        root_cause: "high-churn file concentration",
        approach: "isolate frequently changing logic into stable/unstable boundaries",
    },
    AggRule {
        category: IssueCategory::ArchitecturalDecision,
        dimension: "complexity",
        action_type: Some(ActionType::Split),
        message_contains: None,
        min_count: 3,
        root_cause: "unclear module responsibilities",
        approach: "split modules by responsibility, extract cohesive units",
    },
    AggRule {
        category: IssueCategory::ArchitecturalDecision,
        dimension: "maintainability",
        action_type: Some(ActionType::Split),
        message_contains: None,
        min_count: 3,
        root_cause: "unclear module responsibilities",
        approach: "split modules by responsibility, extract cohesive units",
    },
    AggRule {
        category: IssueCategory::SecurityCritical,
        dimension: "reliability",
        action_type: Some(ActionType::Replace),
        message_contains: Some("sql"),
        min_count: 2,
        root_cause: "missing parameterized query layer",
        approach: "introduce a query builder or use parameterized queries consistently",
    },
];

/// Aggregate classified issues into root-cause groups.
/// Returns aggregated issues for groups meeting the minimum count threshold.
pub fn aggregate_issues(issues: &[Issue]) -> Vec<AggregatedIssue> {
    let mut result = Vec::new();

    for rule in RULES {
        let matching: Vec<&Issue> = issues
            .iter()
            .filter(|i| {
                let cat_match = i.classification == Some(rule.category);
                let dim_match = i.category == rule.dimension;
                let action_match = match &rule.action_type {
                    Some(at) => i.actions.first().map(|a| &a.action_type) == Some(at),
                    None => true,
                };
                let msg_match = match rule.message_contains {
                    Some(pat) => i.message.to_lowercase().contains(pat),
                    None => true,
                };
                cat_match && dim_match && action_match && msg_match
            })
            .collect();

        if matching.len() >= rule.min_count {
            let affected_files: Vec<String> = matching
                .iter()
                .flat_map(|i| i.actions.iter().map(|a| a.target.file.clone()))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let mut files = affected_files;
            files.sort();

            result.push(AggregatedIssue {
                root_cause: rule.root_cause.to_string(),
                category: rule.category,
                issue_count: matching.len(),
                affected_files: files,
                suggested_approach: rule.approach.to_string(),
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionType;
    use crate::test_helpers::make_issue;

    #[test]
    fn test_aggregate_duplicate_code() {
        let issues = vec![
            make_issue("maintainability", "src/a.rs has 2 duplicate block(s) shared with other files",
                IssueCategory::PatternProblem, ActionType::Extract, "src/a.rs"),
            make_issue("maintainability", "src/b.rs has 1 duplicate block(s) shared with other files",
                IssueCategory::PatternProblem, ActionType::Extract, "src/b.rs"),
        ];
        let agg = aggregate_issues(&issues);
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].root_cause, "missing shared abstraction");
        assert_eq!(agg[0].issue_count, 2);
        assert_eq!(agg[0].affected_files.len(), 2);
    }

    #[test]
    fn test_aggregate_unwrap_pattern() {
        let issues: Vec<Issue> = (0..4)
            .map(|i| make_issue(
                "observability",
                &format!("src/mod{i}.rs has 8 unwrap/panic calls"),
                IssueCategory::MechanicalFix,
                ActionType::Replace,
                &format!("src/mod{i}.rs"),
            ))
            .collect();
        let agg = aggregate_issues(&issues);
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].root_cause, "missing unified error type");
        assert_eq!(agg[0].issue_count, 4);
    }

    #[test]
    fn test_no_aggregation_below_threshold() {
        let issues = vec![
            make_issue("observability", "src/a.rs has 6 unwrap/panic calls",
                IssueCategory::MechanicalFix, ActionType::Replace, "src/a.rs"),
        ];
        let agg = aggregate_issues(&issues);
        assert!(agg.is_empty()); // 1 < min_count of 3
    }

    #[test]
    fn test_aggregate_split_pattern() {
        let issues: Vec<Issue> = (0..3)
            .map(|i| make_issue(
                "complexity",
                &format!("src/big{i}.rs (52KB)"),
                IssueCategory::ArchitecturalDecision,
                ActionType::Split,
                &format!("src/big{i}.rs"),
            ))
            .collect();
        let agg = aggregate_issues(&issues);
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].root_cause, "unclear module responsibilities");
    }

    #[test]
    fn test_aggregate_long_functions() {
        let issues: Vec<Issue> = (0..5)
            .map(|i| make_issue(
                "maintainability",
                &format!("process_data in src/mod{i}.rs is 180 lines long"),
                IssueCategory::PatternProblem,
                ActionType::Extract,
                &format!("src/mod{i}.rs"),
            ))
            .collect();
        let agg = aggregate_issues(&issues);
        assert!(agg.iter().any(|a| a.root_cause == "functions exceeding size threshold"));
        let matched = agg.iter().find(|a| a.root_cause == "functions exceeding size threshold").unwrap();
        assert_eq!(matched.issue_count, 5);
    }

    #[test]
    fn test_aggregate_nested_loops() {
        let issues: Vec<Issue> = (0..4)
            .map(|i| make_issue(
                "performance",
                &format!("src/scan{i}.rs:42 has 3-level nested loop"),
                IssueCategory::ArchitecturalDecision,
                ActionType::Extract,
                &format!("src/scan{i}.rs"),
            ))
            .collect();
        let agg = aggregate_issues(&issues);
        assert!(agg.iter().any(|a| a.root_cause == "deeply nested iteration patterns"));
        let matched = agg.iter().find(|a| a.root_cause == "deeply nested iteration patterns").unwrap();
        assert_eq!(matched.issue_count, 4);
    }

    #[test]
    fn test_aggregate_fragility_split() {
        let issues = vec![
            make_issue("fragility", "src/core.rs has 450 lines churn",
                IssueCategory::ArchitecturalDecision, ActionType::Split, "src/core.rs"),
            make_issue("fragility", "src/engine.rs has 320 lines churn",
                IssueCategory::ArchitecturalDecision, ActionType::Split, "src/engine.rs"),
        ];
        let agg = aggregate_issues(&issues);
        assert!(agg.iter().any(|a| a.root_cause == "high-churn file concentration"));
        let matched = agg.iter().find(|a| a.root_cause == "high-churn file concentration").unwrap();
        assert_eq!(matched.issue_count, 2);
        assert_eq!(matched.affected_files.len(), 2);
    }

    #[test]
    fn test_aggregate_sql_injection() {
        let issues = vec![
            make_issue("reliability", "src/db.rs:10: potential SQL string concatenation",
                IssueCategory::SecurityCritical, ActionType::Replace, "src/db.rs"),
            make_issue("reliability", "src/query.rs:5: potential SQL string concatenation",
                IssueCategory::SecurityCritical, ActionType::Replace, "src/query.rs"),
        ];
        let agg = aggregate_issues(&issues);
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].root_cause, "missing parameterized query layer");
    }
}
