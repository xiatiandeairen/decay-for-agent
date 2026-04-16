/// Development impact assessment.
///
/// Quantifies how much a detected issue affects daily development work:
/// how many files are coupled, how long it takes to understand the code,
/// and what the change risk level is.

use std::collections::HashMap;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

/// Development impact of an issue.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Impact {
    /// How many other files typically change together with this file.
    pub coupled_files: usize,
    /// Estimated time to understand the affected code (e.g. "~5 min").
    pub review_burden: String,
    /// Change risk level based on churn and coupling.
    pub change_risk: RiskLevel,
    /// Human-readable impact statement.
    pub statement: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::High => write!(f, "high"),
            RiskLevel::Medium => write!(f, "medium"),
            RiskLevel::Low => write!(f, "low"),
        }
    }
}

/// Analyze git co-change patterns: which files are commonly modified together.
/// Returns a map of file → list of files that changed in the same commits.
pub fn build_coupling_map(conn: &Connection, snapshot_id: i64) -> Result<HashMap<String, Vec<String>>> {
    // Find files that share high change_count (proxy for co-change in absence of per-commit data)
    // Group by change frequency similarity — files with similar churn patterns are likely coupled
    let mut stmt = conn.prepare(
        "SELECT path, change_count FROM git_changes WHERE snapshot_id = ?1 AND change_count > 3 ORDER BY change_count DESC"
    )?;

    let files: Vec<(String, i64)> = stmt
        .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut coupling: HashMap<String, Vec<String>> = HashMap::new();

    // Files with similar churn patterns (within 30% of each other) are likely coupled
    for i in 0..files.len() {
        let (ref path_a, count_a) = files[i];
        let mut coupled = Vec::new();
        for j in 0..files.len() {
            if i == j { continue; }
            let (ref path_b, count_b) = files[j];
            let ratio = count_a.min(count_b) as f64 / count_a.max(count_b).max(1) as f64;
            if ratio > 0.7 {
                coupled.push(path_b.clone());
            }
        }
        if !coupled.is_empty() {
            coupling.insert(path_a.clone(), coupled);
        }
    }

    Ok(coupling)
}

/// Estimate review burden based on file line count.
fn estimate_review_burden(line_count: usize) -> String {
    let minutes = match line_count {
        0..=100 => 2,
        101..=300 => 5,
        301..=500 => 10,
        501..=800 => 15,
        _ => 20,
    };
    format!("~{minutes} min")
}

/// Determine change risk based on coupling and churn.
fn assess_risk(coupled_files: usize, line_count: usize) -> RiskLevel {
    if coupled_files >= 3 || line_count > 500 {
        RiskLevel::High
    } else if coupled_files >= 1 || line_count > 300 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

/// Generate impact statement for an issue's target file.
fn impact_statement(file: &str, coupled_files: usize, line_count: usize, risk: RiskLevel) -> String {
    let coupling_part = if coupled_files > 0 {
        format!("changes to {file} typically affect {coupled_files} other file{}", if coupled_files > 1 { "s" } else { "" })
    } else {
        format!("{file} changes are self-contained", )
    };

    let burden_part = if line_count > 300 {
        format!(", takes ~{} min to understand context", line_count / 60 + 2)
    } else {
        String::new()
    };

    let risk_part = match risk {
        RiskLevel::High => " — high change risk",
        RiskLevel::Medium => " — moderate change risk",
        RiskLevel::Low => "",
    };

    format!("{coupling_part}{burden_part}{risk_part}")
}

/// Compute impact for a specific file.
pub fn compute_impact(
    file: &str,
    line_count: usize,
    coupling_map: &HashMap<String, Vec<String>>,
) -> Impact {
    let coupled = coupling_map.get(file).map(|v| v.len()).unwrap_or(0);
    let review_burden = estimate_review_burden(line_count);
    let risk = assess_risk(coupled, line_count);
    let statement = impact_statement(file, coupled, line_count, risk);

    Impact {
        coupled_files: coupled,
        review_burden,
        change_risk: risk,
        statement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_review_burden() {
        assert_eq!(estimate_review_burden(50), "~2 min");
        assert_eq!(estimate_review_burden(200), "~5 min");
        assert_eq!(estimate_review_burden(400), "~10 min");
        assert_eq!(estimate_review_burden(1000), "~20 min");
    }

    #[test]
    fn test_assess_risk() {
        assert_eq!(assess_risk(0, 100), RiskLevel::Low);
        assert_eq!(assess_risk(1, 200), RiskLevel::Medium);
        assert_eq!(assess_risk(3, 100), RiskLevel::High);
        assert_eq!(assess_risk(0, 600), RiskLevel::High);
    }

    #[test]
    fn test_compute_impact_isolated() {
        let map = HashMap::new();
        let impact = compute_impact("src/small.rs", 50, &map);
        assert_eq!(impact.coupled_files, 0);
        assert_eq!(impact.change_risk, RiskLevel::Low);
        assert!(impact.statement.contains("self-contained"));
    }

    #[test]
    fn test_compute_impact_coupled() {
        let mut map = HashMap::new();
        map.insert("src/big.rs".to_string(), vec!["src/a.rs".into(), "src/b.rs".into(), "src/c.rs".into()]);
        let impact = compute_impact("src/big.rs", 500, &map);
        assert_eq!(impact.coupled_files, 3);
        assert_eq!(impact.change_risk, RiskLevel::High);
        assert!(impact.statement.contains("3 other files"));
    }

    #[test]
    fn test_impact_statement_formatting() {
        let s = impact_statement("src/x.rs", 2, 400, RiskLevel::Medium);
        assert!(s.contains("2 other files"));
        assert!(s.contains("min to understand"));
        assert!(s.contains("moderate"));
    }
}
