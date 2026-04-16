use anyhow::Result;
use log::debug;

use super::helpers;
use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
/// Number of deeply nested loop sites (depth ≥ 3) before penalizing.
/// More than 5 occurrences means O(n³)+ complexity is not an isolated incident.
const DEEP_NEST_WARN: usize = 5;
/// Critical nested loop count: 15+ sites with 3+ loop levels likely tanks throughput under load.
const DEEP_NEST_CRIT: usize = 15;
/// Clone/copy calls per 1,000 lines triggering a warning. Excessive cloning inflates allocations.
/// 15 per 1K lines is where clone pressure starts to show up in profiling on hot paths.
/// Rust projects naturally use more clone than GC languages.
const CLONE_DENSITY_WARN: f64 = 15.0;
/// Critical clone density: 25+ per 1K lines means ownership design is consistently avoided.
/// At this level, heap allocation patterns are likely causing measurable memory pressure.
const CLONE_DENSITY_CRIT: f64 = 25.0;
/// Number of synchronous blocking calls (sleep, block_on) before penalizing.
/// More than 10 blocking calls in async code risks starving the runtime's thread pool.
/// Test code and pattern strings inflate this count, so threshold is generous.
const BLOCKING_CALLS_WARN: usize = 10;
/// Critical blocking call count: 15+ makes async throughput effectively synchronous.
const BLOCKING_CALLS_CRIT: usize = 15;
/// Minimum loop nesting depth considered "deep" for nest detection.
const DEEP_NEST_DEPTH: usize = 3;

pub struct Performance;

impl Dimension for Performance {
    fn name(&self) -> &'static str {
        "performance"
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
        debug!("performance: {} files, {} lines", analysis.file_count, analysis.total_lines);

        // Deep nested loops
        if analysis.deep_nests > DEEP_NEST_CRIT {
            score -= 30;
        } else if analysis.deep_nests > DEEP_NEST_WARN {
            score -= 15;
        }
        for (path, line_no, depth, ctx) in &analysis.nest_details {
            // Parser context: downgrade — nested loops are expected in parsers
            let (level, priority) = if *ctx == helpers::FileContext::Parser {
                (Level::Info, Priority::Low)
            } else if *depth >= 4 {
                (Level::Critical, Priority::Critical)
            } else {
                (Level::Warning, Priority::High)
            };
            let ln = *line_no as u32;
            issues.push(Issue::with_actions(
                level, name.clone(), format!("{path}:{line_no} has {depth}-level nested loop"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Extract,
                    target: Target::at(path.as_str(), (ln, ln), None),
                    suggestion: "extract inner loops into separate functions or use iterators".into(),
                    reason: format!("{depth}-level nested loop at line {line_no}"),
                    priority, effort: Effort::Small,
                    details: vec![],
                    impact: None,
                    verify: String::new(),
                }],
            ));
        }

        // Clone/copy density
        if analysis.total_lines > 0 {
            let density = analysis.clone_count as f64 / (analysis.total_lines as f64 / 1000.0);
            if density > CLONE_DENSITY_CRIT {
                score -= 20;
            } else if density > CLONE_DENSITY_WARN {
                score -= 10;
            }
        }
        for (path, count, ctx) in &analysis.clone_details {
            if *count > 10 {
                // Builder context: downgrade — cloning is idiomatic in builder patterns
                let (level, priority) = if *ctx == helpers::FileContext::Builder {
                    (Level::Info, Priority::Low)
                } else {
                    (Level::Warning, Priority::Medium)
                };
                issues.push(Issue::with_actions(
                    level, name.clone(),
                    format!("{path} has {count} clone/copy calls"),
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Refactor,
                        target: Target::file(path),
                        suggestion: format!("reduce cloning in {path}, prefer references or Cow"),
                        reason: format!("{path} has {count} clones"),
                        priority, effort: Effort::Medium,
                        details: vec![],
                        impact: None,
                        verify: String::new(),
                    }],
                ));
            }
        }

        // Sync blocking calls
        if analysis.blocking_calls > BLOCKING_CALLS_CRIT {
            score -= 20;
        } else if analysis.blocking_calls > BLOCKING_CALLS_WARN {
            score -= 10;
        }
        for (path, call) in &analysis.blocking_details {
            issues.push(Issue::new(
                Level::Info, name.clone(), format!("{path}: blocking call {call}"),
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
    deep_nests: usize,
    clone_count: usize,
    blocking_calls: usize,
    nest_details: Vec<(String, usize, usize, helpers::FileContext)>, // (path, line, depth, ctx)
    clone_details: Vec<(String, usize, helpers::FileContext)>, // (path, count, ctx)
    blocking_details: Vec<(String, String)>,
}

fn analyze(source_files: &[SourceFile]) -> Analysis {
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut deep_nests = 0;
    let mut clone_count = 0;
    let mut blocking_calls = 0;
    let mut nest_details = Vec::new();
    let mut clone_details = Vec::new();
    let mut blocking_details: Vec<(String, String)> = Vec::new();

    let clone_patterns: &[&str] = &[".clone()", ".copy()", ".deepcopy(", ".to_owned()", "Clone::clone"];
    let blocking_pattern_pairs = [
        ("thread::sleep", "thread::sleep"),
        ("time.sleep", "time.sleep"),
        ("Sleep(", "Sleep"),
        ("std::thread::sleep", "std::thread::sleep"),
        (".block_on(", "block_on"),
    ];
    let blocking_pats: Vec<&str> = blocking_pattern_pairs.iter().map(|(p, _)| *p).collect();
    let loop_keywords = ["for ", "while ", "loop {", "loop{"];

    for sf in source_files {
        file_count += 1;
        total_lines += sf.line_count;
        let ctx = helpers::detect_file_context(&sf.path, &sf.lines);

        // Use helpers for clone pattern scanning
        let clone_hits = helpers::count_pattern_matches(&sf.lines, clone_patterns);
        let file_clones = clone_hits.len();
        clone_count += file_clones;

        // Use helpers for blocking call scanning — skip test files and pattern definition files
        let test_mask = if ctx == helpers::FileContext::Test {
            vec![true; sf.lines.len()] // skip entire test files
        } else {
            helpers::mark_test_lines(&sf.lines)
        };
        let blocking_hits = helpers::count_pattern_matches_filtered(&sf.lines, &blocking_pats, Some(&test_mask));
        for hit in &blocking_hits {
            // Skip pattern definitions: lines where the match is inside a string literal
            let line = sf.lines.get(hit.line_no as usize - 1).map(|l| l.trim()).unwrap_or("");
            if line.contains(&format!("\"{}\"", hit.pattern))
                || line.contains(&format!("\"{}", hit.pattern))
            {
                continue;
            }
            // Map back to label
            let label = blocking_pattern_pairs.iter()
                .find(|(p, _)| *p == hit.pattern)
                .map(|(_, l)| *l)
                .unwrap_or(&hit.pattern);
            blocking_calls += 1;
            blocking_details.push((sf.path.clone(), label.to_string()));
        }

        // Loop nesting tracking needs stateful brace analysis — keep inline
        let mut loop_depth: usize = 0;
        let mut brace_stack: Vec<bool> = Vec::new();

        for (i, line) in sf.lines.iter().enumerate() {
            let trimmed = line.trim();
            if helpers::is_comment(trimmed) {
                continue;
            }

            let is_loop_start = loop_keywords.iter().any(|kw| trimmed.starts_with(kw) || trimmed.contains(&format!(" {kw}")));
            let opens = trimmed.matches('{').count();
            let closes = trimmed.matches('}').count();

            if is_loop_start {
                loop_depth += 1;
                if loop_depth >= DEEP_NEST_DEPTH {
                    deep_nests += 1;
                    nest_details.push((sf.path.clone(), i + 1, loop_depth, ctx));
                }
            }

            for _ in 0..opens {
                brace_stack.push(is_loop_start);
            }
            for _ in 0..closes {
                if let Some(was_loop) = brace_stack.pop() {
                    if was_loop && loop_depth > 0 {
                        loop_depth -= 1;
                    }
                }
            }
        }

        if file_clones > 0 {
            clone_details.push((sf.path.clone(), file_clones, ctx));
        }
    }

    Analysis {
        file_count,
        total_lines,
        deep_nests,
        clone_count,
        blocking_calls,
        nest_details,
        clone_details,
        blocking_details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::test_support;
    use tempfile::TempDir;

    #[test]
    fn test_clean_performance() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        test_support::add_file(&store, &dir, "src/main.rs", "fn main() {\n    let x = 42;\n}\n");
        let dim = Performance;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score > 80, "clean project should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_many_clones() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        let content = (0..30).map(|i| format!("let x{i} = data.clone();")).collect::<Vec<_>>().join("\n");
        test_support::add_file(&store, &dir, "src/main.rs", &content);
        let dim = Performance;
        let issues = dim.evaluate(&store)?.issues;
        assert!(issues.iter().any(|i| i.message.contains("clone/copy")));
        Ok(())
    }
}
