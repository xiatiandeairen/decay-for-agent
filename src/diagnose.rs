use std::fmt;

use crate::action::Action;

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

#[derive(serde::Serialize)]
pub struct Issue {
    pub level: Level,
    pub category: String,
    pub message: String,
    pub prescription: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

impl Issue {
    /// Create an issue without actions (default for dimensions not yet migrated to Action).
    pub fn new(level: Level, category: impl Into<String>, message: impl Into<String>, prescription: Option<String>) -> Self {
        Self {
            level,
            category: category.into(),
            message: message.into(),
            prescription,
            actions: vec![],
        }
    }

    /// Create an issue with structured actions.
    pub fn with_actions(level: Level, category: impl Into<String>, message: impl Into<String>, prescription: Option<String>, actions: Vec<Action>) -> Self {
        Self {
            level,
            category: category.into(),
            message: message.into(),
            prescription,
            actions,
        }
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  [{}] {}: {}", self.level, self.category, self.message)?;
        if let Some(rx) = &self.prescription {
            write!(f, " — {rx}")?;
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
