use std::collections::HashSet;

use super::math::*;

/// A forecast predicting when a dimension will breach a health threshold.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Forecast {
    pub dimension: String,
    pub current_score: i32,
    pub slope: f64,
    pub r_squared: f64,
    pub threshold: i32,
    pub snapshots_until_breach: u32,
}

/// Coefficient of determination (R²) for a linear fit.
/// Returns None if fewer than 2 data points.
fn r_squared(points: &[(i64, i32)]) -> Option<f64> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let slope = linear_regression_slope(points)?;
    let n_f = n as f64;
    let y_mean: f64 = points.iter().map(|(_, y)| *y as f64).sum::<f64>() / n_f;

    let ss_tot: f64 = points.iter().map(|(_, y)| (*y as f64 - y_mean).powi(2)).sum();
    if ss_tot == 0.0 {
        return Some(1.0); // All values identical → perfect fit
    }

    // intercept = y_mean - slope * x_mean
    let x_mean = (n_f - 1.0) / 2.0;
    let intercept = y_mean - slope * x_mean;

    let ss_res: f64 = points
        .iter()
        .enumerate()
        .map(|(i, (_, y))| {
            let predicted = intercept + slope * i as f64;
            (*y as f64 - predicted).powi(2)
        })
        .sum();

    Some(1.0 - ss_res / ss_tot)
}

/// Predict which dimensions will breach a health threshold based on linear trend.
/// Only forecasts for dimensions with ≥5 data points, R² > 0.7, and negative slope.
/// Results sorted by snapshots_until_breach ascending (most urgent first).
pub fn forecast_breaches(
    snapshots: &[crate::db::SnapshotScores],
    threshold: i32,
) -> Vec<Forecast> {
    let mut dims: HashSet<&str> = HashSet::new();
    for s in snapshots {
        for key in s.scores.keys() {
            dims.insert(key.as_str());
        }
    }

    let mut result: Vec<Forecast> = dims
        .into_iter()
        .filter_map(|dim| {
            let series = dimension_series(snapshots, dim);
            if series.len() < 5 {
                return None;
            }
            let slope = linear_regression_slope(&series)?;
            if slope >= 0.0 {
                return None; // Not declining
            }
            let r2 = r_squared(&series)?;
            if r2 <= 0.7 {
                return None; // Trend not reliable
            }
            let current = series.last()?.1;
            if current <= threshold {
                return None; // Already breached
            }
            let steps = ((current - threshold) as f64 / slope.abs()).ceil() as u32;
            Some(Forecast {
                dimension: dim.to_string(),
                current_score: current,
                slope,
                r_squared: r2,
                threshold,
                snapshots_until_breach: steps,
            })
        })
        .collect();

    result.sort_by_key(|f| f.snapshots_until_breach);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r_squared_perfect_fit() {
        // Perfect linear: (1,10), (2,20), (3,30)
        let points = vec![(1, 10), (2, 20), (3, 30)];
        let r2 = r_squared(&points).unwrap();
        assert!((r2 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_r_squared_constant() {
        let points = vec![(1, 50), (2, 50), (3, 50)];
        let r2 = r_squared(&points).unwrap();
        assert_eq!(r2, 1.0);
    }

    #[test]
    fn test_r_squared_insufficient() {
        assert!(r_squared(&[]).is_none());
        assert!(r_squared(&[(1, 10)]).is_none());
    }

    #[test]
    fn test_forecast_declining_trend() {
        use crate::db::SnapshotScores;

        // Steady decline: 90, 85, 80, 75, 70 → slope=-5, R²=1.0
        let snapshots: Vec<SnapshotScores> = (0..5)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [("structural".into(), Some(90 - i as i32 * 5))].into(),
            })
            .collect();

        let forecasts = forecast_breaches(&snapshots, 60);
        assert_eq!(forecasts.len(), 1);
        assert_eq!(forecasts[0].dimension, "structural");
        assert_eq!(forecasts[0].current_score, 70);
        assert_eq!(forecasts[0].snapshots_until_breach, 2); // (70-60)/5 = 2
        assert!((forecasts[0].r_squared - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_forecast_low_r_squared() {
        use crate::db::SnapshotScores;

        // Noisy data: no clear trend → R² low → no forecast
        let scores_vals = [80, 65, 85, 60, 78];
        let snapshots: Vec<SnapshotScores> = scores_vals
            .iter()
            .enumerate()
            .map(|(i, &s)| SnapshotScores {
                snapshot_id: i as i64 + 1,
                created_at: format!("d{}", i + 1),
                scores: [("structural".into(), Some(s))].into(),
            })
            .collect();

        let forecasts = forecast_breaches(&snapshots, 60);
        assert!(forecasts.is_empty());
    }

    #[test]
    fn test_forecast_positive_slope() {
        use crate::db::SnapshotScores;

        // Improving: 60, 65, 70, 75, 80 → no forecast
        let snapshots: Vec<SnapshotScores> = (0..5)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [("structural".into(), Some(60 + i as i32 * 5))].into(),
            })
            .collect();

        let forecasts = forecast_breaches(&snapshots, 60);
        assert!(forecasts.is_empty());
    }

    #[test]
    fn test_forecast_already_breached() {
        use crate::db::SnapshotScores;

        // Already below threshold: 55, 50, 45, 40, 35
        let snapshots: Vec<SnapshotScores> = (0..5)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [("structural".into(), Some(55 - i as i32 * 5))].into(),
            })
            .collect();

        let forecasts = forecast_breaches(&snapshots, 60);
        assert!(forecasts.is_empty());
    }

    #[test]
    fn test_forecast_insufficient_data() {
        use crate::db::SnapshotScores;

        // Only 3 snapshots — below minimum of 5
        let snapshots: Vec<SnapshotScores> = (0..3)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [("structural".into(), Some(90 - i as i32 * 5))].into(),
            })
            .collect();

        let forecasts = forecast_breaches(&snapshots, 60);
        assert!(forecasts.is_empty());
    }
}
