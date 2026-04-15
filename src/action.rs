use serde::Serialize;
use std::fmt;

/// Type of change an action recommends.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Split an oversized file, directory, or function into smaller units.
    Split,
    /// Extract a module, interface, or shared logic into a separate unit.
    Extract,
    /// Add missing tests, logging, error handling, or documentation.
    Add,
    /// Remove dead code, unused dependencies, or obsolete files.
    Remove,
    /// Replace unsafe patterns, panics, or hardcoded values with safe alternatives.
    Replace,
    /// Move a file or module to a more appropriate location.
    Move,
    /// General refactoring (reduce coupling, optimize loops, etc.).
    Refactor,
}

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionType::Split => write!(f, "SPLIT"),
            ActionType::Extract => write!(f, "EXTRACT"),
            ActionType::Add => write!(f, "ADD"),
            ActionType::Remove => write!(f, "REMOVE"),
            ActionType::Replace => write!(f, "REPLACE"),
            ActionType::Move => write!(f, "MOVE"),
            ActionType::Refactor => write!(f, "REFACTOR"),
        }
    }
}

/// How urgently the action should be addressed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Critical => write!(f, "CRITICAL"),
            Priority::High => write!(f, "HIGH"),
            Priority::Medium => write!(f, "MEDIUM"),
            Priority::Low => write!(f, "LOW"),
        }
    }
}

/// Estimated effort to implement the action.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    /// < 30 min, typically 1 file.
    Small,
    /// 30 min – 2 hr, 2–5 files.
    Medium,
    /// > 2 hr, 5+ files or cross-module.
    Large,
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effort::Small => write!(f, "S"),
            Effort::Medium => write!(f, "M"),
            Effort::Large => write!(f, "L"),
        }
    }
}

/// Where the action should be applied.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Target {
    /// File or directory path (always present).
    pub file: String,
    /// Line range within the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<(u32, u32)>,
    /// Function or module name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

impl Target {
    /// Target a file or directory without line-level precision.
    pub fn file(path: impl Into<String>) -> Self {
        Self { file: path.into(), line_range: None, symbol: None }
    }

    /// Target a specific location within a file.
    pub fn at(path: impl Into<String>, line_range: (u32, u32), symbol: Option<String>) -> Self {
        Self { file: path.into(), line_range: Some(line_range), symbol }
    }
}

/// A structured, agent-consumable prescription.
///
/// Each action describes what to do (`suggestion`), why (`reason`),
/// where (`target`), how urgent (`priority`), and how much work (`effort`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Action {
    /// Source dimension that generated this action.
    pub dimension: String,
    /// What type of change is recommended.
    pub action_type: ActionType,
    /// Where to apply the change.
    pub target: Target,
    /// Human-friendly instruction (e.g. "split into sub-modules").
    pub suggestion: String,
    /// Why this action is needed (e.g. "1200 files exceed threshold").
    pub reason: String,
    /// How urgently this should be addressed.
    pub priority: Priority,
    /// Estimated implementation effort.
    pub effort: Effort,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} {} — {}",
            self.priority, self.action_type, self.target.file, self.suggestion
        )
    }
}

/// Collect all actions from issues, sort by priority+effort, then dedup.
///
/// Sort first guarantees same-type actions are adjacent for dedup.
/// Order: priority asc (Critical first) → effort asc (Small first).
pub fn collect_sorted(issues: &[crate::diagnose::Issue]) -> Vec<Action> {
    let mut actions: Vec<Action> = issues
        .iter()
        .flat_map(|i| i.actions.iter().cloned())
        .collect();
    actions.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then(a.effort.cmp(&b.effort))
            .then(a.dimension.cmp(&b.dimension))
            .then(a.target.file.cmp(&b.target.file))
    });
    actions.dedup_by(|b, a| {
        a.dimension == b.dimension
            && a.target.file == b.target.file
            && a.action_type == b.action_type
    });
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_action() -> Action {
        Action {
            dimension: "structural".into(),
            action_type: ActionType::Split,
            target: Target::file("src/"),
            suggestion: "split into sub-modules".into(),
            reason: "1200 files exceed threshold".into(),
            priority: Priority::Critical,
            effort: Effort::Large,
        }
    }

    #[test]
    fn serialize_action_json() {
        let action = sample_action();
        let json = serde_json::to_string_pretty(&action).unwrap();
        assert!(json.contains("\"action_type\": \"split\""));
        assert!(json.contains("\"priority\": \"critical\""));
        assert!(json.contains("\"effort\": \"large\""));
        assert!(json.contains("\"suggestion\": \"split into sub-modules\""));
        assert!(json.contains("\"file\": \"src/\""));
        assert!(!json.contains("line_range"));
        assert!(!json.contains("symbol"));
    }

    #[test]
    fn serialize_action_with_location() {
        let action = Action {
            target: Target::at("src/main.rs", (10, 50), Some("handle_request".into())),
            ..sample_action()
        };
        let json = serde_json::to_string_pretty(&action).unwrap();
        assert!(json.contains("line_range"));
        assert!(json.contains("symbol"));
        assert!(json.contains("handle_request"));
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Medium);
        assert!(Priority::Medium < Priority::Low);
    }

    #[test]
    fn effort_ordering() {
        assert!(Effort::Small < Effort::Medium);
        assert!(Effort::Medium < Effort::Large);
    }

    #[test]
    fn effort_display() {
        assert_eq!(format!("{}", Effort::Small), "S");
        assert_eq!(format!("{}", Effort::Medium), "M");
        assert_eq!(format!("{}", Effort::Large), "L");
    }

    #[test]
    fn display_format() {
        let action = sample_action();
        let display = format!("{action}");
        assert_eq!(display, "[CRITICAL] SPLIT src/ — split into sub-modules");
    }

    #[test]
    fn sort_priority_then_effort() {
        let make = |p: Priority, e: Effort| Action {
            priority: p,
            effort: e,
            ..sample_action()
        };
        let mut actions = vec![
            make(Priority::Medium, Effort::Large),
            make(Priority::Critical, Effort::Medium),
            make(Priority::Critical, Effort::Small),
            make(Priority::High, Effort::Small),
        ];
        actions.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.effort.cmp(&b.effort)));
        assert_eq!(actions[0].priority, Priority::Critical);
        assert_eq!(actions[0].effort, Effort::Small);
        assert_eq!(actions[1].priority, Priority::Critical);
        assert_eq!(actions[1].effort, Effort::Medium);
        assert_eq!(actions[2].priority, Priority::High);
        assert_eq!(actions[3].priority, Priority::Medium);
    }

    #[test]
    fn collect_sorted_dedup() {
        use crate::diagnose::Issue;

        let issues = vec![
            Issue::with_actions(
                crate::diagnose::Level::Warning,
                "complexity",
                "file A big",
                vec![Action {
                    dimension: "complexity".into(),
                    action_type: ActionType::Split,
                    target: Target::file("src/a.rs"),
                    suggestion: "split A".into(),
                    reason: "A is big".into(),
                    priority: Priority::High,
                    effort: Effort::Medium,
                }],
            ),
            Issue::with_actions(
                crate::diagnose::Level::Critical,
                "maintainability",
                "file A long",
                vec![Action {
                    dimension: "complexity".into(),
                    action_type: ActionType::Split,
                    target: Target::file("src/a.rs"),
                    suggestion: "split A".into(),
                    reason: "A is long".into(),
                    priority: Priority::Critical,
                    effort: Effort::Medium,
                }],
            ),
        ];

        let sorted = collect_sorted(&issues);
        // Should dedup: same dimension + file + action_type
        assert_eq!(sorted.len(), 1);
        // Should keep the Critical one (sorted first)
        assert_eq!(sorted[0].priority, Priority::Critical);
    }

    #[test]
    fn action_type_display() {
        assert_eq!(format!("{}", ActionType::Split), "SPLIT");
        assert_eq!(format!("{}", ActionType::Extract), "EXTRACT");
        assert_eq!(format!("{}", ActionType::Add), "ADD");
        assert_eq!(format!("{}", ActionType::Remove), "REMOVE");
        assert_eq!(format!("{}", ActionType::Replace), "REPLACE");
        assert_eq!(format!("{}", ActionType::Move), "MOVE");
        assert_eq!(format!("{}", ActionType::Refactor), "REFACTOR");
    }

    #[test]
    fn target_constructors() {
        let t1 = Target::file("src/main.rs");
        assert_eq!(t1.file, "src/main.rs");
        assert!(t1.line_range.is_none());
        assert!(t1.symbol.is_none());

        let t2 = Target::at("src/lib.rs", (10, 50), Some("foo".into()));
        assert_eq!(t2.line_range, Some((10, 50)));
        assert_eq!(t2.symbol.as_deref(), Some("foo"));
    }
}
