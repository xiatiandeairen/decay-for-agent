use std::fs;
use std::path::{Path, PathBuf};

use crate::config::EXCLUDED_DIRS;
use crate::error::{DecayError, Result};

/// Recursively walk `project_root` and return every `.rs` file path.
///
/// Directories whose name matches `EXCLUDED_DIRS` (e.g. `target`, `.git`) are
/// skipped at any depth. Returned paths are absolute when `project_root` is
/// absolute; the parser is responsible for converting them to project-relative
/// paths.
///
/// Returns `DecayError::Io` when a directory listing fails (e.g. permission
/// denied on `project_root`). Per-entry metadata errors propagate the same
/// variant with the offending path.
pub fn walk_rust_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    walk_rust_files_with_excludes(project_root, &[])
}

/// Recursively walk `project_root`, honoring the default excluded directories
/// plus user-provided basename/path/glob excludes.
pub fn walk_rust_files_with_excludes(
    project_root: &Path,
    excludes: &[String],
) -> Result<Vec<PathBuf>> {
    let gitignore = GitignoreRules::load(project_root)?;
    let excludes = ExcludeSet::new(excludes);
    let mut out = Vec::new();
    walk_dir(project_root, project_root, &gitignore, &excludes, &mut out)?;
    Ok(out)
}

fn walk_dir(
    project_root: &Path,
    dir: &Path,
    gitignore: &GitignoreRules,
    excludes: &ExcludeSet,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in read_dir(dir)? {
        let entry = entry.map_err(io_err(dir))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(io_err(&path))?;

        if file_type.is_dir() {
            visit_dir(project_root, &path, gitignore, excludes, out)?;
        } else if file_type.is_file()
            && !gitignore.is_ignored(project_root, &path, false)
            && !excludes.matches_path(project_root, &path)
        {
            collect_if_rs(path, out);
        }
        // Symlinks and other entry kinds are ignored.
    }
    Ok(())
}

/// `fs::read_dir` with consistent IO error wrapping.
fn read_dir(dir: &Path) -> Result<fs::ReadDir> {
    fs::read_dir(dir).map_err(io_err(dir))
}

/// Build a closure that wraps any `io::Error` into `DecayError::Io` carrying
/// the offending path. Owned String avoids lifetime issues at the call site.
fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> DecayError {
    let path = path.display().to_string();
    move |source| DecayError::Io { path, source }
}

/// Recurse into `dir` unless it matches default or user-provided excludes.
fn visit_dir(
    project_root: &Path,
    dir: &Path,
    gitignore: &GitignoreRules,
    excludes: &ExcludeSet,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    if is_default_excluded_dir(dir)
        || gitignore.is_ignored(project_root, dir, true)
        || excludes.matches_path(project_root, dir)
    {
        return Ok(());
    }
    walk_dir(project_root, dir, gitignore, excludes, out)
}

/// Push `path` to `out` iff its extension is `rs`.
fn collect_if_rs(path: PathBuf, out: &mut Vec<PathBuf>) {
    if path.extension().is_some_and(|e| e == "rs") {
        out.push(path);
    }
}

fn is_default_excluded_dir(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    EXCLUDED_DIRS.contains(&name)
}

struct ExcludeSet {
    patterns: Vec<String>,
}

impl ExcludeSet {
    fn new(excludes: &[String]) -> Self {
        let patterns = excludes
            .iter()
            .map(|pattern| normalize_pattern(pattern))
            .filter(|pattern| !pattern.is_empty())
            .collect();
        Self { patterns }
    }

    fn matches_path(&self, project_root: &Path, path: &Path) -> bool {
        let rel = relative_path(project_root, path);
        if rel.is_empty() {
            return false;
        }

        let basename = path.file_name().and_then(|name| name.to_str());
        self.patterns
            .iter()
            .any(|pattern| pattern_matches(pattern, &rel, basename))
    }
}

fn relative_path(project_root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(project_root).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

fn normalize_pattern(pattern: &str) -> String {
    let mut normalized = pattern.trim().replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }
    while normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn pattern_matches(pattern: &str, rel: &str, basename: Option<&str>) -> bool {
    if is_component_pattern(pattern) {
        return basename == Some(pattern) || rel.split('/').any(|part| part == pattern);
    }

    if has_glob(pattern) {
        return wildcard_match(pattern, rel);
    }

    rel == pattern
        || rel
            .strip_prefix(pattern)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_component_pattern(pattern: &str) -> bool {
    !pattern.contains('/') && !has_glob(pattern)
}

fn has_glob(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?')
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star_pi, mut star_ti) = (None, 0usize);

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == b'?' || pattern[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            star_pi = Some(pi);
            pi += 1;
            star_ti = ti;
        } else if let Some(star) = star_pi {
            pi = star + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }

    pi == pattern.len()
}

struct GitignoreRules {
    rules: Vec<GitignoreRule>,
}

impl GitignoreRules {
    fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join(".gitignore");
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self { rules: Vec::new() });
            }
            Err(source) => {
                return Err(DecayError::Io {
                    path: path.display().to_string(),
                    source,
                });
            }
        };

        let mut rules = Vec::new();
        for line in content.lines() {
            if let Some(rule) = GitignoreRule::parse(line) {
                rules.push(rule);
            }
        }
        Ok(Self { rules })
    }

    fn is_ignored(&self, project_root: &Path, path: &Path, is_dir: bool) -> bool {
        let rel = relative_path(project_root, path);
        if rel.is_empty() {
            return false;
        }

        let basename = path.file_name().and_then(|name| name.to_str());
        let mut ignored = false;
        for rule in &self.rules {
            if rule.matches(&rel, basename, is_dir) {
                ignored = !rule.is_negated;
            }
        }
        ignored
    }
}

struct GitignoreRule {
    pattern: String,
    is_negated: bool,
    directory_only: bool,
    anchored: bool,
}

impl GitignoreRule {
    fn parse(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let (is_negated, raw) = match trimmed.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, trimmed),
        };
        let directory_only = raw.ends_with('/');
        let anchored = raw.starts_with('/');
        let pattern = normalize_pattern(raw.trim_start_matches('/'));
        if pattern.is_empty() {
            return None;
        }

        Some(Self {
            pattern,
            is_negated,
            directory_only,
            anchored,
        })
    }

    fn matches(&self, rel: &str, basename: Option<&str>, is_dir: bool) -> bool {
        if self.directory_only && !is_dir {
            return false;
        }

        if self.anchored {
            return path_rule_matches(&self.pattern, rel, basename);
        }

        if self.pattern.contains('/') {
            return path_rule_matches(&self.pattern, rel, basename)
                || rel
                    .split('/')
                    .enumerate()
                    .map(|(i, _)| rel.split('/').skip(i).collect::<Vec<_>>().join("/"))
                    .any(|suffix| path_rule_matches(&self.pattern, &suffix, basename));
        }

        basename_rule_matches(&self.pattern, rel, basename)
    }
}

fn path_rule_matches(pattern: &str, rel: &str, basename: Option<&str>) -> bool {
    if has_glob(pattern) {
        return wildcard_match(pattern, rel);
    }
    pattern_matches(pattern, rel, basename)
}

fn basename_rule_matches(pattern: &str, rel: &str, basename: Option<&str>) -> bool {
    if has_glob(pattern) {
        return rel.split('/').any(|part| wildcard_match(pattern, part))
            || wildcard_match(pattern, rel);
    }
    basename == Some(pattern) || rel.split('/').any(|part| part == pattern)
}
