use anyhow::Result;
use log::debug;

use super::helpers;
use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
/// Unwrap/panic calls per 1,000 lines. These mask errors and make failures opaque.
/// 5 per 1K lines is the point where panic-driven flow starts dominating error paths.
const UNWRAP_DENSITY_WARN: f64 = 5.0;  // per 1K lines
/// Critical unwrap density: 15+ per 1K lines means error handling is essentially absent.
/// At this level any unexpected input will likely crash the process with no useful context.
const UNWRAP_DENSITY_CRIT: f64 = 15.0;
/// Number of hardcoded URLs in non-comment source lines.
/// More than 5 suggests configuration is embedded in code instead of being externalized.
const HARDCODED_CONFIG_WARN: usize = 5;

pub struct Observability;

impl Dimension for Observability {
    fn name(&self) -> &'static str {
        "observability"
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
        debug!("observability: {} files, {} lines", analysis.file_count, analysis.total_lines);

        // Unwrap/panic density
        if analysis.total_lines > 0 {
            let density = analysis.unwrap_panic_count as f64 / (analysis.total_lines as f64 / 1000.0);
            if density > UNWRAP_DENSITY_CRIT {
                score -= 30;
            } else if density > UNWRAP_DENSITY_WARN {
                score -= 15;
            }
        }
        for (path, count, lines) in &analysis.unwrap_details {
            if *count > 5 {
                let line_range = lines.first().and_then(|first| {
                    lines.last().map(|last| (*first, *last))
                });
                let target = match line_range {
                    Some(range) => Target::at(path.as_str(), range, None),
                    None => Target::file(path),
                };
                issues.push(Issue::with_actions(
                    Level::Warning, name.clone(),
                    format!("{path} has {count} unwrap/panic calls"),
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Replace, target,
                        suggestion: format!("replace unwrap/panic in {path} with proper error handling"),
                        reason: format!("{path} has {count} unwrap/panic calls"),
                        priority: Priority::High, effort: Effort::Medium,
                        details: vec![],
                        impact: None,
                    }],
                ));
            }
        }

        // No logging framework
        if !analysis.has_logging {
            score -= 20;
            issues.push(Issue::with_actions(
                Level::Warning, name.clone(), "no logging framework detected in project",
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Add,
                    target: Target::file("."),
                    suggestion: "add structured logging (e.g. log/tracing/slog for Rust, logging for Python)".into(),
                    reason: "no logging framework detected".into(),
                    priority: Priority::High, effort: Effort::Medium,
                    details: vec![],
                    impact: None,
                }],
            ));
        }

        // Error swallowing
        if analysis.total_catches > 0 {
            let swallow_ratio = analysis.empty_catches as f64 / analysis.total_catches as f64;
            if swallow_ratio > 0.2 {
                score -= 15;
            }
        }
        if analysis.empty_catches > 0 {
            issues.push(Issue::with_actions(
                Level::Warning, name.clone(),
                format!("{} empty catch/except blocks detected", analysis.empty_catches),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Replace,
                    target: Target::file("."),
                    suggestion: "handle or log errors instead of silently swallowing them".into(),
                    reason: format!("{} empty catch blocks", analysis.empty_catches),
                    priority: Priority::High, effort: Effort::Small,
                    details: vec![],
                    impact: None,
                }],
            ));
        }

        // Hardcoded config
        if analysis.hardcoded_configs > HARDCODED_CONFIG_WARN {
            score -= 10;
        }
        if analysis.hardcoded_configs > 0 {
            issues.push(Issue::new(
                Level::Info, name,
                format!("{} hardcoded configuration values detected", analysis.hardcoded_configs),
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
    unwrap_panic_count: usize,
    has_logging: bool,
    total_catches: usize,
    empty_catches: usize,
    hardcoded_configs: usize,
    unwrap_details: Vec<(String, usize, Vec<u32>)>, // (path, count, line_numbers)
}

fn analyze(source_files: &[SourceFile]) -> Analysis {
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut unwrap_panic_count = 0;
    let mut has_logging = false;
    let mut total_catches = 0;
    let mut empty_catches = 0;
    let mut hardcoded_configs = 0;
    let mut unwrap_details = Vec::new();

    let log_patterns: &[&str] = &["log::", "tracing::", "slog::", "env_logger", "log4", "logger.",
        "logging.", "console.log", "console.error", "console.warn", "print!", "println!",
        "eprintln!", "debug!", "info!", "warn!", "error!"];

    let unwrap_patterns: &[&str] = &[".unwrap()", ".expect(", "panic!(", "unreachable!(", "unimplemented!(",
        "todo!("];

    let catch_patterns = ["catch", "except", "rescue"];

    for sf in source_files {
        file_count += 1;
        total_lines += sf.line_count;

        // Use helpers for pattern scanning (replaces nested loops)
        if !has_logging {
            let log_hits = helpers::count_pattern_matches(&sf.lines, log_patterns);
            if !log_hits.is_empty() {
                has_logging = true;
            }
        }

        let test_mask = helpers::mark_test_lines(&sf.lines);
        let unwrap_hits = helpers::count_pattern_matches_filtered(&sf.lines, unwrap_patterns, Some(&test_mask));
        let file_unwraps = unwrap_hits.len();
        let file_unwrap_lines: Vec<u32> = unwrap_hits.iter().map(|h| h.line_no).collect();
        unwrap_panic_count += file_unwraps;

        let (catches, empties, configs) = analyze_catch_and_config(&sf.lines, &catch_patterns);
        total_catches += catches;
        empty_catches += empties;
        hardcoded_configs += configs;

        if file_unwraps > 0 {
            unwrap_details.push((sf.path.clone(), file_unwraps, file_unwrap_lines));
        }
    }

    Analysis {
        file_count,
        total_lines,
        unwrap_panic_count,
        has_logging,
        total_catches,
        empty_catches,
        hardcoded_configs,
        unwrap_details,
    }
}

/// Detect catch blocks (with empty-catch check) and hardcoded config URLs.
/// Returns (total_catches, empty_catches, hardcoded_configs).
fn analyze_catch_and_config(lines: &[String], catch_patterns: &[&str]) -> (usize, usize, usize) {
    let mut total_catches = 0;
    let mut empty_catches = 0;
    let mut hardcoded_configs = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if helpers::is_comment(trimmed) {
            continue;
        }

        for pat in catch_patterns {
            if trimmed.starts_with(pat) || trimmed.contains(&format!("}} {pat}")) || trimmed.contains(&format!("{pat} ")) {
                total_catches += 1;
                let next_content = lines.get(i + 1).map(|l| l.trim()).unwrap_or("");
                if next_content == "}" || next_content == "pass" || next_content.is_empty() {
                    empty_catches += 1;
                }
            }
        }

        if (trimmed.contains("http://") || trimmed.contains("https://"))
            && !trimmed.contains("example.com")
            && !trimmed.contains("localhost")
        {
            hardcoded_configs += 1;
        }
    }

    (total_catches, empty_catches, hardcoded_configs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::test_support;
    use tempfile::TempDir;

    #[test]
    fn test_healthy_with_logging() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        test_support::add_file(&store, &dir, "src/main.rs", "use log::info;\nfn main() {\n    info!(\"starting\");\n}\n");
        let dim = Observability;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score > 70, "project with logging should score >70, got {score}");
        Ok(())
    }

    #[test]
    fn test_unwraps_in_test_code_ignored() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        let content = "use log::info;\nfn main() {\n    info!(\"ok\");\n}\n#[cfg(test)]\nmod tests {\n    fn test_it() {\n        let a = x.unwrap();\n        let b = y.unwrap();\n        let c = z.unwrap();\n        let d = w.unwrap();\n        let e = v.unwrap();\n        let f = u.unwrap();\n    }\n}\n";
        test_support::add_file(&store, &dir, "src/main.rs", content);
        let dim = Observability;
        let result = dim.evaluate(&store)?;
        // Test unwraps should not generate issues
        assert!(
            !result.issues.iter().any(|i| i.message.contains("unwrap/panic")),
            "unwraps in test code should not trigger issues"
        );
        Ok(())
    }

    #[test]
    fn test_many_unwraps() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        let content = (0..20).map(|i| format!("let x{i} = val.unwrap();")).collect::<Vec<_>>().join("\n");
        test_support::add_file(&store, &dir,"src/main.rs", &content);
        let dim = Observability;
        let issues = dim.evaluate(&store)?.issues;
        assert!(issues.iter().any(|i| i.message.contains("unwrap/panic")));
        Ok(())
    }
}
