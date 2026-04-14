use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::debug;

use crate::config::DecayConfig;
use crate::filter::FileEntry;

/// Context shared across all filter stages.
pub struct FilterContext {
    pub project_path: PathBuf,
    pub extra_exclude_dirs: Vec<String>,
    pub extra_exclude_extensions: Vec<String>,
    pub language_override: Option<Vec<String>>,
}

impl FilterContext {
    /// Build context from DecayConfig.
    pub fn from_config(project_path: &Path, config: &DecayConfig) -> Self {
        Self {
            project_path: project_path.to_path_buf(),
            extra_exclude_dirs: config.exclude_dirs.clone(),
            extra_exclude_extensions: config.exclude_extensions.clone(),
            language_override: config.languages.clone(),
        }
    }
}

/// A stage in the filter pipeline.
pub trait FilterStage: Send + Sync {
    fn name(&self) -> &'static str;
    fn filter(&self, files: Vec<FileEntry>, ctx: &FilterContext) -> Vec<FileEntry>;
}

/// Run all filter stages in sequence.
pub fn run_pipeline(files: Vec<FileEntry>, ctx: &FilterContext) -> Vec<FileEntry> {
    let stages: Vec<Box<dyn FilterStage>> = vec![
        Box::new(DirExclusion),
        Box::new(FileTypeFilter),
        Box::new(LanguageFilter),
    ];

    let mut result = files;
    for stage in &stages {
        let before = result.len();
        result = stage.filter(result, ctx);
        let after = result.len();
        if before != after {
            debug!("filter {}: {} → {} files", stage.name(), before, after);
        }
    }
    result
}

// ============================================================
// L2: Directory Exclusion
// ============================================================

const EXCLUDED_DIRS: &[&str] = &[
    // VCS
    ".git",
    // Rust
    "target",
    // JavaScript/TypeScript
    "node_modules", "dist", ".next", ".nuxt",
    // Python
    "__pycache__", ".venv", "venv", ".tox",
    // iOS/macOS
    ".build", "DerivedData", "Pods", ".archives",
    // Java/Android
    "build", ".gradle",
    // IDE/tools
    ".idea", ".vscode",
    // decay
    ".sprint", ".decay",
    // Vendor / third-party
    "vendor", "vendors", "third_party", "third-party", "ThirdParty",
    "External", "Externals", "Carthage",
];

pub struct DirExclusion;

impl FilterStage for DirExclusion {
    fn name(&self) -> &'static str {
        "dir_exclusion"
    }

    fn filter(&self, files: Vec<FileEntry>, ctx: &FilterContext) -> Vec<FileEntry> {
        files
            .into_iter()
            .filter(|f| {
                for component in f.rel_path.components() {
                    if let std::path::Component::Normal(name) = component {
                        let name_str = name.to_str().unwrap_or("");
                        if EXCLUDED_DIRS.contains(&name_str) {
                            return false;
                        }
                        if ctx.extra_exclude_dirs.iter().any(|d| d == name_str) {
                            return false;
                        }
                    }
                }
                true
            })
            .collect()
    }
}

// ============================================================
// L3: File Type Filter
// ============================================================

const EXCLUDED_EXTENSIONS: &[&str] = &[
    // Compiled binaries
    "o", "a", "dylib", "so", "dll", "exe",
    // iOS/macOS build artifacts
    "pcm", "dia", "hmap",
    // Database files
    "mdb",
    // Debug symbols
    "dsym",
    // Archives
    "zip", "tar", "gz", "tgz", "rar",
    // Images/media
    "png", "jpg", "jpeg", "gif", "ico", "bmp", "svg",
    "mp3", "mp4", "wav", "mov", "avi",
    // Fonts
    "ttf", "otf", "woff", "woff2",
];

/// Max size for non-code files (1MB).
const MAX_NON_CODE_SIZE: u64 = 1_048_576;

pub struct FileTypeFilter;

impl FilterStage for FileTypeFilter {
    fn name(&self) -> &'static str {
        "file_type"
    }

    fn filter(&self, files: Vec<FileEntry>, ctx: &FilterContext) -> Vec<FileEntry> {
        files
            .into_iter()
            .filter(|f| {
                let ext = f.rel_path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                // Excluded extension
                if EXCLUDED_EXTENSIONS.contains(&ext.as_str()) {
                    return false;
                }
                if ctx.extra_exclude_extensions.iter().any(|e| e == &ext) {
                    return false;
                }

                // Large non-source file
                if f.size > MAX_NON_CODE_SIZE && !is_any_language_ext(&ext) {
                    return false;
                }

                true
            })
            .collect()
    }
}

// ============================================================
// L4: Language Filter
// ============================================================

/// Language group: a named set of file extensions.
pub struct LanguageGroup {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
}

pub const LANGUAGE_GROUPS: &[LanguageGroup] = &[
    LanguageGroup { name: "rust", extensions: &["rs"] },
    LanguageGroup { name: "swift", extensions: &["swift"] },
    LanguageGroup { name: "objc", extensions: &["m", "mm", "h"] },
    LanguageGroup { name: "python", extensions: &["py"] },
    LanguageGroup { name: "javascript", extensions: &["js", "jsx", "mjs"] },
    LanguageGroup { name: "typescript", extensions: &["ts", "tsx"] },
    LanguageGroup { name: "go", extensions: &["go"] },
    LanguageGroup { name: "java", extensions: &["java"] },
    LanguageGroup { name: "kotlin", extensions: &["kt", "kts"] },
    LanguageGroup { name: "ruby", extensions: &["rb"] },
    LanguageGroup { name: "cpp", extensions: &["c", "cpp", "cc", "hpp"] },
    LanguageGroup { name: "csharp", extensions: &["cs"] },
    LanguageGroup { name: "php", extensions: &["php"] },
    LanguageGroup { name: "shell", extensions: &["sh", "bash", "zsh"] },
];

/// Minimum share for a language group to be considered "primary".
const PRIMARY_LANGUAGE_THRESHOLD: f64 = 0.10;

pub struct LanguageFilter;

impl FilterStage for LanguageFilter {
    fn name(&self) -> &'static str {
        "language"
    }

    fn filter(&self, files: Vec<FileEntry>, ctx: &FilterContext) -> Vec<FileEntry> {
        if files.is_empty() {
            return files;
        }

        // If override provided, use it directly
        if let Some(ref overrides) = ctx.language_override {
            let allowed: Vec<&str> = overrides.iter().map(|s| s.as_str()).collect();
            return files
                .into_iter()
                .filter(|f| {
                    let ext = file_ext(f);
                    ext_in_groups(&ext, &allowed)
                })
                .collect();
        }

        // Count files per language group
        let mut group_counts: HashMap<&str, usize> = HashMap::new();
        let mut total_with_group = 0;

        for f in &files {
            let ext = file_ext(f);
            if let Some(group) = ext_to_group(&ext) {
                *group_counts.entry(group).or_default() += 1;
                total_with_group += 1;
            }
        }

        // If no files belong to any language group, return all (don't filter)
        if total_with_group == 0 {
            debug!("language filter: no recognized language files, skipping");
            return files;
        }

        // Select primary languages (>10% share)
        let primary: Vec<&str> = group_counts
            .iter()
            .filter(|(_, count)| **count as f64 / total_with_group as f64 >= PRIMARY_LANGUAGE_THRESHOLD)
            .map(|(name, _)| *name)
            .collect();

        debug!(
            "language filter: primary languages = {:?} (from {} files with known language)",
            primary, total_with_group
        );

        // Keep only files whose extension belongs to a primary language group
        files
            .into_iter()
            .filter(|f| {
                let ext = file_ext(f);
                ext_in_groups(&ext, &primary)
            })
            .collect()
    }
}

fn file_ext(f: &FileEntry) -> String {
    f.rel_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn ext_to_group(ext: &str) -> Option<&'static str> {
    for group in LANGUAGE_GROUPS {
        if group.extensions.contains(&ext) {
            return Some(group.name);
        }
    }
    None
}

fn ext_in_groups(ext: &str, group_names: &[&str]) -> bool {
    for group in LANGUAGE_GROUPS {
        if group_names.contains(&group.name) && group.extensions.contains(&ext) {
            return true;
        }
    }
    false
}

fn is_any_language_ext(ext: &str) -> bool {
    ext_to_group(ext).is_some()
}

/// Detect primary languages from a list of file paths.
/// Returns language group names with ≥10% share.
pub fn detect_primary_languages(paths: &[String]) -> Vec<String> {
    let mut group_counts: HashMap<&str, usize> = HashMap::new();
    let mut total = 0;

    for path in paths {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(group) = ext_to_group(&ext) {
            *group_counts.entry(group).or_default() += 1;
            total += 1;
        }
    }

    if total == 0 {
        return vec![];
    }

    group_counts
        .iter()
        .filter(|(_, count)| **count as f64 / total as f64 >= PRIMARY_LANGUAGE_THRESHOLD)
        .map(|(name, _)| name.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(path: &str, size: u64) -> FileEntry {
        FileEntry {
            rel_path: PathBuf::from(path),
            size,
            depth: path.matches('/').count() + 1,
        }
    }

    fn default_ctx() -> FilterContext {
        FilterContext::from_config(Path::new("/tmp/test"), &DecayConfig::default())
    }

    // --- DirExclusion tests ---

    #[test]
    fn test_dir_exclusion_basic() {
        let files = vec![
            make_file("src/main.rs", 100),
            make_file("node_modules/lib/index.js", 100),
            make_file("vendor/lib.go", 100),
            make_file(".git/config", 100),
        ];
        let result = DirExclusion.filter(files, &default_ctx());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rel_path, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_dir_exclusion_vendor() {
        let files = vec![
            make_file("src/app.swift", 100),
            make_file("ThirdParty/DoKit/DoKit.m", 100),
            make_file("External/FMDB/FMDB.m", 100),
            make_file("Carthage/Build/lib.a", 100),
        ];
        let result = DirExclusion.filter(files, &default_ctx());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rel_path, PathBuf::from("src/app.swift"));
    }

    #[test]
    fn test_dir_exclusion_extra() {
        let mut ctx = default_ctx();
        ctx.extra_exclude_dirs.push("custom_vendor".to_string());
        let files = vec![
            make_file("src/main.rs", 100),
            make_file("custom_vendor/lib.rs", 100),
        ];
        let result = DirExclusion.filter(files, &ctx);
        assert_eq!(result.len(), 1);
    }

    // --- FileTypeFilter tests ---

    #[test]
    fn test_file_type_filter() {
        let files = vec![
            make_file("src/main.rs", 100),
            make_file("icon.png", 100),
            make_file("lib.a", 100),
        ];
        let result = FileTypeFilter.filter(files, &default_ctx());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rel_path, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_large_non_code_excluded() {
        let files = vec![
            make_file("data.bin", 2_000_000),
            make_file("big.rs", 2_000_000),
        ];
        let result = FileTypeFilter.filter(files, &default_ctx());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rel_path, PathBuf::from("big.rs"));
    }

    // --- LanguageFilter tests ---

    #[test]
    fn test_language_filter_detects_primary() {
        let mut files = Vec::new();
        // 80 Swift files, 20 ObjC files, 5 JSON files, 2 CSS files
        for i in 0..80 {
            files.push(make_file(&format!("src/file{i}.swift"), 100));
        }
        for i in 0..20 {
            files.push(make_file(&format!("src/obj{i}.m"), 100));
        }
        for i in 0..5 {
            files.push(make_file(&format!("res/data{i}.json"), 100));
        }
        files.push(make_file("style.css", 100));
        files.push(make_file("readme.md", 100));

        let result = LanguageFilter.filter(files, &default_ctx());
        // Should keep swift + objc, drop json/css/md
        assert_eq!(result.len(), 100);
        assert!(result.iter().all(|f| {
            let ext = file_ext(f);
            ext == "swift" || ext == "m"
        }));
    }

    #[test]
    fn test_language_filter_rust_project() {
        let files = vec![
            make_file("src/main.rs", 100),
            make_file("src/lib.rs", 100),
            make_file("Cargo.toml", 100),
            make_file("README.md", 100),
            make_file("config.json", 100),
        ];
        let result = LanguageFilter.filter(files, &default_ctx());
        assert_eq!(result.len(), 2); // only .rs files
    }

    #[test]
    fn test_language_filter_override() {
        let mut ctx = default_ctx();
        ctx.language_override = Some(vec!["python".to_string()]);
        let files = vec![
            make_file("app.py", 100),
            make_file("main.rs", 100),
            make_file("index.js", 100),
        ];
        let result = LanguageFilter.filter(files, &ctx);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rel_path, PathBuf::from("app.py"));
    }

    #[test]
    fn test_language_filter_no_known_languages() {
        let files = vec![
            make_file("data.csv", 100),
            make_file("config.yaml", 100),
        ];
        let result = LanguageFilter.filter(files, &default_ctx());
        // No known languages → don't filter
        assert_eq!(result.len(), 2);
    }

    // --- Pipeline tests ---

    #[test]
    fn test_full_pipeline() {
        let files = vec![
            make_file("src/main.rs", 100),
            make_file("src/lib.rs", 100),
            make_file("node_modules/pkg/index.js", 100),
            make_file("vendor/third.go", 100),
            make_file("icon.png", 100),
            make_file("README.md", 500),
            make_file("Cargo.toml", 200),
        ];
        let ctx = FilterContext::from_config(Path::new("/tmp/test"), &DecayConfig::default());
        let result = run_pipeline(files, &ctx);
        // node_modules → L2 excluded
        // vendor → L2 excluded
        // icon.png → L3 excluded
        // README.md, Cargo.toml → L4 excluded (not rust)
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|f| file_ext(f) == "rs"));
    }
}
