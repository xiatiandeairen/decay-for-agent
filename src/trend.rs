use std::fmt;

use crate::db::PreviousScores;

/// Direction and magnitude of score change.
#[derive(Clone, serde::Serialize)]
#[serde(into = "String")]
pub enum Delta {
    Up(i32),
    Down(i32),
    Unchanged,
    NA,
}

impl fmt::Display for Delta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Delta::Up(n) => write!(f, "↑{n}"),
            Delta::Down(n) => write!(f, "↓{n}"),
            Delta::Unchanged => write!(f, "→"),
            Delta::NA => write!(f, "N/A"),
        }
    }
}

impl From<Delta> for String {
    fn from(d: Delta) -> String {
        match d {
            Delta::Up(n) => format!("+{n}"),
            Delta::Down(n) => format!("-{n}"),
            Delta::Unchanged => "0".to_string(),
            Delta::NA => "N/A".to_string(),
        }
    }
}

/// Trend comparison across all dimensions.
#[derive(serde::Serialize)]
pub struct Trend {
    pub structural: Delta,
    pub complexity: Delta,
    pub fragility: Delta,
    pub composite: Delta,
}

impl Trend {
    /// Compare current scores against previous snapshot.
    pub fn compare(
        structural: i32,
        complexity: i32,
        fragility: Option<i32>,
        composite: i32,
        previous: &PreviousScores,
    ) -> Self {
        Trend {
            structural: delta(structural, previous.structural),
            complexity: delta(complexity, previous.complexity),
            fragility: match (fragility, previous.fragility) {
                (Some(curr), Some(prev)) => delta(curr, prev),
                _ => Delta::NA,
            },
            composite: delta(composite, previous.composite),
        }
    }
}

fn delta(current: i32, previous: i32) -> Delta {
    let diff = current - previous;
    match diff.cmp(&0) {
        std::cmp::Ordering::Greater => Delta::Up(diff),
        std::cmp::Ordering::Less => Delta::Down(-diff),
        std::cmp::Ordering::Equal => Delta::Unchanged,
    }
}

/// Format the health line with trend info.
pub fn format_health_with_trend(
    composite: i32,
    structural: i32,
    complexity: i32,
    fragility: Option<i32>,
    trend: &Trend,
) -> String {
    let f_display = match fragility {
        Some(v) => format!("{v}"),
        None => "N/A".to_string(),
    };
    format!(
        "Health: {composite}/100 ({}) structural: {structural} ({}) complexity: {complexity} ({}) fragility: {f_display} ({})",
        trend.composite, trend.structural, trend.complexity, trend.fragility
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_up() {
        assert!(matches!(delta(85, 80), Delta::Up(5)));
    }

    #[test]
    fn test_delta_down() {
        assert!(matches!(delta(70, 80), Delta::Down(10)));
    }

    #[test]
    fn test_delta_unchanged() {
        assert!(matches!(delta(80, 80), Delta::Unchanged));
    }

    #[test]
    fn test_compare_with_previous() {
        let prev = PreviousScores {
            structural: 80,
            complexity: 90,
            fragility: Some(70),
            composite: 80,
        };
        let trend = Trend::compare(85, 90, Some(60), 78, &prev);
        assert!(matches!(trend.structural, Delta::Up(5)));
        assert!(matches!(trend.complexity, Delta::Unchanged));
        assert!(matches!(trend.fragility, Delta::Down(10)));
        assert!(matches!(trend.composite, Delta::Down(2)));
    }

    #[test]
    fn test_compare_fragility_na() {
        let prev = PreviousScores {
            structural: 80,
            complexity: 90,
            fragility: None,
            composite: 85,
        };
        let trend = Trend::compare(80, 90, Some(70), 80, &prev);
        assert!(matches!(trend.fragility, Delta::NA));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Delta::Up(5)), "↑5");
        assert_eq!(format!("{}", Delta::Down(3)), "↓3");
        assert_eq!(format!("{}", Delta::Unchanged), "→");
        assert_eq!(format!("{}", Delta::NA), "N/A");
    }
}
