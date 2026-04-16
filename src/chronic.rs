/// Chronic decay detection.
///
/// Identifies dimensions that haven't triggered issue thresholds yet
/// but are trending worse based on velocity data. Generates early
/// warnings so action can be taken before problems become acute.

use serde::Serialize;

use crate::trend::{Direction, Trajectory, Velocity};

/// A chronic decay warning for a dimension approaching trouble.
#[derive(Debug, Clone, Serialize)]
pub struct ChronicWarning {
    pub dimension: String,
    pub current_score: i32,
    pub velocity: f64,
    pub direction: Direction,
    pub warning: String,
}

/// Thresholds for chronic decay detection.
const HEALTHY_FLOOR: i32 = 70;  // scores above this are "healthy"
const APPROACHING_ZONE: i32 = 80; // scores in 70-80 with declining trend → warning

/// Detect chronic decay: dimensions with scores still acceptable but trending down.
///
/// Triggers when:
/// - Score is in the "approaching" zone (70-80) AND velocity is declining
/// - Or score > 80 but velocity is strongly declining (slope < -3.0)
pub fn detect_chronic_decay(
    scores: &std::collections::HashMap<String, Option<i32>>,
    trajectory: Option<&Trajectory>,
) -> Vec<ChronicWarning> {
    let Some(traj) = trajectory else {
        return Vec::new();
    };

    let vel_map: std::collections::HashMap<&str, &Velocity> = traj
        .velocities
        .iter()
        .map(|v| (v.dimension.as_str(), v))
        .collect();

    let mut warnings = Vec::new();

    for (dim, score_opt) in scores {
        let Some(score) = *score_opt else { continue };
        let Some(vel) = vel_map.get(dim.as_str()) else { continue };

        if vel.direction != Direction::Declining {
            continue;
        }

        let warning = if score > APPROACHING_ZONE && vel.slope < -3.0 {
            // High score but rapidly declining
            format!(
                "{dim} is healthy ({score}) but declining rapidly ({:.1}/snap) — monitor closely",
                vel.slope
            )
        } else if score > HEALTHY_FLOOR && score <= APPROACHING_ZONE {
            // In the approaching zone with any decline
            format!(
                "{dim} is approaching threshold ({score}, floor={HEALTHY_FLOOR}) with decline ({:.1}/snap)",
                vel.slope
            )
        } else {
            continue;
        };

        warnings.push(ChronicWarning {
            dimension: dim.clone(),
            current_score: score,
            velocity: vel.slope,
            direction: vel.direction,
            warning,
        });
    }

    warnings.sort_by_key(|w| w.current_score);
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_trajectory(velocities: Vec<Velocity>) -> Trajectory {
        Trajectory {
            overall_direction: Direction::Declining,
            snapshot_count: 5,
            velocities,
            regressions: vec![],
            forecasts: vec![],
            correlations: vec![],
        }
    }

    fn make_velocity(dim: &str, slope: f64) -> Velocity {
        let direction = if slope > 1.0 {
            Direction::Improving
        } else if slope < -1.0 {
            Direction::Declining
        } else {
            Direction::Stable
        };
        Velocity {
            dimension: dim.into(),
            slope,
            direction,
            data_points: 5,
        }
    }

    #[test]
    fn test_approaching_zone_with_decline() {
        let mut scores = HashMap::new();
        scores.insert("structural".into(), Some(75));

        let traj = make_trajectory(vec![make_velocity("structural", -2.0)]);
        let warnings = detect_chronic_decay(&scores, Some(&traj));

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].warning.contains("approaching threshold"));
    }

    #[test]
    fn test_high_score_rapid_decline() {
        let mut scores = HashMap::new();
        scores.insert("complexity".into(), Some(90));

        let traj = make_trajectory(vec![make_velocity("complexity", -5.0)]);
        let warnings = detect_chronic_decay(&scores, Some(&traj));

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].warning.contains("declining rapidly"));
    }

    #[test]
    fn test_no_warning_stable() {
        let mut scores = HashMap::new();
        scores.insert("structural".into(), Some(75));

        let traj = make_trajectory(vec![make_velocity("structural", 0.5)]);
        let warnings = detect_chronic_decay(&scores, Some(&traj));

        assert!(warnings.is_empty()); // stable, not declining
    }

    #[test]
    fn test_no_warning_already_low() {
        let mut scores = HashMap::new();
        scores.insert("structural".into(), Some(60));

        let traj = make_trajectory(vec![make_velocity("structural", -3.0)]);
        let warnings = detect_chronic_decay(&scores, Some(&traj));

        assert!(warnings.is_empty()); // already below floor, issues should catch this
    }

    #[test]
    fn test_no_trajectory() {
        let mut scores = HashMap::new();
        scores.insert("structural".into(), Some(75));

        let warnings = detect_chronic_decay(&scores, None);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_high_score_slow_decline_no_warning() {
        let mut scores = HashMap::new();
        scores.insert("complexity".into(), Some(90));

        let traj = make_trajectory(vec![make_velocity("complexity", -2.0)]);
        let warnings = detect_chronic_decay(&scores, Some(&traj));

        assert!(warnings.is_empty()); // slow decline at high score, not alarming yet
    }
}
