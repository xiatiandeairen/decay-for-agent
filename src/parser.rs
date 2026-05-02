use std::path::Path;

use crate::error::Result;
use crate::types::Function;

pub struct ParsedFile {
    pub tree: tree_sitter::Tree,
    pub source: String,
    pub funcs: Vec<ParsedFunc>,
}

pub struct ParsedFunc {
    pub function: Function, // metrics 字段此时 zeroed
    pub body_range: tree_sitter::Range,
}

#[allow(clippy::missing_errors_doc)]
pub fn parse_file(_path: &Path, _project_root: &Path) -> Result<ParsedFile> {
    todo!()
}

pub fn _stub() {}
