/// Issue classification engine.
///
/// Maps each issue to one of 8 categories (A-H) based on
/// dimension, message content, action type, and severity level.

use crate::action::ActionType;
use crate::diagnose::{Issue, IssueCategory, Level};

/// Classify all issues in-place.
pub fn classify_issues(issues: &mut [Issue]) {
    for issue in issues.iter_mut() {
        issue.classification = Some(classify(issue));
    }
}

/// Determine the category for a single issue.
fn classify(issue: &Issue) -> IssueCategory {
    let dim = issue.category.as_str();
    let msg = issue.message.to_lowercase();
    let action_type = issue.actions.first().map(|a| &a.action_type);

    // D: Security Critical — injection and credentials
    if msg.contains("injection") || msg.contains("sql string concatenation")
        || msg.contains("shell command") || msg.contains("hardcoded credential")
    {
        return IssueCategory::SecurityCritical;
    }

    // A: Mechanical Fix — per-file issues with clear Replace/Add pattern
    if dim == "observability" {
        if msg.contains("unwrap/panic calls") {
            return IssueCategory::MechanicalFix;
        }
        if msg.contains("empty catch") {
            return IssueCategory::MechanicalFix;
        }
        if msg.contains("hardcoded configuration") {
            return IssueCategory::MechanicalFix;
        }
        if msg.contains("no logging") {
            return IssueCategory::ConventionDrift;
        }
    }

    // G: Contextual Exception — unsafe code may be legitimate (FFI, etc.)
    if dim == "reliability" && msg.contains("unsafe/eval") {
        return IssueCategory::ContextualException;
    }

    // H: Prevention — dependency management, blocking calls
    if dim == "reliability" && msg.contains("direct dependencies") {
        return IssueCategory::Prevention;
    }
    if dim == "performance" && msg.contains("blocking call") {
        return IssueCategory::Prevention;
    }

    // B: Pattern Problem — duplication, density/ratio issues
    if dim == "maintainability" && msg.contains("duplicate") {
        return IssueCategory::PatternProblem;
    }
    if dim == "maintainability" && msg.contains("todo/fixme") {
        return IssueCategory::ChronicDecay;
    }
    if dim == "performance" && msg.contains("clone/copy") {
        return IssueCategory::PatternProblem;
    }

    // F: Chronic Decay — ratio/percentage Info-level indicators
    if issue.level == Level::Info {
        if msg.contains("% of files") || msg.contains("ratio")
            || msg.contains("top-level entries") || msg.contains("changed") && msg.contains("times")
        {
            return IssueCategory::ChronicDecay;
        }
    }

    // E: Convention Drift — test coverage gaps
    if dim == "quality" {
        if msg.contains("no test files") || msg.contains("no corresponding test") {
            return IssueCategory::ConventionDrift;
        }
        if msg.contains("% of files are tests") || msg.contains("test/source line ratio") {
            return IssueCategory::ConventionDrift;
        }
    }

    // C: Architectural Decision — Split/Refactor actions on structure issues
    if matches!(action_type, Some(ActionType::Split) | Some(ActionType::Refactor)) {
        return IssueCategory::ArchitecturalDecision;
    }

    // C: structural dimension issues are typically architectural
    if dim == "structural" {
        return IssueCategory::ArchitecturalDecision;
    }

    // C: performance nested loops
    if dim == "performance" && msg.contains("nested loop") {
        return IssueCategory::ArchitecturalDecision;
    }

    // B: Extract actions suggest pattern problems
    if matches!(action_type, Some(ActionType::Extract)) {
        return IssueCategory::PatternProblem;
    }

    // Default: Architectural Decision (most conservative)
    IssueCategory::ArchitecturalDecision
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, Effort, Priority, Target};

    fn issue(dim: &str, msg: &str, level: Level, actions: Vec<Action>) -> Issue {
        Issue {
            level,
            category: dim.into(),
            message: msg.into(),
            classification: None,
            actions,
        }
    }

    fn action(action_type: ActionType) -> Action {
        Action {
            dimension: "test".into(),
            action_type,
            target: Target::file("test.rs"),
            suggestion: "fix it".into(),
            reason: "broken".into(),
            priority: Priority::High,
            effort: Effort::Small,
        }
    }

    #[test]
    fn test_security_critical() {
        let i = issue("reliability", "src/db.rs:42: potential SQL string concatenation", Level::Critical, vec![]);
        assert_eq!(classify(&i), IssueCategory::SecurityCritical);

        let i = issue("reliability", "src/config.rs:10: hardcoded credential detected", Level::Critical, vec![]);
        assert_eq!(classify(&i), IssueCategory::SecurityCritical);
    }

    #[test]
    fn test_mechanical_fix() {
        let i = issue("observability", "src/main.rs has 10 unwrap/panic calls", Level::Warning, vec![action(ActionType::Replace)]);
        assert_eq!(classify(&i), IssueCategory::MechanicalFix);

        let i = issue("observability", "3 empty catch/except blocks detected", Level::Warning, vec![]);
        assert_eq!(classify(&i), IssueCategory::MechanicalFix);
    }

    #[test]
    fn test_contextual_exception() {
        let i = issue("reliability", "src/ffi.rs has 8 unsafe/eval occurrences", Level::Warning, vec![]);
        assert_eq!(classify(&i), IssueCategory::ContextualException);
    }

    #[test]
    fn test_prevention() {
        let i = issue("reliability", "80 direct dependencies", Level::Info, vec![]);
        assert_eq!(classify(&i), IssueCategory::Prevention);

        let i = issue("performance", "src/main.rs: blocking call thread::sleep", Level::Info, vec![]);
        assert_eq!(classify(&i), IssueCategory::Prevention);
    }

    #[test]
    fn test_pattern_problem() {
        let i = issue("maintainability", "src/a.rs has 2 duplicate block(s) shared with other files", Level::Warning, vec![]);
        assert_eq!(classify(&i), IssueCategory::PatternProblem);

        let i = issue("performance", "src/lib.rs has 15 clone/copy calls", Level::Warning, vec![action(ActionType::Refactor)]);
        assert_eq!(classify(&i), IssueCategory::PatternProblem);
    }

    #[test]
    fn test_convention_drift() {
        let i = issue("observability", "no logging framework detected in project", Level::Warning, vec![]);
        assert_eq!(classify(&i), IssueCategory::ConventionDrift);

        let i = issue("quality", "no test files found in project", Level::Critical, vec![]);
        assert_eq!(classify(&i), IssueCategory::ConventionDrift);

        let i = issue("quality", "only 5% of files are tests", Level::Warning, vec![]);
        assert_eq!(classify(&i), IssueCategory::ConventionDrift);
    }

    #[test]
    fn test_chronic_decay() {
        let i = issue("maintainability", "15 TODO/FIXME comments across project", Level::Info, vec![]);
        assert_eq!(classify(&i), IssueCategory::ChronicDecay);

        let i = issue("complexity", "30% of files exceed 15KB", Level::Info, vec![]);
        assert_eq!(classify(&i), IssueCategory::ChronicDecay);
    }

    #[test]
    fn test_architectural_decision() {
        let i = issue("complexity", "src/big.rs (52KB)", Level::Critical, vec![action(ActionType::Split)]);
        assert_eq!(classify(&i), IssueCategory::ArchitecturalDecision);

        let i = issue("structural", "1200 files in project", Level::Critical, vec![action(ActionType::Split)]);
        assert_eq!(classify(&i), IssueCategory::ArchitecturalDecision);
    }

    #[test]
    fn test_classify_issues_fills_all() {
        let mut issues = vec![
            issue("observability", "src/a.rs has 6 unwrap/panic calls", Level::Warning, vec![]),
            issue("reliability", "src/db.rs:1: potential SQL string concatenation", Level::Critical, vec![]),
            issue("structural", "max directory depth is 7", Level::Warning, vec![]),
        ];
        classify_issues(&mut issues);
        assert!(issues.iter().all(|i| i.classification.is_some()));
        assert_eq!(issues[0].classification, Some(IssueCategory::MechanicalFix));
        assert_eq!(issues[1].classification, Some(IssueCategory::SecurityCritical));
        assert_eq!(issues[2].classification, Some(IssueCategory::ArchitecturalDecision));
    }
}
