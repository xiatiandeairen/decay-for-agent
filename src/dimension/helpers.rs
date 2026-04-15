/// Shared helpers for dimension implementations.
///
/// Eliminates repeated pattern-scanning loops across dimensions.

/// A single pattern match with location info.
#[derive(Debug, Clone)]
pub struct PatternHit {
    pub line_no: u32,
    pub pattern: String,
}

/// Scan lines for pattern matches, skipping comment lines.
///
/// Returns one `PatternHit` per match (a line matching multiple patterns
/// produces multiple hits). This replaces the nested
/// `for line { for pat { if contains } }` loops in 5+ dimensions.
pub fn count_pattern_matches(lines: &[String], patterns: &[&str]) -> Vec<PatternHit> {
    let mut hits = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed) {
            continue;
        }
        for pat in patterns {
            if trimmed.contains(pat) {
                hits.push(PatternHit {
                    line_no: (i + 1) as u32,
                    pattern: (*pat).to_string(),
                });
            }
        }
    }
    hits
}

/// Check if a trimmed line is a comment (should be skipped in analysis).
pub fn is_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("///")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
}

/// Check if a file is auto-generated or non-source (lock files, minified, config, docs).
pub fn is_generated_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    // Lock files
    if lower.ends_with(".lock") || lower.ends_with("lock.json") {
        return true;
    }
    // Common generated files
    if lower.ends_with(".min.js") || lower.ends_with(".min.css") {
        return true;
    }
    // Markdown/docs
    if lower.ends_with(".md") || lower.ends_with(".txt") || lower.ends_with(".rst") {
        return true;
    }
    // JSON/YAML config — not source code
    if lower.ends_with(".json")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
        || lower.ends_with(".toml")
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_pattern_matches_basic() {
        let lines = vec![
            "fn main() {".into(),
            "    let x = foo.unwrap();".into(),
            "    let y = bar.expect(\"msg\");".into(),
            "    // foo.unwrap() in comment".into(),
            "    baz();".into(),
        ];
        let hits = count_pattern_matches(&lines, &[".unwrap()", ".expect("]);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line_no, 2);
        assert_eq!(hits[0].pattern, ".unwrap()");
        assert_eq!(hits[1].line_no, 3);
        assert_eq!(hits[1].pattern, ".expect(");
    }

    #[test]
    fn test_count_pattern_matches_skips_comments() {
        let lines = vec![
            "// .unwrap() here".into(),
            "# .unwrap() here".into(),
            "/// .unwrap() here".into(),
            "real.unwrap()".into(),
        ];
        let hits = count_pattern_matches(&lines, &[".unwrap()"]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_no, 4);
    }

    #[test]
    fn test_count_pattern_matches_multiple_per_line() {
        let lines = vec!["foo.unwrap().expect(\"x\")".into()];
        let hits = count_pattern_matches(&lines, &[".unwrap()", ".expect("]);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn test_is_comment() {
        assert!(is_comment("// comment"));
        assert!(is_comment("# comment"));
        assert!(is_comment("/// doc comment"));
        assert!(is_comment("/* block comment"));
        assert!(is_comment("* continuation"));
        assert!(!is_comment("let x = 1;"));
        assert!(!is_comment("fn main() {"));
    }

    #[test]
    fn test_is_generated_file() {
        assert!(is_generated_file("Cargo.lock"));
        assert!(is_generated_file("package-lock.json"));
        assert!(is_generated_file("app.min.js"));
        assert!(is_generated_file("README.md"));
        assert!(is_generated_file("config.toml"));
        assert!(!is_generated_file("src/main.rs"));
        assert!(!is_generated_file("lib.py"));
    }
}
