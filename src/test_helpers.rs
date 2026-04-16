/// Shared test helpers used across multiple test modules.
///
/// Contains factory functions for constructing `Issue`, `SourceFile`,
/// and related test data without boilerplate.

use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::SourceFile;
use crate::diagnose::{Issue, IssueCategory, Level};

/// Create a classified issue with a single action.
///
/// Uses sensible defaults (Warning level, High priority, Medium effort)
/// suitable for most aggregation/prevention/report tests.
pub fn make_issue(
    dim: &str,
    msg: &str,
    cat: IssueCategory,
    action_type: ActionType,
    file: &str,
) -> Issue {
    Issue {
        level: Level::Warning,
        category: dim.into(),
        message: msg.into(),
        classification: Some(cat),
        actions: vec![Action {
            dimension: dim.into(),
            action_type,
            target: Target::file(file),
            suggestion: "fix".into(),
            reason: "broken".into(),
            priority: Priority::High,
            effort: Effort::Medium,
            details: vec![],
            impact: None,
            verify: String::new(),
        }],
    }
}

/// Create a `SourceFile` from a path and content string.
pub fn make_source_file(path: &str, content: &str) -> SourceFile {
    let lines: Vec<String> = content.lines().map(Into::into).collect();
    let line_count = lines.len();
    SourceFile {
        path: path.into(),
        content: content.into(),
        lines,
        line_count,
    }
}
