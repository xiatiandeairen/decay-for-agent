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
    let entries = fs::read_dir(dir).map_err(|source| DecayError::Io {
        path: dir.display().to_string(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| DecayError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| DecayError::Io {
            path: path.display().to_string(),
            source,
        })?;

        if file_type.is_dir() {
            // Skip excluded dirs at any depth (match by basename).
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if EXCLUDED_DIRS.contains(&name) {
                continue;
            }
            walk_dir(&path, out)?;
        } else if file_type.is_file() && path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
        // Symlinks and other entry kinds are ignored.
    }
    Ok(())
}
