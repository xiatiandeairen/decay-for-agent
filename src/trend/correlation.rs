use std::fmt;

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
    let mut dims: Vec<&str> = Vec::new();
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
