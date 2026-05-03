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
    let mut out = Vec::new();
    walk_dir(project_root, &mut out)?;
    Ok(out)
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in read_dir(dir)? {
        let entry = entry.map_err(io_err(dir))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(io_err(&path))?;

        if file_type.is_dir() {
            visit_dir(&path, out)?;
        } else if file_type.is_file() {
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

/// Recurse into `dir` unless its basename is in `EXCLUDED_DIRS`.
fn visit_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if EXCLUDED_DIRS.contains(&name) {
        return Ok(());
    }
    walk_dir(dir, out)
}

/// Push `path` to `out` iff its extension is `rs`.
fn collect_if_rs(path: PathBuf, out: &mut Vec<PathBuf>) {
    if path.extension().is_some_and(|e| e == "rs") {
        out.push(path);
    }
}
