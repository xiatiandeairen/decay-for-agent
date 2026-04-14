use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use log::debug;

/// A resolved file with its metadata.
pub struct FileEntry {
    pub rel_path: PathBuf,
    pub size: u64,
    pub depth: usize,
}

// --- Layer 2: Excluded directories ---
const EXCLUDED_DIRS: &[&str] = &[
    // VCS
    ".git",
    // Rust
    "target",
    // JavaScript/TypeScript
    "node_modules",
    "dist",
    ".next",
    ".nuxt",
    // Python
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    // iOS/macOS
    ".build",
    "DerivedData",
    "Pods",
    ".archives",
    // Java/Android
    "build",
    ".gradle",
    // IDE/tools
    ".idea",
    ".vscode",
    // decay
    ".sprint",
    ".decay",
];

// --- Layer 3: Excluded file extensions ---
const EXCLUDED_EXTENSIONS: &[&str] = &[
    // Compiled binaries
    "o", "a", "dylib", "so", "dll", "exe", // iOS/macOS build artifacts
    "pcm", "dia", "hmap", // Database files
    "mdb",  // Debug symbols
    "dsym", // Archives
    "zip", "tar", "gz", "tgz", "rar", // Images/media
    "png", "jpg", "jpeg", "gif", "ico", "bmp", "svg", "mp3", "mp4", "wav", "mov", "avi",
    // Fonts
    "ttf", "otf", "woff", "woff2",
];

/// Known source code extensions (for large file heuristic).
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "swift", "py", "ts", "tsx", "js", "jsx", "go", "java", "kt", "c", "cpp", "cc", "h",
    "hpp", "m", "mm", "rb", "sh", "bash", "toml", "yaml", "yml", "json", "xml", "html", "css",
    "scss", "md", "txt", "rst", "cfg", "ini", "conf", "sql", "graphql", "proto", "lock",
];

/// Max size for non-code files to be included (1MB).
const MAX_NON_CODE_SIZE: u64 = 1_048_576;

/// Resolve the list of files to scan using a three-layer funnel:
/// L1: Source selection (git > walkdir fallback)
/// L2: Directory exclusion
/// L3: File type filtering
pub fn resolve_files(project_path: &Path) -> Result<Vec<FileEntry>> {
    // L1: Try git first
    if let Some(files) = git_files(project_path)? {
        debug!("filter: using git ls-files ({} files)", files.len());
        return Ok(files);
    }

    // L1-P3: Fallback to walkdir
    debug!("filter: no git, falling back to walkdir");
    walk_files(project_path)
}

/// L1-P1: Use git ls-files to get tracked + untracked-but-not-ignored files.
fn git_files(project_path: &Path) -> Result<Option<Vec<FileEntry>>> {
    // Check if git is available and this is a git repo
    let tracked = Command::new("git")
        .args(["ls-files"])
        .current_dir(project_path)
        .output();

    let tracked = match tracked {
        Ok(output) if output.status.success() => output,
        _ => return Ok(None), // Not a git repo or git not available
    };

    let untracked = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(project_path)
        .output()
        .ok()
        .filter(|o| o.status.success());

    let mut files = Vec::new();
    let stdout = String::from_utf8_lossy(&tracked.stdout);

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let rel_path = PathBuf::from(line);
        if let Some(entry) = make_entry(project_path, &rel_path)? {
            files.push(entry);
        }
    }

    if let Some(untracked_output) = untracked {
        let stdout = String::from_utf8_lossy(&untracked_output.stdout);
        for line in stdout.lines() {
            if line.is_empty() {
                continue;
            }
            let rel_path = PathBuf::from(line);
            if let Some(entry) = make_entry(project_path, &rel_path)? {
                files.push(entry);
            }
        }
    }

    Ok(Some(files))
}

/// L1-P3: Walk the directory tree with L2 directory filtering.
fn walk_files(project_path: &Path) -> Result<Vec<FileEntry>> {
    use walkdir::WalkDir;

    let mut files = Vec::new();

    let walker = WalkDir::new(project_path)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                return !is_excluded_dir(entry.file_name().to_str().unwrap_or(""));
            }
            true
        });

    for entry in walker {
        let entry = entry.context("failed to read directory entry")?;
        if entry.file_type().is_dir() {
            continue;
        }

        let rel_path = entry
            .path()
            .strip_prefix(project_path)
            .unwrap_or(entry.path())
            .to_path_buf();

        if let Some(file_entry) = make_entry(project_path, &rel_path)? {
            files.push(file_entry);
        }
    }

    Ok(files)
}

/// Create a FileEntry after applying L2 (directory) and L3 (file type) filters.
/// Returns None if the file should be excluded.
fn make_entry(project_path: &Path, rel_path: &Path) -> Result<Option<FileEntry>> {
    // L2: Check if any path component is an excluded directory
    for component in rel_path.components() {
        if let std::path::Component::Normal(name) = component
            && is_excluded_dir(name.to_str().unwrap_or(""))
        {
            return Ok(None);
        }
    }

    // Get file metadata
    let abs_path = project_path.join(rel_path);
    let metadata = match abs_path.metadata() {
        Ok(m) => m,
        Err(_) => return Ok(None), // File might have been deleted
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    let size = metadata.len();

    // L3: Check excluded extensions
    if is_excluded_file(rel_path, size) {
        return Ok(None);
    }

    let depth = rel_path.components().count();

    Ok(Some(FileEntry {
        rel_path: rel_path.to_path_buf(),
        size,
        depth,
    }))
}

/// L2: Is this directory name in the exclusion list?
fn is_excluded_dir(name: &str) -> bool {
    EXCLUDED_DIRS.contains(&name)
}

/// L3: Should this file be excluded based on extension and size?
fn is_excluded_file(path: &Path, size: u64) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Excluded extension
    if EXCLUDED_EXTENSIONS.contains(&ext.as_str()) {
        return true;
    }

    // Large non-code file heuristic
    if size > MAX_NON_CODE_SIZE && !CODE_EXTENSIONS.contains(&ext.as_str()) {
        return true;
    }

    false
}

/// Count unique parent directories from file entries.
pub fn count_dirs(files: &[FileEntry]) -> usize {
    use std::collections::HashSet;
    let mut dirs = HashSet::new();
    for f in files {
        if let Some(parent) = f.rel_path.parent() {
            let mut p = parent;
            loop {
                if p.as_os_str().is_empty() {
                    break;
                }
                dirs.insert(p.to_path_buf());
                match p.parent() {
                    Some(pp) => p = pp,
                    None => break,
                }
            }
        }
    }
    dirs.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_excluded_dir() {
        assert!(is_excluded_dir(".git"));
        assert!(is_excluded_dir("node_modules"));
        assert!(is_excluded_dir(".build"));
        assert!(is_excluded_dir("DerivedData"));
        assert!(!is_excluded_dir("src"));
    }

    #[test]
    fn test_excluded_file() {
        assert!(is_excluded_file(Path::new("lib.a"), 100));
        assert!(is_excluded_file(Path::new("icon.png"), 100));
        assert!(!is_excluded_file(Path::new("main.rs"), 100));
        // Large non-code file
        assert!(is_excluded_file(Path::new("data.bin"), 2_000_000));
        // Large code file is NOT excluded
        assert!(!is_excluded_file(Path::new("big.rs"), 2_000_000));
    }

    #[test]
    fn test_walk_excludes_build_dirs() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("main.rs"), "fn main() {}")?;
        fs::create_dir_all(dir.path().join(".build/debug"))?;
        fs::write(dir.path().join(".build/debug/app"), "binary")?;
        fs::create_dir_all(dir.path().join("DerivedData/cache"))?;
        fs::write(dir.path().join("DerivedData/cache/data"), "cache")?;

        let files = walk_files(dir.path())?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, PathBuf::from("main.rs"));
        Ok(())
    }

    #[test]
    fn test_walk_excludes_binary_extensions() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("main.rs"), "fn main() {}")?;
        fs::write(dir.path().join("lib.a"), "archive")?;
        fs::write(dir.path().join("icon.png"), "image")?;

        let files = walk_files(dir.path())?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, PathBuf::from("main.rs"));
        Ok(())
    }

    #[test]
    fn test_git_files_returns_none_for_non_git() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("main.rs"), "fn main() {}")?;
        let result = git_files(dir.path())?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_count_dirs() {
        let files = vec![
            FileEntry {
                rel_path: PathBuf::from("src/main.rs"),
                size: 100,
                depth: 2,
            },
            FileEntry {
                rel_path: PathBuf::from("src/lib.rs"),
                size: 100,
                depth: 2,
            },
            FileEntry {
                rel_path: PathBuf::from("src/sub/mod.rs"),
                size: 100,
                depth: 3,
            },
        ];
        assert_eq!(count_dirs(&files), 2); // src, src/sub
    }
}
