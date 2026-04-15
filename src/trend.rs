use std::collections::HashMap;
use std::fmt;

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

fn delta(current: i32, previous: i32) -> Delta {
    let diff = current - previous;
    match diff.cmp(&0) {
        std::cmp::Ordering::Greater => Delta::Up(diff),
        std::cmp::Ordering::Less => Delta::Down(-diff),
        std::cmp::Ordering::Equal => Delta::Unchanged,
    }
}

/// Compare current dimension scores against previous snapshot scores.
pub fn compare_dimensions(
    current: &HashMap<String, Option<i32>>,
    previous: &HashMap<String, Option<i32>>,
) -> HashMap<String, Delta> {
    let mut result = HashMap::new();
    for (name, curr_score) in current {
        let prev_score = previous.get(name).copied().flatten();
        let d = match (*curr_score, prev_score) {
            (Some(c), Some(p)) => delta(c, p),
            _ => Delta::NA,
        };
        result.insert(name.clone(), d);
    }
    result
}

/// Extract a single dimension's score sequence from snapshot time series.
/// Returns (snapshot_id, score) pairs, skipping snapshots where the dimension is absent or None.
pub fn dimension_series(
    snapshots: &[crate::db::SnapshotScores],
    dimension: &str,
) -> Vec<(i64, i32)> {
    snapshots
        .iter()
        .filter_map(|s| {
            s.scores
                .get(dimension)
                .copied()
                .flatten()
                .map(|score| (s.snapshot_id, score))
        })
        .collect()
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
    fn test_compare_dimensions() {
        let mut current = HashMap::new();
        current.insert("structural".to_string(), Some(85));
        current.insert("complexity".to_string(), Some(90));
        current.insert("fragility".to_string(), Some(60));
        current.insert("composite".to_string(), Some(78));

        let mut previous = HashMap::new();
        previous.insert("structural".to_string(), Some(80));
        previous.insert("complexity".to_string(), Some(90));
        previous.insert("fragility".to_string(), Some(70));
        previous.insert("composite".to_string(), Some(80));

        let trend = compare_dimensions(&current, &previous);
        assert!(matches!(trend.get("structural").unwrap(), Delta::Up(5)));
        assert!(matches!(trend.get("complexity").unwrap(), Delta::Unchanged));
        assert!(matches!(trend.get("fragility").unwrap(), Delta::Down(10)));
        assert!(matches!(trend.get("composite").unwrap(), Delta::Down(2)));
    }

    #[test]
    fn test_compare_na() {
        let mut current = HashMap::new();
        current.insert("fragility".to_string(), Some(70));

        let mut previous = HashMap::new();
        previous.insert("fragility".to_string(), None);

        let trend = compare_dimensions(&current, &previous);
        assert!(matches!(trend.get("fragility").unwrap(), Delta::NA));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Delta::Up(5)), "↑5");
        assert_eq!(format!("{}", Delta::Down(3)), "↓3");
        assert_eq!(format!("{}", Delta::Unchanged), "→");
        assert_eq!(format!("{}", Delta::NA), "N/A");
    }

    #[test]
    fn test_dimension_series() {
        use crate::db::SnapshotScores;

        let snapshots = vec![
            SnapshotScores {
                snapshot_id: 1,
                created_at: "2026-01-01".into(),
                scores: [("structural".into(), Some(80)), ("complexity".into(), Some(70))].into(),
            },
            SnapshotScores {
                snapshot_id: 2,
                created_at: "2026-01-02".into(),
                scores: [("structural".into(), Some(75)), ("complexity".into(), None)].into(),
            },
            SnapshotScores {
                snapshot_id: 3,
                created_at: "2026-01-03".into(),
                scores: [("structural".into(), Some(82))].into(),
            },
        ];

        let series = dimension_series(&snapshots, "structural");
        assert_eq!(series, vec![(1, 80), (2, 75), (3, 82)]);

        let complexity = dimension_series(&snapshots, "complexity");
        assert_eq!(complexity, vec![(1, 70)]); // snapshot 2 has None, snapshot 3 missing

        let absent = dimension_series(&snapshots, "fragility");
        assert!(absent.is_empty());
    }
}
