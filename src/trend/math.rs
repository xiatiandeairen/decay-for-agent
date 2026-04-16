/// Compute the slope of a least-squares linear regression.
/// Uses sequential index (0, 1, 2, ...) as x-axis, ignoring snapshot_id gaps.
/// Returns None if fewer than 2 data points.
pub(super) fn linear_regression_slope(points: &[(i64, i32)]) -> Option<f64> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let n_f = n as f64;
    let x_mean = (n_f - 1.0) / 2.0;
    let y_mean: f64 = points.iter().map(|(_, y)| *y as f64).sum::<f64>() / n_f;

    let mut num = 0.0;
    let mut den = 0.0;
    for (i, (_, y)) in points.iter().enumerate() {
        let dx = i as f64 - x_mean;
        let dy = *y as f64 - y_mean;
        num += dx * dy;
        den += dx * dx;
    }

    if den == 0.0 {
        return Some(0.0);
    }
    Some(num / den)
}

/// Population standard deviation.
pub(super) fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    variance.sqrt()
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
