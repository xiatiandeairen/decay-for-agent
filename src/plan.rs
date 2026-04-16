/// Improvement plan generator.
///
/// Organizes actions into phased plan based on effort and impact:
/// Phase 1: Quick Wins (small effort, high priority)
/// Phase 2: Pattern Fixes (medium effort, addresses root causes)
/// Phase 3: Structural (large effort, architectural improvements)

use serde::Serialize;

use crate::action::{Action, Effort, Priority};
use crate::aggregate::AggregatedIssue;

/// A phase within the improvement plan.
#[derive(Debug, Clone, Serialize)]
pub struct Phase {
    pub name: String,
    pub description: String,
    pub action_count: usize,
    pub top_actions: Vec<PlanAction>,
    pub estimated_effort: String,
}

/// Compact action for plan display.
#[derive(Debug, Clone, Serialize)]
pub struct PlanAction {
    pub what: String,
    pub file: String,
    pub effort: String,
}

/// The full improvement plan.
#[derive(Debug, Clone, Serialize)]
pub struct ImprovementPlan {
    pub phases: Vec<Phase>,
    pub total_actions: usize,
}

/// Generate an improvement plan from actions and aggregated issues.
pub fn generate_plan(actions: &[Action], aggregated: &[AggregatedIssue]) -> ImprovementPlan {
    // Phase 1: Quick Wins — Critical/High priority + Small effort
    let quick_wins: Vec<&Action> = actions
        .iter()
        .filter(|a| {
            (a.priority == Priority::Critical || a.priority == Priority::High)
                && a.effort == Effort::Small
        })
        .collect();

    // Phase 2: Pattern Fixes — Medium effort or aggregated root causes
    let pattern_fixes: Vec<&Action> = actions
        .iter()
        .filter(|a| a.effort == Effort::Medium && a.priority != Priority::Low)
        .take(5)
        .collect();

    // Phase 3: Structural — Large effort
    let structural: Vec<&Action> = actions
        .iter()
        .filter(|a| a.effort == Effort::Large)
        .take(3)
        .collect();

    let mut phases = Vec::new();

    if !quick_wins.is_empty() {
        phases.push(Phase {
            name: "Quick Wins".into(),
            description: "High-priority fixes with small effort. Start here for immediate impact.".into(),
            action_count: quick_wins.len(),
            top_actions: quick_wins.iter().take(5).map(|a| PlanAction {
                what: a.suggestion.clone(),
                file: a.target.file.clone(),
                effort: format!("{}", a.effort),
            }).collect(),
            estimated_effort: format!("~{} min", quick_wins.len() * 15),
        });
    }

    if !pattern_fixes.is_empty() || !aggregated.is_empty() {
        let mut desc_parts = Vec::new();
        if !aggregated.is_empty() {
            for agg in aggregated.iter().take(2) {
                desc_parts.push(format!("{} ({} files)", agg.root_cause, agg.affected_files.len()));
            }
        }
        let description = if desc_parts.is_empty() {
            "Medium-effort improvements addressing recurring patterns.".into()
        } else {
            format!("Address root causes: {}.", desc_parts.join("; "))
        };

        phases.push(Phase {
            name: "Pattern Fixes".into(),
            description,
            action_count: pattern_fixes.len(),
            top_actions: pattern_fixes.iter().take(5).map(|a| PlanAction {
                what: a.suggestion.clone(),
                file: a.target.file.clone(),
                effort: format!("{}", a.effort),
            }).collect(),
            estimated_effort: format!("~{} hr", (pattern_fixes.len() as f64 * 0.5).ceil() as usize),
        });
    }

    if !structural.is_empty() {
        phases.push(Phase {
            name: "Structural".into(),
            description: "Larger architectural improvements. Plan these across multiple sessions.".into(),
            action_count: structural.len(),
            top_actions: structural.iter().take(3).map(|a| PlanAction {
                what: a.suggestion.clone(),
                file: a.target.file.clone(),
                effort: format!("{}", a.effort),
            }).collect(),
            estimated_effort: format!("~{} hr", structural.len() * 2),
        });
    }

    let total_actions = actions.len();
    ImprovementPlan { phases, total_actions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionType, Target};

    fn make_action(priority: Priority, effort: Effort, suggestion: &str) -> Action {
        Action::new("test", ActionType::Replace, Target::file("test.rs"),
            suggestion, "reason", priority, effort)
    }

    #[test]
    fn test_plan_three_phases() {
        let actions = vec![
            make_action(Priority::Critical, Effort::Small, "fix SQL injection"),
            make_action(Priority::High, Effort::Small, "replace unwrap"),
            make_action(Priority::High, Effort::Medium, "extract module"),
            make_action(Priority::Medium, Effort::Large, "restructure project"),
        ];
        let plan = generate_plan(&actions, &[]);
        assert_eq!(plan.phases.len(), 3);
        assert_eq!(plan.phases[0].name, "Quick Wins");
        assert_eq!(plan.phases[0].action_count, 2);
        assert_eq!(plan.phases[1].name, "Pattern Fixes");
        assert_eq!(plan.phases[2].name, "Structural");
    }

    #[test]
    fn test_plan_empty() {
        let plan = generate_plan(&[], &[]);
        assert!(plan.phases.is_empty());
    }

    #[test]
    fn test_plan_quick_wins_only() {
        let actions = vec![
            make_action(Priority::Critical, Effort::Small, "fix it"),
        ];
        let plan = generate_plan(&actions, &[]);
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.phases[0].name, "Quick Wins");
    }
}
