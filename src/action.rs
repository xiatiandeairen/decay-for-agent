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

/// Where the action should be applied.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Target {
    /// File or directory path (always present).
    pub file: String,
    /// Line range within the file (M3: precision upgrade).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<(u32, u32)>,
    /// Function or module name (M3: precision upgrade).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// A structured, agent-consumable prescription.
///
/// Replaces free-text prescriptions with typed actions that include
/// what to do, where, why, how urgent, and how much effort.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Action {
    /// Source dimension that generated this action.
    pub dimension: String,
    /// What type of change is recommended.
    pub action_type: ActionType,
    /// Where to apply the change.
    pub target: Target,
    /// Why this action is needed.
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
            self.priority, self.action_type, self.target.file, self.reason
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_action() -> Action {
        Action {
            dimension: "structural".into(),
            action_type: ActionType::Split,
            target: Target {
                file: "src/".into(),
                line_range: None,
                symbol: None,
            },
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
        assert!(json.contains("\"file\": \"src/\""));
        // Optional fields should be absent when None
        assert!(!json.contains("line_range"));
        assert!(!json.contains("symbol"));
    }

    #[test]
    fn serialize_action_with_location() {
        let action = Action {
            target: Target {
                file: "src/main.rs".into(),
                line_range: Some((10, 50)),
                symbol: Some("handle_request".into()),
            },
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
    fn display_format() {
        let action = sample_action();
        let display = format!("{action}");
        assert_eq!(display, "[CRITICAL] SPLIT src/ — 1200 files exceed threshold");
    }

    #[test]
    fn effort_ordering() {
        assert!(Effort::Small < Effort::Medium);
        assert!(Effort::Medium < Effort::Large);
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
    fn action_type_display() {
        assert_eq!(format!("{}", ActionType::Split), "SPLIT");
        assert_eq!(format!("{}", ActionType::Extract), "EXTRACT");
        assert_eq!(format!("{}", ActionType::Add), "ADD");
        assert_eq!(format!("{}", ActionType::Remove), "REMOVE");
        assert_eq!(format!("{}", ActionType::Replace), "REPLACE");
        assert_eq!(format!("{}", ActionType::Move), "MOVE");
        assert_eq!(format!("{}", ActionType::Refactor), "REFACTOR");
    }
}
