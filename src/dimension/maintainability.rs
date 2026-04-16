use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use anyhow::Result;
use log::debug;

use super::helpers;
use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
/// Files exceeding 300 lines are flagged as long. This is a maintenance burden indicator.
/// 300 lines is roughly the upper bound for a file to remain fully readable in one sitting.
const LONG_FILE_LINES: usize = 300;
/// Fraction of files considered long that triggers a warning.
/// 20% means the pattern is widespread enough to slow down navigation and review.
const LONG_FILE_RATIO_WARN: f64 = 0.20;
/// Critical ratio: 40%+ long files signals systemic poor decomposition across the codebase.
const LONG_FILE_RATIO_CRIT: f64 = 0.40;
/// Functions exceeding 60 lines are hard to reason about and test in isolation.
/// 60 lines accounts for Rust's match expressions and error handling verbosity.
const LONG_FUNC_LINES: usize = 60;
/// 15%+ of functions being long is a warning that single-responsibility isn't being enforced.
const LONG_FUNC_RATIO_WARN: f64 = 0.15;
/// 25%+ long functions is critical — refactoring has become necessary to keep the codebase safe.
const LONG_FUNC_RATIO_CRIT: f64 = 0.25;
/// Fraction of files sharing duplicate code blocks.
/// 15% means duplication is widespread; test pattern similarity is normal below this.
const DUP_FILE_RATIO_WARN: f64 = 0.15;
/// 30%+ files with shared duplicate blocks signals copy-paste culture has taken hold.
const DUP_FILE_RATIO_CRIT: f64 = 0.30;
/// TODO/FIXME density per 10,000 lines of code. High density signals deferred debt.
/// 20 per 10K lines (~1 per 500 lines) is where debt starts compounding faster than it's resolved.
const TODO_DENSITY_WARN: f64 = 20.0; // per 10K lines
/// Minimum consecutive non-blank, non-comment lines to constitute a duplicate block.
/// 6 lines is large enough to avoid false positives from common boilerplate patterns.
const MIN_DUP_BLOCK: usize = 6;

pub struct Maintainability;

impl Dimension for Maintainability {
    fn name(&self) -> &'static str {
        "maintainability"
    }

    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult> {
        let source_files = store.source_files();
        let name = self.name().to_string();

        if source_files.is_empty() {
            return Ok(DimensionResult { name, score: Some(100), issues: vec![] });
        }

        let analysis = analyze_files(source_files);
        let mut score: i32 = 100;
        let mut issues = Vec::new();
        debug!("maintainability: {} files analyzed", analysis.file_count);

        // Duplicate code ratio
        if analysis.file_count > 0 {
            let dup_ratio = analysis.files_with_dups as f64 / analysis.file_count as f64;
            if dup_ratio > DUP_FILE_RATIO_CRIT {
                score -= 25;
            } else if dup_ratio > DUP_FILE_RATIO_WARN {
                score -= 10;
            }
        }
        for (path, dup_count) in &analysis.dup_details {
            if *dup_count > 0 {
                issues.push(Issue::with_actions(
                    Level::Warning, name.clone(),
                    format!("{path} has {dup_count} duplicate block(s) shared with other files"),
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Extract,
                        target: Target::file(path),
                        suggestion: format!("extract shared logic from {path} into a common module"),
                        reason: format!("{path} has {dup_count} duplicate blocks"),
                        priority: Priority::High, effort: Effort::Medium,
                        details: vec![],
                    }],
                ));
            }
        }

        // Long file ratio
        if analysis.file_count > 0 {
            let long_ratio = analysis.long_files as f64 / analysis.file_count as f64;
            if long_ratio > LONG_FILE_RATIO_CRIT {
                score -= 30;
            } else if long_ratio > LONG_FILE_RATIO_WARN {
                score -= 15;
            }
        }
        for (path, lines) in &analysis.long_file_details {
            let level = if *lines > 600 { Level::Critical } else { Level::Warning };
            let priority = if *lines > 600 { Priority::Critical } else { Priority::High };
            // Generate specific split suggestions based on function analysis
            let details = source_files
                .iter()
                .find(|sf| sf.path == *path)
                .map(|sf| helpers::suggest_split_details(&sf.lines, path))
                .unwrap_or_default();
            let suggestion = if details.is_empty() {
                format!("split {path} into smaller modules")
            } else {
                format!("split {path} by responsibility ({} groups identified)", details.len())
            };
            issues.push(Issue::with_actions(
                level, name.clone(), format!("{path} has {lines} lines"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Split,
                    target: Target::file(path),
                    suggestion,
                    reason: format!("{path} has {lines} lines"),
                    priority, effort: Effort::Medium,
                    details,
                }],
            ));
        }

        // Long function ratio
        if analysis.total_functions > 0 {
            let func_ratio = analysis.long_functions as f64 / analysis.total_functions as f64;
            if func_ratio > LONG_FUNC_RATIO_CRIT {
                score -= 20;
            } else if func_ratio > LONG_FUNC_RATIO_WARN {
                score -= 10;
            }
        }
        for (path, func_name, lines, start_line) in &analysis.long_func_details {
            let start = *start_line as u32;
            let end = start + *lines as u32;
            issues.push(Issue::with_actions(
                Level::Warning, name.clone(),
                format!("{func_name} in {path} is {lines} lines long"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Extract,
                    target: Target::at(path.as_str(), (start, end), Some(func_name.clone())),
                    suggestion: format!("break {func_name} into smaller functions"),
                    reason: format!("{func_name} is {lines} lines"),
                    priority: Priority::High, effort: Effort::Small,
                    details: vec![],
                }],
            ));
        }

        // TODO/FIXME density
        if analysis.total_lines > 0 {
            let density = analysis.todo_count as f64 / (analysis.total_lines as f64 / 10000.0);
            if density > TODO_DENSITY_WARN {
                score -= 5;
            }
        }
        if analysis.todo_count > 0 {
            issues.push(Issue::new(
                Level::Info, name,
                format!("{} TODO/FIXME comments across project", analysis.todo_count),
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
    files_with_dups: usize,
    long_files: usize,
    total_functions: usize,
    long_functions: usize,
    todo_count: usize,
    dup_details: Vec<(String, usize)>,
    long_file_details: Vec<(String, usize)>,
    long_func_details: Vec<(String, String, usize, usize)>, // (path, func_name, func_len, start_line)
}

fn analyze_files(source_files: &[SourceFile]) -> Analysis {
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut long_files = 0;
    let mut total_functions = 0;
    let mut long_functions = 0;
    let mut todo_count = 0;
    let mut long_file_details = Vec::new();
    let mut long_func_details = Vec::new();

    // For duplicate detection: map block fingerprint -> list of (file, line_no)
    let mut block_index: HashMap<u64, Vec<(String, usize)>> = HashMap::new();

    for sf in source_files {
        // Skip auto-generated and non-source files
        if helpers::is_generated_file(&sf.path) {
            continue;
        }

        file_count += 1;
        let line_count = sf.line_count;
        total_lines += line_count;

        // Long file check
        if line_count > LONG_FILE_LINES {
            long_files += 1;
            long_file_details.push((sf.path.clone(), line_count));
        }

        // TODO/FIXME count
        for line in &sf.lines {
            let upper = line.to_uppercase();
            if upper.contains("TODO") || upper.contains("FIXME") {
                todo_count += 1;
            }
        }

        // Function length detection
        let line_refs: Vec<&str> = sf.lines.iter().map(|s| s.as_str()).collect();
        let func_positions = detect_functions(&line_refs);
        total_functions += func_positions.len();
        for i in 0..func_positions.len() {
            let (func_name, start) = &func_positions[i];
            let end = if i + 1 < func_positions.len() {
                func_positions[i + 1].1
            } else {
                line_count
            };
            let func_len = end - start;
            if func_len > LONG_FUNC_LINES {
                long_functions += 1;
                long_func_details.push((sf.path.clone(), func_name.clone(), func_len, *start));
            }
        }

        index_block_fingerprints(&sf.lines, &sf.path, &mut block_index);
    }

    let (files_with_dups, dup_details) = count_cross_file_duplicates(&block_index);

    Analysis {
        file_count,
        total_lines,
        files_with_dups,
        long_files,
        total_functions,
        long_functions,
        todo_count,
        dup_details,
        long_file_details,
        long_func_details,
    }
}

/// Build block fingerprints for duplicate detection across files.
fn index_block_fingerprints(
    lines: &[String],
    file_path: &str,
    block_index: &mut HashMap<u64, Vec<(String, usize)>>,
) {
    let normalized: Vec<u64> = lines
        .iter()
        .map(|l| {
            let trimmed = l.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                0
            } else {
                let mut hasher = DefaultHasher::new();
                trimmed.hash(&mut hasher);
                hasher.finish()
            }
        })
        .collect();

    for start in 0..normalized.len().saturating_sub(MIN_DUP_BLOCK) {
        if normalized[start] == 0 {
            continue;
        }
        let block: Vec<u64> = normalized[start..start + MIN_DUP_BLOCK].to_vec();
        if block.iter().any(|h| *h == 0) {
            continue;
        }
        let mut hasher = DefaultHasher::new();
        block.hash(&mut hasher);
        let fingerprint = hasher.finish();
        block_index
            .entry(fingerprint)
            .or_default()
            .push((file_path.to_string(), start));
    }
}

/// Count files with duplicate blocks appearing in more than one file.
fn count_cross_file_duplicates(
    block_index: &HashMap<u64, Vec<(String, usize)>>,
) -> (usize, Vec<(String, usize)>) {
    let mut files_with_dups_set: HashMap<String, usize> = HashMap::new();
    for locations in block_index.values() {
        let unique_files: std::collections::HashSet<&str> =
            locations.iter().map(|(f, _)| f.as_str()).collect();
        if unique_files.len() > 1 {
            for f in &unique_files {
                *files_with_dups_set.entry(f.to_string()).or_default() += 1;
            }
        }
    }
    let files_with_dups = files_with_dups_set.len();
    let dup_details: Vec<(String, usize)> = files_with_dups_set.into_iter().collect();
    (files_with_dups, dup_details)
}

/// Detect function definitions and return (name, line_number) pairs.
fn detect_functions(lines: &[&str]) -> Vec<(String, usize)> {
    let mut results = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = extract_func_name(trimmed) {
            results.push((name, i));
        }
    }
    results
}

fn extract_func_name(line: &str) -> Option<String> {
    // Rust: fn name(
    // Also matches pub fn, pub(crate) fn, async fn, etc.
    if let Some(pos) = line.find("fn ") {
        let after = &line[pos + 3..];
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() && (after.contains('(') || after.contains('<')) {
            return Some(name);
        }
    }

    // Python: def name(
    if line.starts_with("def ") || line.starts_with("async def ") {
        let after = if line.starts_with("async def ") {
            &line[10..]
        } else {
            &line[4..]
        };
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            return Some(name);
        }
    }

    // Go: func name(
    if line.starts_with("func ") {
        let after = &line[5..];
        // Skip receiver: func (r *Receiver) name(
        let after = if after.starts_with('(') {
            if let Some(close) = after.find(')') {
                after[close + 1..].trim_start()
            } else {
                after
            }
        } else {
            after
        };
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            return Some(name);
        }
    }

    // JavaScript/TypeScript: function name(
    if line.starts_with("function ") || line.starts_with("async function ") {
        let after = if line.starts_with("async function ") {
            &line[15..]
        } else {
            &line[9..]
        };
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            return Some(name);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::test_support;
    use tempfile::TempDir;

    #[test]
    fn test_healthy_project() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        test_support::add_file(&store, &dir, "src/main.rs", "fn main() {\n    println!(\"hello\");\n}\n");
        test_support::add_file(&store, &dir, "src/lib.rs", "pub fn greet() {\n    println!(\"hi\");\n}\n");

        let dim = Maintainability;
        let result = dim.evaluate(&store)?;
        let score = result.score.unwrap();
        assert!(score > 80, "healthy project should score >80, got {score}");
        let issues = result.issues;
        assert!(issues.is_empty() || issues.iter().all(|i| i.level == Level::Info));
        Ok(())
    }

    #[test]
    fn test_long_file_detected() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        let long_content = (0..400).map(|i| format!("let x{i} = {i};")).collect::<Vec<_>>().join("\n");
        test_support::add_file(&store, &dir, "src/big.rs", &long_content);
        test_support::add_file(&store, &dir, "src/small.rs", "fn main() {}\n");

        let dim = Maintainability;
        let issues = dim.evaluate(&store)?.issues;
        assert!(issues.iter().any(|i| i.message.contains("big.rs") && i.message.contains("400")));
        Ok(())
    }

    #[test]
    fn test_todo_detected() -> Result<()> {
        let dir = TempDir::new()?;
        let store = test_support::setup_store(&dir);
        let content = "fn main() {\n    // TODO: fix this\n    // FIXME: and this\n}\n";
        test_support::add_file(&store, &dir, "src/main.rs", content);

        let dim = Maintainability;
        let issues = dim.evaluate(&store)?.issues;
        assert!(issues.iter().any(|i| i.message.contains("TODO/FIXME")));
        Ok(())
    }

    #[test]
    fn test_extract_func_name_rust() {
        assert_eq!(extract_func_name("fn main() {"), Some("main".to_string()));
        assert_eq!(extract_func_name("pub fn hello(x: i32) {"), Some("hello".to_string()));
        assert_eq!(extract_func_name("async fn run() {"), Some("run".to_string()));
        assert_eq!(extract_func_name("let x = 5;"), None);
    }

    #[test]
    fn test_extract_func_name_python() {
        assert_eq!(extract_func_name("def hello():"), Some("hello".to_string()));
        assert_eq!(extract_func_name("async def run():"), Some("run".to_string()));
    }

    #[test]
    fn test_extract_func_name_go() {
        assert_eq!(extract_func_name("func main() {"), Some("main".to_string()));
        assert_eq!(extract_func_name("func (s *Server) Run() {"), Some("Run".to_string()));
    }
}
