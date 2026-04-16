use std::collections::HashSet;
use std::fmt;

use super::math::*;

/// Direction of score change velocity.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum Direction {
    Improving,
    Declining,
    Stable,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Improving => write!(f, "↑"),
            Direction::Declining => write!(f, "↓"),
            Direction::Stable => write!(f, "→"),
        }
    }
}

/// Velocity of a dimension's score over time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Velocity {
    pub dimension: String,
    pub slope: f64,
    pub direction: Direction,
    pub data_points: usize,
}

fn direction_from_slope(slope: f64) -> Direction {
    if slope > 1.0 {
        Direction::Improving
    } else if slope < -1.0 {
        Direction::Declining
    } else {
        Direction::Stable
    }
}

/// Calculate velocity for all dimensions with ≥3 data points.
/// Results sorted by dimension name.
pub fn calculate_velocities(snapshots: &[crate::db::SnapshotScores]) -> Vec<Velocity> {
    let mut dims: HashSet<&str> = HashSet::new();
    for s in snapshots {
        for k in s.scores.keys() {
            dims.insert(k.as_str());
        }
    }

    let mut result: Vec<Velocity> = dims
        .into_iter()
        .filter_map(|dim| {
            let series = dimension_series(snapshots, dim);
            if series.len() < 3 {
                return None;
            }
            let slope = linear_regression_slope(&series)?;
            Some(Velocity {
                dimension: dim.to_string(),
                slope,
                direction: direction_from_slope(slope),
                data_points: series.len(),
            })
        })
        .collect();

    result.sort_by(|a, b| a.dimension.cmp(&b.dimension));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_from_slope() {
        assert_eq!(direction_from_slope(2.5), Direction::Improving);
        assert_eq!(direction_from_slope(-3.0), Direction::Declining);
        assert_eq!(direction_from_slope(0.5), Direction::Stable);
        assert_eq!(direction_from_slope(-0.5), Direction::Stable);
        assert_eq!(direction_from_slope(1.0), Direction::Stable); // boundary: ≤1.0 is stable
    }

    #[test]
    fn test_calculate_velocities() {
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
                scores: [("structural".into(), Some(85)), ("complexity".into(), Some(65))].into(),
            },
            SnapshotScores {
                snapshot_id: 3,
                created_at: "2026-01-03".into(),
                scores: [("structural".into(), Some(90)), ("complexity".into(), Some(60))].into(),
            },
        ];

        let vels = calculate_velocities(&snapshots);
        assert_eq!(vels.len(), 2);

        // Sorted alphabetically: complexity first
        assert_eq!(vels[0].dimension, "complexity");
        assert!((vels[0].slope - (-5.0)).abs() < 0.001);
        assert_eq!(vels[0].direction, Direction::Declining);
        assert_eq!(vels[0].data_points, 3);

        assert_eq!(vels[1].dimension, "structural");
        assert!((vels[1].slope - 5.0).abs() < 0.001);
        assert_eq!(vels[1].direction, Direction::Improving);
    }

    #[test]
    fn test_calculate_velocities_insufficient_data() {
        use crate::db::SnapshotScores;

        let snapshots = vec![
            SnapshotScores {
                snapshot_id: 1,
                created_at: "2026-01-01".into(),
                scores: [("structural".into(), Some(80))].into(),
            },
            SnapshotScores {
                snapshot_id: 2,
                created_at: "2026-01-02".into(),
                scores: [("structural".into(), Some(85))].into(),
            },
        ];

        let vels = calculate_velocities(&snapshots);
        assert!(vels.is_empty()); // < 3 data points
    }
}
