use std::path::Path;

use anyhow::Result;
use log::debug;

use super::helpers;
use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
/// Minimum ratio of test files to total files (source + test) before penalizing.
/// Below 10% means tests are rare outliers rather than a first-class practice.
const TEST_FILE_RATIO_WARN: f64 = 0.10;
/// Healthy test file ratio floor. Projects above 20% treat testing as a standard habit.
/// Below this, meaningful coverage gaps are almost guaranteed.
const TEST_FILE_RATIO_GOOD: f64 = 0.20;
/// Ratio of test lines to source lines. Below 10% indicates very thin test suites.
/// A ratio under 0.1 typically means only happy-path cases are covered.
const TEST_LINE_RATIO_WARN: f64 = 0.10;
/// Moderate test line ratio. 30% test-to-source lines is a reasonable baseline for safety.
const TEST_LINE_RATIO_GOOD: f64 = 0.30;
/// Assertions per 20 test lines. Below 1 assertion per 20 lines means tests assert little.
/// Tests without assertions may pass trivially and provide no real confidence.
const ASSERT_DENSITY_MIN: f64 = 1.0;

pub struct QualityAssurance;

impl Dimension for QualityAssurance {
    fn name(&self) -> &'static str {
        "quality_assurance"
    }

    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult> {
        let source_files = store.source_files();
        let name = self.name().to_string();

        if source_files.is_empty() {
            return Ok(DimensionResult { name, score: Some(100), issues: vec![] });
        }

        let analysis = analyze(source_files);
        let mut score: i32 = 100;
        let mut issues = Vec::new();
        debug!("quality_assurance: {} source, {} test files", analysis.source_files, analysis.test_files);

        // Test file ratio
        let total = analysis.source_files + analysis.test_files;
        if total > 0 {
            let test_ratio = analysis.test_files as f64 / total as f64;
            if test_ratio == 0.0 {
                score -= 40;
                issues.push(Issue::with_actions(
                    Level::Critical, name.clone(), "no test files found in project",
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Add,
                        target: Target::file("."),
                        suggestion: "add tests for critical paths and public APIs".into(),
                        reason: "no test files found".into(),
                        priority: Priority::Critical, effort: Effort::Large,
                        details: vec![],
                        impact: None,
                        verify: String::new(),
                    }],
                ));
            } else if test_ratio < TEST_FILE_RATIO_WARN {
                score -= 25;
                let pct = (test_ratio * 100.0) as i32;
                issues.push(Issue::with_actions(
                    Level::Warning, name.clone(), format!("only {pct}% of files are tests"),
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Add,
                        target: Target::file("."),
                        suggestion: "increase test coverage, focus on complex and critical modules".into(),
                        reason: format!("only {pct}% test files"),
                        priority: Priority::High, effort: Effort::Large,
                        details: vec![],
                        impact: None,
                        verify: String::new(),
                    }],
                ));
            } else if test_ratio < TEST_FILE_RATIO_GOOD {
                score -= 10;
            }
        }

        // Test/source line ratio
        if analysis.source_lines > 0 {
            let line_ratio = analysis.test_lines as f64 / analysis.source_lines as f64;
            if line_ratio < TEST_LINE_RATIO_WARN {
                score -= 20;
                issues.push(Issue::with_actions(
                    Level::Warning, name.clone(),
                    format!("test/source line ratio is {:.1}% (very low)", line_ratio * 100.0),
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Add,
                        target: Target::file("."),
                        suggestion: "add more tests to improve confidence in changes".into(),
                        reason: format!("test/source ratio {:.1}%", line_ratio * 100.0),
                        priority: Priority::High, effort: Effort::Medium,
                        details: vec![],
                        impact: None,
                        verify: String::new(),
                    }],
                ));
            } else if line_ratio < TEST_LINE_RATIO_GOOD {
                score -= 10;
            }
        }

        // Assertion density in test files
        if analysis.test_lines > 0 {
            let assert_per_20_lines = analysis.assert_count as f64 / (analysis.test_lines as f64 / 20.0);
            if assert_per_20_lines < ASSERT_DENSITY_MIN {
                score -= 10;
            }
        }

        // Report source files without corresponding test files
        for path in &analysis.untested_source_files {
            issues.push(Issue::new(
                Level::Info, name.clone(), format!("{path} has no corresponding test file"),
            ));
        }

        Ok(DimensionResult {
            name: self.name().to_string(),
            score: Some(score.max(0)),
            issues,
        })
    }
}

struct Analysis {
    source_files: usize,
    test_files: usize,
    source_lines: usize,
    test_lines: usize,
    assert_count: usize,
    untested_source_files: Vec<String>,
}

fn analyze(files: &[SourceFile]) -> Analysis {
    let mut source_files = 0;
    let mut test_files = 0;
    let mut source_lines = 0;
    let mut test_lines = 0;
    let mut assert_count = 0;
    let mut source_names: Vec<String> = Vec::new();
    let mut test_names: Vec<String> = Vec::new();

    let assert_patterns: &[&str] = &["assert", "expect(", "should.", ".toBe(", ".toEqual(",
        ".to_equal(", "assert_eq!", "assert_ne!", "assert!(", "#[test]",
        "@test", "@Test", "def test_"];

    for sf in files {
        if is_test_file(&sf.path) {
            test_files += 1;
            test_lines += sf.line_count;
            test_names.push(sf.path.clone());

            // Count lines with assertions (deduplicate per line)
            let hits = helpers::count_pattern_matches(&sf.lines, assert_patterns);
            let unique_lines: std::collections::HashSet<u32> = hits.iter().map(|h| h.line_no).collect();
            assert_count += unique_lines.len();
        } else {
            source_files += 1;
            source_names.push(sf.path.clone());

            let (src_lines, tst_lines) = split_inline_test_lines(&sf.lines);
            source_lines += src_lines;
            if tst_lines > 0 {
                test_files += 1;
                test_lines += tst_lines;
                test_names.push(sf.path.clone());

                // Count assertions only in the test section
                let test_start = sf.line_count - tst_lines;
                let test_section: Vec<String> = sf.lines.iter().skip(test_start).cloned().collect();
                let hits = helpers::count_pattern_matches(&test_section, assert_patterns);
                let unique_lines: std::collections::HashSet<u32> = hits.iter().map(|h| h.line_no).collect();
                assert_count += unique_lines.len();
            }
        }
    }

    // Find untested source files (no matching test file)
    let untested: Vec<String> = source_names
        .iter()
        .filter(|src| {
            let stem = Path::new(src)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            !test_names.iter().any(|t| {
                t.contains(&format!("test_{stem}"))
                    || t.contains(&format!("{stem}_test"))
                    || t.contains(&format!("{stem}_spec"))
                    || t.contains(&format!("{stem}.test"))
            })
        })
        .take(10) // Limit to top 10
        .cloned()
        .collect();

    Analysis {
        source_files,
        test_files,
        source_lines,
        test_lines,
        assert_count,
        untested_source_files: untested,
    }
}

fn is_test_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("/test/") || lower.contains("/tests/")
        || lower.contains("_test.") || lower.contains("_spec.")
        || lower.contains(".test.") || lower.contains(".spec.")
        || lower.starts_with("test_") || lower.contains("/test_")
}

/// Split a source file's lines into (source_lines, test_lines).
/// Detects Rust inline test modules (#[cfg(test)]) and counts lines after the marker as test lines.
fn split_inline_test_lines(lines: &[String]) -> (usize, usize) {
    // Find #[cfg(test)] marker — lines from there to end are test lines
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "#[cfg(test)]" {
            return (i, lines.len() - i);
        }
    }
    (lines.len(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::test_support;
    use tempfile::TempDir;

    #[test]
    fn test_no_tests() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        test_support::add_file(&store, &dir, "src/main.rs", "fn main() {}\n");
        let dim = QualityAssurance;
        let result = dim.evaluate(&store)?;
        let score = result.score.unwrap();
        assert!(score < 50, "no-test project should score <50, got {score}");
        let issues = result.issues;
        assert!(issues.iter().any(|i| i.level == Level::Critical && i.message.contains("no test")));
        Ok(())
    }

    #[test]
    fn test_with_tests() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        test_support::add_file(&store, &dir, "src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n");
        test_support::add_file(&store, &dir, "tests/test_lib.rs", "#[test]\nfn test_add() {\n    assert_eq!(add(1, 2), 3);\n}\n");
        let dim = QualityAssurance;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score > 50, "project with tests should score >50, got {score}");
        Ok(())
    }
}
