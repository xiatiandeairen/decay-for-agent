/// Agent-friendly summary generation.
///
/// Produces a layered summary: one-line overview + top N priority actions,
/// designed for efficient agent consumption without reading the full report.

use serde::Serialize;

use crate::action::Action;
use crate::diagnose::{Issue, Level};
use crate::trend::Trajectory;

/// Compact top-priority action for agent consumption.
#[derive(Debug, Clone, Serialize)]
pub struct TopAction {
    pub priority: String,
    pub what: String,
    pub effort: String,
}

/// Layered summary for agent consumption.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// One-line health overview.
    pub headline: String,
    /// Top 3 priority actions.
    pub top_actions: Vec<TopAction>,
    /// Issue counts by severity.
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// Generate a summary from issues, composite score, and trajectory.
pub fn generate_summary(
    composite: i32,
    issues: &[Issue],
    actions: &[Action],
    trajectory: Option<&Trajectory>,
) -> Summary {
    let critical_count = issues.iter().filter(|i| i.level == Level::Critical).count();
    let warning_count = issues.iter().filter(|i| i.level == Level::Warning).count();
    let info_count = issues.iter().filter(|i| i.level == Level::Info).count();

    // Build headline
    let direction = trajectory
        .map(|t| format!(", {}", t.overall_direction))
        .unwrap_or_default();
    let urgency = if critical_count > 0 {
        format!(". {} critical issue{} need immediate attention", critical_count, if critical_count > 1 { "s" } else { "" })
    } else if warning_count > 0 {
        format!(". {} warning{} to review", warning_count, if warning_count > 1 { "s" } else { "" })
    } else {
        ". No urgent issues".to_string()
    };
    let headline = format!("Health {composite}/100{direction}{urgency}");

    // Top 3 actions (already sorted by priority+effort in collect_sorted)
    let top_actions: Vec<TopAction> = actions
        .iter()
        .take(3)
        .map(|a| TopAction {
            priority: format!("{}", a.priority),
            what: if a.details.is_empty() {
                a.suggestion.clone()
            } else {
                format!("{} ({})", a.suggestion, a.details.first().unwrap_or(&String::new()))
            },
            effort: format!("{}", a.effort),
        })
        .collect();

    Summary {
        headline,
        top_actions,
        critical_count,
        warning_count,
        info_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionType, Effort, Priority, Target};

    #[test]
    fn test_summary_with_criticals() {
        let issues = vec![
            Issue {
                level: Level::Critical, category: "reliability".into(),
                message: "SQL injection".into(), classification: None, actions: vec![],
            },
            Issue {
                level: Level::Warning, category: "maintainability".into(),
                message: "long file".into(), classification: None, actions: vec![],
            },
        ];
        let actions = vec![Action::new(
            "reliability", ActionType::Replace, Target::file("db.rs"),
            "fix SQL injection", "SQL found", Priority::Critical, Effort::Small,
        )];

        let s = generate_summary(85, &issues, &actions, None);
        assert!(s.headline.contains("85/100"));
        assert!(s.headline.contains("1 critical issue"));
        assert_eq!(s.top_actions.len(), 1);
        assert_eq!(s.critical_count, 1);
    }

    #[test]
    fn test_summary_no_issues() {
        let s = generate_summary(95, &[], &[], None);
        assert!(s.headline.contains("No urgent issues"));
        assert!(s.top_actions.is_empty());
    }

    #[test]
    fn test_summary_with_trajectory() {
        use crate::trend::{Direction, Trajectory};

        let traj = Trajectory {
            overall_direction: Direction::Declining,
            snapshot_count: 5,
            velocities: vec![], regressions: vec![],
            forecasts: vec![], correlations: vec![],
        };
        let s = generate_summary(80, &[], &[], Some(&traj));
        assert!(s.headline.contains("↓"));
    }
}
