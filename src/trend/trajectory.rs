use super::correlation::{analyze_correlations, Correlation};
use super::forecast::{forecast_breaches, Forecast};
use super::regression::{detect_regressions, Regression};
use super::velocity::{calculate_velocities, Direction, Velocity};

/// Unified health trajectory aggregating all trend analysis.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Trajectory {
    pub overall_direction: Direction,
    pub snapshot_count: usize,
    pub velocities: Vec<Velocity>,
    pub regressions: Vec<Regression>,
    pub forecasts: Vec<Forecast>,
    pub correlations: Vec<Correlation>,
}

/// Build a unified trajectory from snapshot time series.
/// Aggregates velocity, regression, forecast, and correlation analyses.
pub fn build_trajectory(
    snapshots: &[crate::db::SnapshotScores],
    regression_k: f64,
    forecast_threshold: i32,
) -> Trajectory {
    let velocities = calculate_velocities(snapshots);
    let regressions = detect_regressions(snapshots, regression_k);
    let forecasts = forecast_breaches(snapshots, forecast_threshold);
    let correlations = analyze_correlations(snapshots);

    let overall_direction = velocities
        .iter()
        .find(|v| v.dimension == "composite")
        .map(|v| v.direction)
        .unwrap_or(Direction::Stable);

    Trajectory {
        overall_direction,
        snapshot_count: snapshots.len(),
        velocities,
        regressions,
        forecasts,
        correlations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_trajectory_full() {
        use crate::db::SnapshotScores;

        // 5 snapshots with declining composite
        let snapshots: Vec<SnapshotScores> = (0..5)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [
                    ("structural".into(), Some(90 - i as i32 * 3)),
                    ("composite".into(), Some(80 - i as i32 * 5)),
                ].into(),
            })
            .collect();

        let traj = build_trajectory(&snapshots, 2.0, 60);
        assert_eq!(traj.snapshot_count, 5);
        assert_eq!(traj.overall_direction, Direction::Declining);
        assert!(!traj.velocities.is_empty());
    }

    #[test]
    fn test_build_trajectory_empty() {
        let traj = build_trajectory(&[], 2.0, 60);
        assert_eq!(traj.snapshot_count, 0);
        assert_eq!(traj.overall_direction, Direction::Stable);
        assert!(traj.velocities.is_empty());
        assert!(traj.regressions.is_empty());
        assert!(traj.forecasts.is_empty());
        assert!(traj.correlations.is_empty());
    }

    #[test]
    fn test_build_trajectory_no_composite() {
        use crate::db::SnapshotScores;

        // 3 snapshots, no composite dimension
        let snapshots: Vec<SnapshotScores> = (0..3)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [("structural".into(), Some(80 + i as i32 * 2))].into(),
            })
            .collect();

        let traj = build_trajectory(&snapshots, 2.0, 60);
        assert_eq!(traj.overall_direction, Direction::Stable); // No composite → Stable
    }
}
