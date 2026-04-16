/// Issue classification engine.
///
/// Maps each issue to one of 8 categories (A-H) based on
/// dimension, message content, action type, and severity level.
/// Uses a data-driven rule table — the first matching rule wins.

use crate::action::ActionType;
use crate::diagnose::{Issue, IssueCategory, Level};

/// A single classification rule.
/// All `Some` fields must match; `None` fields are wildcards.
struct ClassifyRule {
    /// Required dimension (None = any).
    dimension: Option<&'static str>,
    /// Message must contain ANY of these substrings (None = skip check).
    message_any: Option<&'static [&'static str]>,
    /// Message must contain ALL of these substrings (None = skip check).
    message_all: Option<&'static [&'static str]>,
    /// Required issue level (None = any).
    level: Option<Level>,
    /// Required action type on the first action (None = any).
    action_type: Option<ActionType>,
    /// Category to assign when all conditions match.
    category: IssueCategory,
}

impl ClassifyRule {
    /// Check whether this rule matches the given issue context.
    fn matches(&self, dim: &str, msg: &str, level: Level, action_type: Option<&ActionType>) -> bool {
        if let Some(d) = self.dimension {
            if dim != d {
                return false;
            }
        }
        if let Some(pats) = self.message_any {
            if !pats.iter().any(|p| msg.contains(p)) {
                return false;
            }
        }
        if let Some(pats) = self.message_all {
            if !pats.iter().all(|p| msg.contains(p)) {
                return false;
            }
        }
        if let Some(l) = self.level {
            if level != l {
                return false;
            }
        }
        if let Some(ref at) = self.action_type {
            if action_type != Some(at) {
                return false;
            }
        }
        true
    }
}

/// Classification rules evaluated in priority order — first match wins.
const RULES: &[ClassifyRule] = &[
    // D: Security Critical — injection and credentials
    ClassifyRule {
        dimension: None,
        message_any: Some(&["injection", "sql string concatenation", "shell command", "hardcoded credential"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::SecurityCritical,
    },
    // A: Mechanical Fix — observability per-file issues
    ClassifyRule {
        dimension: Some("observability"),
        message_any: Some(&["unwrap/panic calls", "empty catch", "hardcoded configuration"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::MechanicalFix,
    },
    // E: Convention Drift — observability no-logging
    ClassifyRule {
        dimension: Some("observability"),
        message_any: Some(&["no logging"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::ConventionDrift,
    },
    // G: Contextual Exception — unsafe code may be legitimate
    ClassifyRule {
        dimension: Some("reliability"),
        message_any: Some(&["unsafe/eval"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::ContextualException,
    },
    // H: Prevention — dependency management
    ClassifyRule {
        dimension: Some("reliability"),
        message_any: Some(&["direct dependencies"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::Prevention,
    },
    // H: Prevention — blocking calls
    ClassifyRule {
        dimension: Some("performance"),
        message_any: Some(&["blocking call"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::Prevention,
    },
    // B: Pattern Problem — duplication
    ClassifyRule {
        dimension: Some("maintainability"),
        message_any: Some(&["duplicate"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::PatternProblem,
    },
    // F: Chronic Decay — TODO/FIXME
    ClassifyRule {
        dimension: Some("maintainability"),
        message_any: Some(&["todo/fixme"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::ChronicDecay,
    },
    // B: Pattern Problem — clone/copy
    ClassifyRule {
        dimension: Some("performance"),
        message_any: Some(&["clone/copy"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::PatternProblem,
    },
    // F: Chronic Decay — ratio/percentage Info-level indicators
    ClassifyRule {
        dimension: None,
        message_any: Some(&["% of files", "ratio", "top-level entries"]),
        message_all: None, level: Some(Level::Info), action_type: None,
        category: IssueCategory::ChronicDecay,
    },
    // F: Chronic Decay — churn indicator (changed N times)
    ClassifyRule {
        dimension: None,
        message_any: None,
        message_all: Some(&["changed", "times"]),
        level: Some(Level::Info), action_type: None,
        category: IssueCategory::ChronicDecay,
    },
    // E: Convention Drift — test coverage gaps
    ClassifyRule {
        dimension: Some("quality"),
        message_any: Some(&["no test files", "no corresponding test", "% of files are tests", "test/source line ratio"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::ConventionDrift,
    },
    // C: Architectural Decision — Split/Refactor actions
    ClassifyRule {
        dimension: None, message_any: None, message_all: None,
        level: None, action_type: Some(ActionType::Split),
        category: IssueCategory::ArchitecturalDecision,
    },
    ClassifyRule {
        dimension: None, message_any: None, message_all: None,
        level: None, action_type: Some(ActionType::Refactor),
        category: IssueCategory::ArchitecturalDecision,
    },
    // C: structural dimension issues are typically architectural
    ClassifyRule {
        dimension: Some("structural"),
        message_any: None, message_all: None,
        level: None, action_type: None,
        category: IssueCategory::ArchitecturalDecision,
    },
    // C: performance nested loops
    ClassifyRule {
        dimension: Some("performance"),
        message_any: Some(&["nested loop"]),
        message_all: None, level: None, action_type: None,
        category: IssueCategory::ArchitecturalDecision,
    },
    // B: Extract actions suggest pattern problems
    ClassifyRule {
        dimension: None, message_any: None, message_all: None,
        level: None, action_type: Some(ActionType::Extract),
        category: IssueCategory::PatternProblem,
    },
];

/// Classify all issues in-place.
pub fn classify_issues(issues: &mut [Issue]) {
    for issue in issues.iter_mut() {
        issue.classification = Some(classify(issue));
    }
}

/// Determine the category for a single issue.
/// Iterates rules in priority order; returns the first match,
/// defaulting to `ArchitecturalDecision`.
fn classify(issue: &Issue) -> IssueCategory {
    let dim = issue.category.as_str();
    let msg = issue.message.to_lowercase();
    let action_type = issue.actions.first().map(|a| &a.action_type);

    for rule in RULES {
        if rule.matches(dim, &msg, issue.level, action_type) {
            return rule.category;
        }
    }

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
            details: vec![],
            impact: None,
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
