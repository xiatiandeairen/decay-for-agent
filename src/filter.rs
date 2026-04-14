use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use log::debug;

use crate::filter_pipeline::{self, FilterContext};

/// A resolved file with its metadata.
pub struct FileEntry {
    pub rel_path: PathBuf,
    pub size: u64,
    pub depth: usize,
}

/// Resolve the list of files to scan using:
/// L1: Source selection (git > walkdir fallback)
/// L2-L4: Filter pipeline (dir exclusion → file type → language)
pub fn resolve_files(project_path: &Path) -> Result<Vec<FileEntry>> {
    // L1: Collect raw files
    let raw = if let Some(files) = git_files(project_path)? {
        debug!("filter: using git ls-files ({} files)", files.len());
        files
    } else {
        debug!("filter: no git, falling back to walkdir");
        walk_files(project_path)?
    };

    // L2-L4: Run filter pipeline
    let ctx = FilterContext::new(project_path);
    let filtered = filter_pipeline::run_pipeline(raw, &ctx);
    debug!("filter: pipeline result: {} files", filtered.len());

    Ok(filtered)
}

/// L1-P1: Use git ls-files to get tracked + untracked-but-not-ignored files.
fn git_files(project_path: &Path) -> Result<Option<Vec<FileEntry>>> {
    let tracked = Command::new("git")
        .args(["ls-files"])
        .current_dir(project_path)
        .output();

    let tracked = match tracked {
        Ok(output) if output.status.success() => output,
        _ => return Ok(None),
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

/// L1 fallback: Walk the directory tree (no filtering, pipeline handles it).
fn walk_files(project_path: &Path) -> Result<Vec<FileEntry>> {
    use walkdir::WalkDir;

    let mut files = Vec::new();

    for entry in WalkDir::new(project_path).into_iter().filter_map(|e| e.ok()) {
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

/// Create a FileEntry (L1 only — no filtering, just metadata).
fn make_entry(project_path: &Path, rel_path: &Path) -> Result<Option<FileEntry>> {
    let abs_path = project_path.join(rel_path);
    let metadata = match abs_path.metadata() {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    let size = metadata.len();
    let depth = rel_path.components().count();

    Ok(Some(FileEntry {
        rel_path: rel_path.to_path_buf(),
        size,
        depth,
    }))
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
    fn test_walk_and_filter() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("main.rs"), "fn main() {}")?;
        fs::create_dir_all(dir.path().join(".build/debug"))?;
        fs::write(dir.path().join(".build/debug/app"), "binary")?;
        fs::create_dir_all(dir.path().join("DerivedData/cache"))?;
        fs::write(dir.path().join("DerivedData/cache/data"), "cache")?;
        fs::write(dir.path().join("icon.png"), "image")?;
        fs::write(dir.path().join("readme.md"), "# hello")?;

        let files = resolve_files(dir.path())?;
        // .build → L2 excluded, DerivedData → L2 excluded
        // icon.png → L3 excluded, readme.md → L4 excluded (not rust)
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, PathBuf::from("main.rs"));
        Ok(())
    }

    #[test]
    fn test_vendor_excluded() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("app.swift"), "import UIKit")?;
        fs::create_dir_all(dir.path().join("vendor/DoKit"))?;
        fs::write(dir.path().join("vendor/DoKit/DoKit.m"), "objc")?;

        let files = resolve_files(dir.path())?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, PathBuf::from("app.swift"));
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
            FileEntry { rel_path: PathBuf::from("src/main.rs"), size: 100, depth: 2 },
            FileEntry { rel_path: PathBuf::from("src/lib.rs"), size: 100, depth: 2 },
            FileEntry { rel_path: PathBuf::from("src/sub/mod.rs"), size: 100, depth: 3 },
        ];
        assert_eq!(count_dirs(&files), 2);
    }
}
