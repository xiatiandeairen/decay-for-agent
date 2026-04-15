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
///
/// If `test_mask` is provided, lines marked as test code are also skipped.
pub fn count_pattern_matches(lines: &[String], patterns: &[&str]) -> Vec<PatternHit> {
    count_pattern_matches_filtered(lines, patterns, None)
}

/// Like `count_pattern_matches`, but with optional test-code filtering.
pub fn count_pattern_matches_filtered(
    lines: &[String],
    patterns: &[&str],
    test_mask: Option<&[bool]>,
) -> Vec<PatternHit> {
    let mut hits = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed) {
            continue;
        }
        if let Some(mask) = test_mask {
            if mask.get(i).copied().unwrap_or(false) {
                continue;
            }
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

/// Mark which lines are inside test blocks (`#[cfg(test)]` modules or `#[test]` functions).
/// Returns a bool per line: `true` = inside test code.
pub fn mark_test_lines(lines: &[String]) -> Vec<bool> {
    let mut result = vec![false; lines.len()];
    let mut in_test_block = false;
    let mut brace_depth: i32 = 0;
    let mut test_start_depth: i32 = 0;
    let mut pending_test_attr = false; // saw #[test] or #[cfg(test)], waiting for fn/mod

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if !in_test_block {
            if trimmed == "#[cfg(test)]" || trimmed == "#[test]" {
                pending_test_attr = true;
            } else if pending_test_attr
                && (trimmed.starts_with("mod ") || trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub mod ") || trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("async fn ") || trimmed.starts_with("pub async fn "))
            {
                in_test_block = true;
                test_start_depth = brace_depth;
                pending_test_attr = false;
            } else if pending_test_attr && !trimmed.starts_with("#[") && !trimmed.is_empty() {
                // Attribute not followed by fn/mod — reset
                pending_test_attr = false;
            }
        }

        // Count braces (simple, not string-aware — good enough for structural detection)
        for ch in trimmed.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
        }

        if in_test_block {
            result[i] = true;
            if brace_depth <= test_start_depth {
                in_test_block = false;
            }
        }
    }

    result
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

    #[test]
    fn test_mark_test_lines_cfg_test() {
        let lines: Vec<String> = vec![
            "fn main() {",
            "    let x = foo.unwrap();",
            "}",
            "#[cfg(test)]",
            "mod tests {",
            "    fn test_foo() {",
            "        bar.unwrap();",
            "    }",
            "}",
        ].into_iter().map(Into::into).collect();

        let mask = mark_test_lines(&lines);
        assert!(!mask[0]); // fn main
        assert!(!mask[1]); // unwrap in main
        assert!(!mask[2]); // }
        assert!(!mask[3]); // #[cfg(test)] — attr itself not in block yet
        assert!(mask[4]);  // mod tests {
        assert!(mask[5]);  // fn test_foo
        assert!(mask[6]);  // unwrap in test
        assert!(mask[7]);  // }
        assert!(mask[8]);  // } closing mod
    }

    #[test]
    fn test_mark_test_lines_test_fn() {
        let lines: Vec<String> = vec![
            "fn production() {",
            "    real.unwrap();",
            "}",
            "#[test]",
            "fn test_something() {",
            "    val.unwrap();",
            "}",
        ].into_iter().map(Into::into).collect();

        let mask = mark_test_lines(&lines);
        assert!(!mask[0]);
        assert!(!mask[1]);
        assert!(!mask[2]);
        assert!(!mask[3]); // #[test] attr
        assert!(mask[4]);  // fn test_something
        assert!(mask[5]);  // unwrap in test
        assert!(mask[6]);  // }
    }

    #[test]
    fn test_count_pattern_matches_with_test_mask() {
        let lines: Vec<String> = vec![
            "prod.unwrap();",
            "test.unwrap();",
            "prod2.unwrap();",
        ].into_iter().map(Into::into).collect();

        let mask = vec![false, true, false];
        let hits = count_pattern_matches_filtered(&lines, &[".unwrap()"], Some(&mask));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line_no, 1);
        assert_eq!(hits[1].line_no, 3);
    }
}
