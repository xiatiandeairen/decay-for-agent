use std::fmt;

use crate::action::Action;

/// Problem classification category (A-H).
///
/// Each category implies a different handling strategy:
/// - A: pattern-match → generate patch
/// - B: aggregate similar → unified solution
/// - C: analyze trade-offs → output options
/// - D: precise fix → mandatory review
/// - E: detect inconsistency → alignment plan
/// - F: trajectory-based → preventive warning
/// - G: context-aware → suppress or downgrade
/// - H: recommend toolchain/CI config
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    /// Mechanical fix — clear pattern, direct fix, no judgment needed.
    MechanicalFix,
    /// Pattern problem — multiple similar issues sharing a root cause.
    PatternProblem,
    /// Architectural decision — needs understanding of project context.
    ArchitecturalDecision,
    /// Security critical — high risk, precise fix, mandatory review.
    SecurityCritical,
    /// Convention drift — inconsistency within the project.
    ConventionDrift,
    /// Chronic decay — not yet alarming but trending worse.
    ChronicDecay,
    /// Contextual exception — may be legitimate in certain contexts.
    ContextualException,
    /// Prevention — recommend toolchain/CI config to prevent recurrence.
    Prevention,
}

impl fmt::Display for IssueCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueCategory::MechanicalFix => write!(f, "A:mechanical"),
            IssueCategory::PatternProblem => write!(f, "B:pattern"),
            IssueCategory::ArchitecturalDecision => write!(f, "C:architectural"),
            IssueCategory::SecurityCritical => write!(f, "D:security"),
            IssueCategory::ConventionDrift => write!(f, "E:convention"),
            IssueCategory::ChronicDecay => write!(f, "F:chronic"),
            IssueCategory::ContextualException => write!(f, "G:contextual"),
            IssueCategory::Prevention => write!(f, "H:prevention"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Critical,
    Warning,
    Info,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Critical => write!(f, "CRITICAL"),
            Level::Warning => write!(f, "WARNING"),
            Level::Info => write!(f, "INFO"),
        }
    }
}

/// A diagnostic issue — what's wrong.
///
/// Pure diagnostic: level + category + message.
/// Prescriptive actions (what to do) live in `actions`.
#[derive(serde::Serialize)]
pub struct Issue {
    pub level: Level,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<IssueCategory>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

impl Issue {
    /// Create a diagnostic issue without actions.
    pub fn new(level: Level, category: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            category: category.into(),
            message: message.into(),
            classification: None,
            actions: vec![],
        }
    }

    /// Create a diagnostic issue with structured actions.
    pub fn with_actions(level: Level, category: impl Into<String>, message: impl Into<String>, actions: Vec<Action>) -> Self {
        Self {
            level,
            category: category.into(),
            message: message.into(),
            classification: None,
            actions,
        }
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class_tag = self.classification
            .map(|c| format!(" [{c}]"))
            .unwrap_or_default();
        write!(f, "  [{}]{class_tag} {}: {}", self.level, self.category, self.message)?;
        if let Some(action) = self.actions.first() {
            write!(f, " — {}", action.suggestion)?;
        }
        Ok(())
    }
}

/// Format and print the issues list.
pub fn print_issues(issues: &[Issue]) {
    if issues.is_empty() {
        println!("No issues found.");
        return;
    }

    let critical = issues.iter().filter(|i| i.level == Level::Critical).count();
    let warning = issues.iter().filter(|i| i.level == Level::Warning).count();
    let info = issues.iter().filter(|i| i.level == Level::Info).count();

    let mut parts = Vec::new();
    if critical > 0 {
        parts.push(format!("{critical} critical"));
    }
    if warning > 0 {
        parts.push(format!("{warning} warning"));
    }
    if info > 0 {
        parts.push(format!("{info} info"));
    }

    println!("Issues ({}):", parts.join(", "));
    for issue in issues {
        println!("{issue}");
    }
}
