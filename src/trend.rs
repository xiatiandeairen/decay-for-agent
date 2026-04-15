use std::collections::{HashMap, HashSet};
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

/// Compute the slope of a least-squares linear regression.
/// Uses sequential index (0, 1, 2, ...) as x-axis, ignoring snapshot_id gaps.
/// Returns None if fewer than 2 data points.
fn linear_regression_slope(points: &[(i64, i32)]) -> Option<f64> {
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

/// Severity of a detected regression.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum RegressionSeverity {
    Moderate,
    Severe,
}

impl fmt::Display for RegressionSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegressionSeverity::Moderate => write!(f, "moderate"),
            RegressionSeverity::Severe => write!(f, "severe"),
        }
    }
}

/// A detected regression in a dimension's score.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Regression {
    pub dimension: String,
    pub previous_score: i32,
    pub current_score: i32,
    pub drop: i32,
    pub threshold: f64,
    pub severity: RegressionSeverity,
}

/// Population standard deviation.
fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    variance.sqrt()
}

/// Detect regressions by comparing the latest score drop against historical volatility.
/// `k` controls sensitivity: higher k = fewer detections (default 2.0).
/// Requires ≥3 data points per dimension.
pub fn detect_regressions(
    snapshots: &[crate::db::SnapshotScores],
    k: f64,
) -> Vec<Regression> {
    let mut dims: HashSet<&str> = HashSet::new();
    for s in snapshots {
        for key in s.scores.keys() {
            dims.insert(key.as_str());
        }
    }

    let mut result: Vec<Regression> = dims
        .into_iter()
        .filter_map(|dim| {
            let series = dimension_series(snapshots, dim);
            if series.len() < 3 {
                return None;
            }

            // Compute adjacent diffs
            let diffs: Vec<f64> = series
                .windows(2)
                .map(|w| (w[1].1 - w[0].1) as f64)
                .collect();

            let last_diff = *diffs.last()?;
            if last_diff >= 0.0 {
                return None; // No drop
            }

            let drop_abs = (-last_diff) as i32;
            // Use historical diffs (excluding the latest) as baseline
            let historical = &diffs[..diffs.len() - 1];
            let sigma = std_dev(historical);
            let threshold = k * sigma;

            // σ=0 means no historical variation; any drop is anomalous
            if sigma == 0.0 || last_diff.abs() > threshold {
                let severity = if sigma == 0.0 || last_diff.abs() > 2.0 * threshold {
                    RegressionSeverity::Severe
                } else {
                    RegressionSeverity::Moderate
                };
                let current = series.last()?.1;
                let previous = series[series.len() - 2].1;
                Some(Regression {
                    dimension: dim.to_string(),
                    previous_score: previous,
                    current_score: current,
                    drop: drop_abs,
                    threshold,
                    severity,
                })
            } else {
                None
            }
        })
        .collect();

    result.sort_by(|a, b| a.dimension.cmp(&b.dimension));
    result
}

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

/// Strength of correlation between two dimensions.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum CorrelationStrength {
    Strong,
    Moderate,
}

impl fmt::Display for CorrelationStrength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorrelationStrength::Strong => write!(f, "strong"),
            CorrelationStrength::Moderate => write!(f, "moderate"),
        }
    }
}

/// Correlation between two dimensions.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Correlation {
    pub dim_a: String,
    pub dim_b: String,
    pub coefficient: f64,
    pub strength: CorrelationStrength,
    pub data_points: usize,
}

/// Pearson correlation coefficient between two score sequences.
fn pearson_correlation(xs: &[i32], ys: &[i32]) -> Option<f64> {
    let n = xs.len();
    if n < 2 || n != ys.len() {
        return None;
    }
    let n_f = n as f64;
    let x_mean = xs.iter().map(|&x| x as f64).sum::<f64>() / n_f;
    let y_mean = ys.iter().map(|&y| y as f64).sum::<f64>() / n_f;

    let mut num = 0.0;
    let mut den_x = 0.0;
    let mut den_y = 0.0;
    for i in 0..n {
        let dx = xs[i] as f64 - x_mean;
        let dy = ys[i] as f64 - y_mean;
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }

    let den = (den_x * den_y).sqrt();
    if den == 0.0 {
        return Some(0.0);
    }
    Some(num / den)
}

/// Analyze correlations between all dimension pairs.
/// Returns pairs with |r| > 0.4 and ≥5 common data points, sorted by |r| descending.
pub fn analyze_correlations(
    snapshots: &[crate::db::SnapshotScores],
) -> Vec<Correlation> {
    // Collect all dimension names, sorted
    let mut dims: Vec<&str> = HashSet::<&str>::new()
        .into_iter()
        .collect();
    for s in snapshots {
        for k in s.scores.keys() {
            if !dims.contains(&k.as_str()) {
                dims.push(k.as_str());
            }
        }
    }
    dims.sort();

    let mut result = Vec::new();

    for i in 0..dims.len() {
        for j in (i + 1)..dims.len() {
            let a = dims[i];
            let b = dims[j];

            // Extract paired scores where both dimensions have values in the same snapshot
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for s in snapshots {
                let va = s.scores.get(a).and_then(|v| *v);
                let vb = s.scores.get(b).and_then(|v| *v);
                if let (Some(x), Some(y)) = (va, vb) {
                    xs.push(x);
                    ys.push(y);
                }
            }

            if xs.len() < 5 {
                continue;
            }

            let Some(r) = pearson_correlation(&xs, &ys) else { continue };
            let abs_r = r.abs();
            if abs_r <= 0.4 {
                continue;
            }

            let strength = if abs_r > 0.6 {
                CorrelationStrength::Strong
            } else {
                CorrelationStrength::Moderate
            };

            result.push(Correlation {
                dim_a: a.to_string(),
                dim_b: b.to_string(),
                coefficient: r,
                strength,
                data_points: xs.len(),
            });
        }
    }

    result.sort_by(|a, b| b.coefficient.abs().partial_cmp(&a.coefficient.abs()).unwrap_or(std::cmp::Ordering::Equal));
    result
}

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

    #[test]
    fn test_linear_regression_slope_ascending() {
        // Points: (1,10), (2,20), (3,30) → perfect slope of 10
        let points = vec![(1, 10), (2, 20), (3, 30)];
        let slope = linear_regression_slope(&points).unwrap();
        assert!((slope - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_linear_regression_slope_descending() {
        let points = vec![(1, 90), (2, 80), (3, 70)];
        let slope = linear_regression_slope(&points).unwrap();
        assert!((slope - (-10.0)).abs() < 0.001);
    }

    #[test]
    fn test_linear_regression_slope_flat() {
        let points = vec![(1, 50), (2, 50), (3, 50)];
        let slope = linear_regression_slope(&points).unwrap();
        assert!((slope).abs() < 0.001);
    }

    #[test]
    fn test_linear_regression_slope_insufficient() {
        assert!(linear_regression_slope(&[]).is_none());
        assert!(linear_regression_slope(&[(1, 10)]).is_none());
    }

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

    #[test]
    fn test_std_dev() {
        assert_eq!(std_dev(&[]), 0.0);
        assert_eq!(std_dev(&[5.0, 5.0, 5.0]), 0.0);
        // [2, 4, 6] → mean=4, variance=(4+0+4)/3=2.667, σ≈1.633
        let sd = std_dev(&[2.0, 4.0, 6.0]);
        assert!((sd - 1.633).abs() < 0.01);
    }

    #[test]
    fn test_detect_regressions_with_drop() {
        use crate::db::SnapshotScores;

        // Stable at 80, then sudden drop to 60
        let snapshots = vec![
            SnapshotScores {
                snapshot_id: 1, created_at: "2026-01-01".into(),
                scores: [("structural".into(), Some(80))].into(),
            },
            SnapshotScores {
                snapshot_id: 2, created_at: "2026-01-02".into(),
                scores: [("structural".into(), Some(80))].into(),
            },
            SnapshotScores {
                snapshot_id: 3, created_at: "2026-01-03".into(),
                scores: [("structural".into(), Some(80))].into(),
            },
            SnapshotScores {
                snapshot_id: 4, created_at: "2026-01-04".into(),
                scores: [("structural".into(), Some(60))].into(),
            },
        ];

        let regs = detect_regressions(&snapshots, 2.0);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].dimension, "structural");
        assert_eq!(regs[0].previous_score, 80);
        assert_eq!(regs[0].current_score, 60);
        assert_eq!(regs[0].drop, 20);
        assert_eq!(regs[0].severity, RegressionSeverity::Severe);
    }

    #[test]
    fn test_detect_regressions_no_drop() {
        use crate::db::SnapshotScores;

        // Scores improving — no regression
        let snapshots = vec![
            SnapshotScores {
                snapshot_id: 1, created_at: "2026-01-01".into(),
                scores: [("structural".into(), Some(70))].into(),
            },
            SnapshotScores {
                snapshot_id: 2, created_at: "2026-01-02".into(),
                scores: [("structural".into(), Some(75))].into(),
            },
            SnapshotScores {
                snapshot_id: 3, created_at: "2026-01-03".into(),
                scores: [("structural".into(), Some(80))].into(),
            },
        ];

        let regs = detect_regressions(&snapshots, 2.0);
        assert!(regs.is_empty());
    }

    #[test]
    fn test_detect_regressions_insufficient_data() {
        use crate::db::SnapshotScores;

        let snapshots = vec![
            SnapshotScores {
                snapshot_id: 1, created_at: "2026-01-01".into(),
                scores: [("structural".into(), Some(80))].into(),
            },
            SnapshotScores {
                snapshot_id: 2, created_at: "2026-01-02".into(),
                scores: [("structural".into(), Some(60))].into(),
            },
        ];

        let regs = detect_regressions(&snapshots, 2.0);
        assert!(regs.is_empty()); // < 3 data points
    }

    #[test]
    fn test_detect_regressions_moderate() {
        use crate::db::SnapshotScores;

        // Volatile history: diffs = [-5, +5, -5, +5, -15]
        // σ of diffs is high enough that -15 is moderate but not severe
        let snapshots = vec![
            SnapshotScores { snapshot_id: 1, created_at: "d1".into(), scores: [("s".into(), Some(80))].into() },
            SnapshotScores { snapshot_id: 2, created_at: "d2".into(), scores: [("s".into(), Some(75))].into() },
            SnapshotScores { snapshot_id: 3, created_at: "d3".into(), scores: [("s".into(), Some(80))].into() },
            SnapshotScores { snapshot_id: 4, created_at: "d4".into(), scores: [("s".into(), Some(75))].into() },
            SnapshotScores { snapshot_id: 5, created_at: "d5".into(), scores: [("s".into(), Some(80))].into() },
            SnapshotScores { snapshot_id: 6, created_at: "d6".into(), scores: [("s".into(), Some(65))].into() },
        ];

        let regs = detect_regressions(&snapshots, 2.0);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].severity, RegressionSeverity::Moderate);
    }

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

    #[test]
    fn test_pearson_perfect_positive() {
        let xs = vec![10, 20, 30, 40, 50];
        let ys = vec![10, 20, 30, 40, 50];
        let r = pearson_correlation(&xs, &ys).unwrap();
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_pearson_perfect_negative() {
        let xs = vec![10, 20, 30, 40, 50];
        let ys = vec![50, 40, 30, 20, 10];
        let r = pearson_correlation(&xs, &ys).unwrap();
        assert!((r - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_pearson_zero_variance() {
        let xs = vec![50, 50, 50, 50, 50];
        let ys = vec![10, 20, 30, 40, 50];
        let r = pearson_correlation(&xs, &ys).unwrap();
        assert_eq!(r, 0.0);
    }

    #[test]
    fn test_pearson_insufficient() {
        assert!(pearson_correlation(&[10], &[20]).is_none());
        assert!(pearson_correlation(&[], &[]).is_none());
    }

    #[test]
    fn test_analyze_correlations_strong() {
        use crate::db::SnapshotScores;

        // complexity and maintainability perfectly negatively correlated
        let snapshots: Vec<SnapshotScores> = (0..5)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [
                    ("complexity".into(), Some(60 + i as i32 * 5)),
                    ("maintainability".into(), Some(90 - i as i32 * 5)),
                ].into(),
            })
            .collect();

        let corrs = analyze_correlations(&snapshots);
        assert_eq!(corrs.len(), 1);
        assert_eq!(corrs[0].dim_a, "complexity");
        assert_eq!(corrs[0].dim_b, "maintainability");
        assert!((corrs[0].coefficient - (-1.0)).abs() < 0.001);
        assert_eq!(corrs[0].strength, CorrelationStrength::Strong);
        assert_eq!(corrs[0].data_points, 5);
    }

    #[test]
    fn test_analyze_correlations_weak_excluded() {
        use crate::db::SnapshotScores;

        // Uncorrelated data
        let scores_a = [80, 70, 85, 65, 90];
        let scores_b = [50, 80, 40, 90, 60];
        let snapshots: Vec<SnapshotScores> = (0..5)
            .map(|i| SnapshotScores {
                snapshot_id: i as i64 + 1,
                created_at: format!("d{}", i + 1),
                scores: [
                    ("a".into(), Some(scores_a[i])),
                    ("b".into(), Some(scores_b[i])),
                ].into(),
            })
            .collect();

        let corrs = analyze_correlations(&snapshots);
        // Weak correlation should be excluded (|r| ≤ 0.4)
        for c in &corrs {
            assert!(c.coefficient.abs() > 0.4);
        }
    }

    #[test]
    fn test_analyze_correlations_insufficient_data() {
        use crate::db::SnapshotScores;

        let snapshots: Vec<SnapshotScores> = (0..3)
            .map(|i| SnapshotScores {
                snapshot_id: i + 1,
                created_at: format!("d{}", i + 1),
                scores: [
                    ("a".into(), Some(80 + i as i32)),
                    ("b".into(), Some(70 - i as i32)),
                ].into(),
            })
            .collect();

        let corrs = analyze_correlations(&snapshots);
        assert!(corrs.is_empty()); // < 5 data points
    }

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
