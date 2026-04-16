/// Mechanical patch generation for category-A issues.
///
/// Generates structured fix suggestions (original → replacement) for issues
/// that have clear, pattern-based solutions. Patches are not auto-applied.

use serde::Serialize;

use crate::data_store::SourceFile;
use crate::diagnose::{Issue, IssueCategory};

/// A generated fix suggestion with before/after code.
#[derive(Debug, Clone, Serialize)]
pub struct Patch {
    pub file: String,
    pub line: u32,
    pub original: String,
    pub replacement: String,
    pub description: String,
}

/// Generate patches for all mechanical-fix (category A) issues.
pub fn generate_patches(issues: &[Issue], source_files: &[SourceFile]) -> Vec<Patch> {
    let mut patches = Vec::new();

    for issue in issues {
        if issue.classification != Some(IssueCategory::MechanicalFix) {
            continue;
        }
        let msg = issue.message.to_lowercase();

        if msg.contains("unwrap/panic") {
            if let Some(file_path) = issue.actions.first().map(|a| &a.target.file) {
                if let Some(sf) = source_files.iter().find(|f| &f.path == file_path) {
                    patches.extend(generate_unwrap_patches(sf));
                }
            }
        } else if msg.contains("empty catch") {
            patches.extend(generate_empty_catch_patches(source_files));
        } else if msg.contains("hardcoded configuration") {
            patches.extend(generate_hardcoded_config_patches(source_files));
        }
    }

    patches
}

/// Generate patches replacing .unwrap() with ? and .expect("msg") with .context("msg")?
fn generate_unwrap_patches(sf: &SourceFile) -> Vec<Patch> {
    let mut patches = Vec::new();

    for (i, line) in sf.lines.iter().enumerate() {
        let trimmed = line.trim();
        if crate::dimension::helpers::is_comment(trimmed) {
            continue;
        }
        // Skip test code
        let test_mask = crate::dimension::helpers::mark_test_lines(&sf.lines);
        if test_mask.get(i).copied().unwrap_or(false) {
            continue;
        }

        let line_no = (i + 1) as u32;

        if trimmed.contains(".unwrap()") {
            let replacement = trimmed.replace(".unwrap()", "?");
            patches.push(Patch {
                file: sf.path.clone(),
                line: line_no,
                original: trimmed.to_string(),
                replacement,
                description: "replace .unwrap() with ? operator".into(),
            });
        } else if trimmed.contains(".expect(") {
            // .expect("msg") → .context("msg")?
            if let Some(start) = trimmed.find(".expect(") {
                let after = &trimmed[start + 8..]; // skip ".expect("
                if let Some(end) = after.find(')') {
                    let msg = &after[..end];
                    let before = &trimmed[..start];
                    let after_paren = &trimmed[start + 8 + end + 1..];
                    let replacement = format!("{before}.context({msg})?{after_paren}");
                    patches.push(Patch {
                        file: sf.path.clone(),
                        line: line_no,
                        original: trimmed.to_string(),
                        replacement,
                        description: "replace .expect() with .context()?".into(),
                    });
                }
            }
        }
    }

    patches
}

/// Generate patches for empty catch/except blocks.
fn generate_empty_catch_patches(source_files: &[SourceFile]) -> Vec<Patch> {
    let mut patches = Vec::new();
    let catch_patterns = ["catch", "except", "rescue"];

    for sf in source_files {
        for (i, line) in sf.lines.iter().enumerate() {
            let trimmed = line.trim();
            if crate::dimension::helpers::is_comment(trimmed) {
                continue;
            }

            for pat in &catch_patterns {
                let is_catch = trimmed.starts_with(pat)
                    || trimmed.contains(&format!("}} {pat}"))
                    || trimmed.contains(&format!("{pat} "));
                if !is_catch {
                    continue;
                }
                let next = sf.lines.get(i + 1).map(|l| l.trim().to_string()).unwrap_or_default();
                if next == "}" || next == "pass" || next.is_empty() {
                    patches.push(Patch {
                        file: sf.path.clone(),
                        line: (i + 2) as u32, // the empty line after catch
                        original: next.clone(),
                        replacement: format!("    log::error!(\"caught error: {{:?}}\", e);"),
                        description: "add error logging to empty catch block".into(),
                    });
                }
            }
        }
    }

    patches
}

/// Generate patches for hardcoded URLs.
fn generate_hardcoded_config_patches(source_files: &[SourceFile]) -> Vec<Patch> {
    let mut patches = Vec::new();

    for sf in source_files {
        for (i, line) in sf.lines.iter().enumerate() {
            let trimmed = line.trim();
            if crate::dimension::helpers::is_comment(trimmed) {
                continue;
            }
            if (trimmed.contains("http://") || trimmed.contains("https://"))
                && !trimmed.contains("example.com")
                && !trimmed.contains("localhost")
            {
                patches.push(Patch {
                    file: sf.path.clone(),
                    line: (i + 1) as u32,
                    original: trimmed.to_string(),
                    replacement: "// TODO: extract URL to configuration / environment variable".into(),
                    description: "extract hardcoded URL to configuration".into(),
                });
            }
        }
    }

    patches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_source_file;

    #[test]
    fn test_unwrap_patches() {
        let sf = make_source_file("src/main.rs", "fn main() {\n    let x = foo.unwrap();\n    let y = bar.expect(\"oops\");\n}\n");
        let patches = generate_unwrap_patches(&sf);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].line, 2);
        assert!(patches[0].replacement.contains("?"));
        assert!(!patches[0].replacement.contains("unwrap"));
        assert_eq!(patches[1].line, 3);
        assert!(patches[1].replacement.contains("context"));
    }

    #[test]
    fn test_unwrap_skips_test_code() {
        let sf = make_source_file("src/lib.rs",
            "fn prod() {\n    x.unwrap();\n}\n#[cfg(test)]\nmod tests {\n    fn t() {\n        y.unwrap();\n    }\n}\n");
        let patches = generate_unwrap_patches(&sf);
        assert_eq!(patches.len(), 1); // only production unwrap
        assert_eq!(patches[0].line, 2);
    }

    #[test]
    fn test_empty_catch_patches() {
        let sf = make_source_file("src/app.py",
            "try:\n    do_thing()\nexcept Exception as e:\n    pass\n");
        let patches = generate_empty_catch_patches(&[sf]);
        assert_eq!(patches.len(), 1);
        assert!(patches[0].replacement.contains("log"));
    }

    #[test]
    fn test_hardcoded_config_patches() {
        let sf = make_source_file("src/client.rs",
            "let url = \"https://api.prod.com/v1\";\nlet local = \"http://localhost:8080\";\n");
        let patches = generate_hardcoded_config_patches(&[sf]);
        assert_eq!(patches.len(), 1); // localhost excluded
        assert!(patches[0].replacement.contains("TODO"));
    }

    #[test]
    fn test_generate_patches_filters_non_mechanical() {
        use crate::action::{Action, ActionType, Effort, Priority, Target};
        use crate::diagnose::Level;

        let issues = vec![
            Issue {
                level: Level::Warning,
                category: "observability".into(),
                message: "src/a.rs has 6 unwrap/panic calls".into(),
                classification: Some(IssueCategory::MechanicalFix),
                actions: vec![Action {
                    dimension: "observability".into(),
                    action_type: ActionType::Replace,
                    target: Target::file("src/a.rs"),
                    suggestion: "fix".into(),
                    reason: "broken".into(),
                    priority: Priority::High,
                    effort: Effort::Small,
                }],
            },
            Issue {
                level: Level::Critical,
                category: "structural".into(),
                message: "1200 files in project".into(),
                classification: Some(IssueCategory::ArchitecturalDecision),
                actions: vec![],
            },
        ];
        let sf = make_source_file("src/a.rs", "let x = val.unwrap();\nlet y = 42;\n");
        let patches = generate_patches(&issues, &[sf]);
        assert_eq!(patches.len(), 1); // only the mechanical fix
    }
}
