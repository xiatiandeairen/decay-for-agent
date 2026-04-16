use anyhow::Result;
use log::debug;

use super::helpers;
use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
/// Unsafe/eval occurrences per 1,000 lines triggering a warning.
/// 3+ per 1K lines means unsafe usage is habitual rather than exceptional.
/// FFI-heavy projects naturally have more unsafe code.
const UNSAFE_DENSITY_WARN: f64 = 3.0;
/// Critical unsafe density: 8+ per 1K lines indicates safety guarantees are systematically bypassed.
/// At this level, memory safety and sandboxing assumptions can no longer be trusted.
const UNSAFE_DENSITY_CRIT: f64 = 8.0;
/// Direct dependency count above which supply-chain risk becomes significant.
/// 60+ direct deps dramatically increases the attack surface and update burden.
const DEP_COUNT_WARN: usize = 60;
/// Critical dependency count. 100+ direct deps signals transitive exposure is very likely unaudited.
const DEP_COUNT_CRIT: usize = 100;
/// Unsafe/eval occurrences per file before flagging it individually in diagnosis.
/// More than 3 in one file suggests that file specifically is a reliability weak point.
const UNSAFE_PER_FILE_WARN: usize = 3;

pub struct Reliability;

impl Dimension for Reliability {
    fn name(&self) -> &'static str {
        "reliability"
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
        debug!("reliability: {} files, {} lines", analysis.file_count, analysis.total_lines);

        // Unsafe/eval density
        if analysis.total_lines > 0 {
            let density = analysis.unsafe_count as f64 / (analysis.total_lines as f64 / 1000.0);
            if density > UNSAFE_DENSITY_CRIT {
                score -= 30;
            } else if density > UNSAFE_DENSITY_WARN {
                score -= 15;
            }
        }
        for (path, count, ctx) in &analysis.unsafe_details {
            if *count > UNSAFE_PER_FILE_WARN {
                // FFI context: downgrade to Info — unsafe is expected
                let (level, priority) = if *ctx == helpers::FileContext::FFI {
                    (Level::Info, Priority::Low)
                } else {
                    (Level::Warning, Priority::High)
                };
                issues.push(Issue::with_actions(
                    level, name.clone(),
                    format!("{path} has {count} unsafe/eval occurrences"),
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Replace,
                        target: Target::file(path),
                        suggestion: format!("minimize unsafe code in {path}, prefer safe abstractions"),
                        reason: format!("{path} has {count} unsafe/eval"),
                        priority, effort: Effort::Medium,
                        details: vec![],
                        impact: None,
                        verify: String::new(),
                    }],
                ));
            }
        }

        // SQL/shell injection patterns
        let injection_penalty = (analysis.injection_patterns * 20).min(40) as i32;
        score -= injection_penalty;
        for (path, pattern, line_no) in &analysis.injection_details {
            issues.push(Issue::with_actions(
                Level::Critical, name.clone(), format!("{path}:{line_no}: potential {pattern}"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Replace,
                    target: Target::at(path.as_str(), (*line_no, *line_no), None),
                    suggestion: "use parameterized queries or safe command execution".into(),
                    reason: format!("{path}:{line_no}: potential {pattern}"),
                    priority: Priority::Critical, effort: Effort::Small,
                    details: vec![],
                    impact: None,
                    verify: String::new(),
                }],
            ));
        }

        // Hardcoded secrets
        let secret_penalty = (analysis.hardcoded_secrets * 15).min(30) as i32;
        score -= secret_penalty;
        for (path, kind, line_no) in &analysis.secret_details {
            issues.push(Issue::with_actions(
                Level::Critical, name.clone(), format!("{path}:{line_no}: {kind}"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Replace,
                    target: Target::at(path.as_str(), (*line_no, *line_no), None),
                    suggestion: "use environment variables or secret management for credentials".into(),
                    reason: format!("{path}:{line_no}: {kind}"),
                    priority: Priority::Critical, effort: Effort::Small,
                    details: vec![],
                    impact: None,
                    verify: String::new(),
                }],
            ));
        }

        // Dependency count
        let dep_count = store.dependencies().direct_count;
        if dep_count > DEP_COUNT_CRIT {
            score -= 20;
        } else if dep_count > DEP_COUNT_WARN {
            score -= 10;
        }
        if dep_count > DEP_COUNT_WARN {
            issues.push(Issue::with_actions(
                Level::Info, name, format!("{dep_count} direct dependencies"),
                vec![Action {
                    dimension: "reliability".into(), action_type: ActionType::Remove,
                    target: Target::file("."),
                    suggestion: "audit dependencies for necessity, remove unused ones".into(),
                    reason: format!("{dep_count} direct dependencies"),
                    priority: Priority::Medium, effort: Effort::Small,
                    details: vec![],
                    impact: None,
                    verify: String::new(),
                }],
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
    file_count: usize,
    total_lines: usize,
    unsafe_count: usize,
    injection_patterns: usize,
    hardcoded_secrets: usize,
    unsafe_details: Vec<(String, usize, helpers::FileContext)>, // (path, count, context)
    injection_details: Vec<(String, String, u32)>, // (path, pattern, line_no)
    secret_details: Vec<(String, String, u32)>,   // (path, kind, line_no)
}

fn analyze(source_files: &[SourceFile]) -> Analysis {
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut unsafe_count = 0;
    let mut injection_patterns = 0;
    let mut hardcoded_secrets = 0;
    let mut unsafe_details = Vec::new();
    let mut injection_details = Vec::new();
    let mut secret_details: Vec<(String, String, u32)> = Vec::new();

    let unsafe_patterns: &[&str] = &["unsafe {", "unsafe{", "eval(", "exec(", "Function("];

    for sf in source_files {
        file_count += 1;
        total_lines += sf.line_count;

        // Use helpers for unsafe pattern scanning
        let unsafe_hits = helpers::count_pattern_matches(&sf.lines, unsafe_patterns);
        let file_unsafe = unsafe_hits.len();
        unsafe_count += file_unsafe;

        let (inj, inj_det, sec, sec_det) = detect_injection_and_secrets(&sf.lines, &sf.path);
        injection_patterns += inj;
        injection_details.extend(inj_det);
        hardcoded_secrets += sec;
        secret_details.extend(sec_det);

        if file_unsafe > 0 {
            let ctx = helpers::detect_file_context(&sf.path, &sf.lines);
            unsafe_details.push((sf.path.clone(), file_unsafe, ctx));
        }
    }

    Analysis {
        file_count,
        total_lines,
        unsafe_count,
        injection_patterns,
        hardcoded_secrets,
        unsafe_details,
        injection_details,
        secret_details,
    }
}

/// Detect SQL/shell injection risks and hardcoded secrets in source lines.
/// Returns (injection_count, injection_details, secret_count, secret_details).
fn detect_injection_and_secrets(
    lines: &[String],
    file_path: &str,
) -> (usize, Vec<(String, String, u32)>, usize, Vec<(String, String, u32)>) {
    let mut injection_count = 0;
    let mut injection_details = Vec::new();
    let mut secret_count = 0;
    let mut secret_details = Vec::new();

    let test_lines = helpers::mark_test_lines(lines);

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if helpers::is_comment(trimmed) || test_lines[i] {
            continue;
        }
        let line_no = (i + 1) as u32;

        // SQL injection — skip error messages and logging that happen to mention SQL keywords.
        // Also skip detection-logic lines (contain `to_uppercase()` or `contains("SELECT`)
        // to avoid false positives when scanning our own source code.
        let is_detection_logic = trimmed.contains("to_uppercase()")
            || trimmed.contains("contains(\"SELECT")
            || trimmed.contains("contains(\"INSERT")
            || trimmed.contains("contains(\"DELETE")
            || trimmed.contains("contains(\"UPDATE");
        if !is_detection_logic
            && (trimmed.contains("format!(") || trimmed.contains("f\""))
            && (trimmed.to_uppercase().contains("SELECT ")
                || trimmed.to_uppercase().contains("INSERT ")
                || trimmed.to_uppercase().contains("DELETE ")
                || trimmed.to_uppercase().contains("UPDATE "))
        {
            let lower = trimmed.to_lowercase();
            let is_error_context = lower.contains("bail!")
                || lower.contains("anyhow!")
                || lower.contains("panic!")
                || lower.contains("eprintln!")
                || lower.contains("error!")
                || lower.contains("warn!")
                || lower.contains("\"failed")
                || lower.contains("\"error")
                || lower.contains("\"unable")
                || lower.contains("\"could not");
            if !is_error_context {
                injection_count += 1;
                injection_details.push((file_path.to_string(), "SQL string concatenation".to_string(), line_no));
            }
        }

        // Shell injection
        if (trimmed.contains("Command::new") || trimmed.contains("subprocess") || trimmed.contains("os.system"))
            && (trimmed.contains("format!(") || trimmed.contains("f\"") || trimmed.contains("+ "))
        {
            injection_count += 1;
            injection_details.push((file_path.to_string(), "shell command injection risk".to_string(), line_no));
        }

        // Hardcoded secrets
        let lower = trimmed.to_lowercase();
        if (lower.contains("password") || lower.contains("secret") || lower.contains("api_key") || lower.contains("apikey"))
            && (lower.contains("= \"") || lower.contains("= '"))
            && !lower.contains("env") && !lower.contains("config") && !lower.contains("example")
        {
            secret_count += 1;
            secret_details.push((file_path.to_string(), "hardcoded credential detected".to_string(), line_no));
        }
    }

    (injection_count, injection_details, secret_count, secret_details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::test_support;
    use tempfile::TempDir;

    #[test]
    fn test_safe_project() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        test_support::add_file(&store, &dir, "src/main.rs", "fn main() {\n    let x = 42;\n}\n");
        let dim = Reliability;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score > 80, "safe project should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_sql_injection_false_positive() {
        let lines: Vec<String> = vec![
            r#"bail!("failed to DELETE user {id}");"#.into(),
            r#"eprintln!("error: SELECT from {} failed", table);"#.into(),
            r#"let query = format!("SELECT * FROM users WHERE id = {}", id);"#.into(),
        ];
        let (count, details, _, _) = detect_injection_and_secrets(&lines, "test.rs");
        // Only the real SQL injection (line 3) should be detected, not error messages
        assert_eq!(count, 1);
        assert_eq!(details[0].2, 3); // line 3
    }

    #[test]
    fn test_unsafe_code() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        let content = (0..10).map(|_| "unsafe { std::ptr::null() };").collect::<Vec<_>>().join("\n");
        test_support::add_file(&store, &dir, "src/main.rs", &content);
        let dim = Reliability;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score < 80, "unsafe project should score <80, got {score}");
        Ok(())
    }
}
