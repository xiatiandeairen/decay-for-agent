use std::fs;
use std::path::Path;

use tree_sitter::{Node, Parser, Range, Tree};

use crate::error::{DecayError, Result};
use crate::types::{Function, Metrics};

pub struct ParsedFile {
    pub tree: Tree,
    pub source: String,
    pub funcs: Vec<ParsedFunc>,
}

pub struct ParsedFunc {
    pub function: Function, // metrics zeroed; signature_hash 0; pipeline fills both
    pub body_range: Range,
}

/// Parse a Rust source file and extract every concrete `function_item`.
///
/// `function_signature_item` (trait method declarations without a body) and
/// `closure_expression` are intentionally skipped per §2.11.
///
/// `metrics` and `signature_hash` on each `Function` are left zeroed; the
/// pipeline fills them in later stages.
///
/// Returns `DecayError::Io` on read failure, `DecayError::Parse` when
/// tree-sitter reports `has_error()` (we still surface even if a partial tree
/// is returned, because malformed source produces unreliable function shapes).
pub fn parse_file(path: &Path, project_root: &Path) -> Result<ParsedFile> {
    let path_str = path.display().to_string();

    let source = fs::read_to_string(path).map_err(|source| DecayError::Io {
        path: path_str.clone(),
        source,
    })?;

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
        .map_err(|e| DecayError::Parse {
            path: path_str.clone(),
            message: format!("failed to load Rust grammar: {e}"),
        })?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| DecayError::Parse {
            path: path_str.clone(),
            message: "tree-sitter returned no tree".to_string(),
        })?;

    if tree.root_node().has_error() {
        return Err(DecayError::Parse {
            path: path_str,
            message: "source contains syntax errors".to_string(),
        });
    }

    let rel_file = relative_path(path, project_root);
    let funcs = collect_functions(&tree, &source, &rel_file);

    Ok(ParsedFile {
        tree,
        source,
        funcs,
    })
}

/// Convert `path` to a project-relative string with forward slashes.
/// Falls back to the absolute display when stripping fails (defensive — the
/// pipeline always passes paths produced by `walk` rooted at `project_root`).
fn relative_path(path: &Path, project_root: &Path) -> String {
    let rel = path.strip_prefix(project_root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn collect_functions(tree: &Tree, source: &str, rel_file: &str) -> Vec<ParsedFunc> {
    let mut out = Vec::new();
    let mut cursor = tree.walk();
    let root = tree.root_node();
    visit(root, source, rel_file, &mut cursor, &mut out);
    out
}

/// Recursively descend, picking up every `function_item`.
/// `function_signature_item` and `closure_expression` are not recursed into for
/// extraction — but we still descend into their bodies because nested
/// `function_item`s could appear (e.g. a fn defined inside a closure body).
fn visit<'a>(
    node: Node<'a>,
    source: &str,
    rel_file: &str,
    cursor: &mut tree_sitter::TreeCursor<'a>,
    out: &mut Vec<ParsedFunc>,
) {
    if node.kind() == "function_item" {
        if let Some(parsed) = extract_function(node, source, rel_file) {
            out.push(parsed);
        }
    }

    // Always descend; nested function_items may exist anywhere.
    for child in node.children(cursor) {
        let mut child_cursor = child.walk();
        visit(child, source, rel_file, &mut child_cursor, out);
    }
}

fn extract_function(node: Node<'_>, source: &str, rel_file: &str) -> Option<ParsedFunc> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?.to_string();

    let body_node = node.child_by_field_name("body")?;
    let body_range = body_node.range();

    let params_node = node.child_by_field_name("parameters")?;
    let param_types = collect_param_types(params_node, source);

    // tree-sitter Point.row is 0-indexed; spec wants 1-indexed lines.
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;

    let function = Function {
        file: rel_file.to_string(),
        name,
        start_line,
        end_line,
        param_types,
        signature_hash: 0,
        metrics: Metrics {
            nesting: 0,
            cyclomatic: 0,
            cognitive: 0,
            params: 0,
        },
    };

    Some(ParsedFunc {
        function,
        body_range,
    })
}

fn collect_param_types(params_node: Node<'_>, source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.named_children(&mut cursor) {
        match child.kind() {
            "self_parameter" => {
                if let Some(text) = node_text(child, source) {
                    out.push(normalize_self(text));
                }
            }
            "parameter" => {
                if let Some(ty_node) = child.child_by_field_name("type") {
                    if let Some(text) = node_text(ty_node, source) {
                        out.push(normalize_type(text));
                    }
                }
            }
            // Skip attribute_item, variadic_parameter, raw `_type` (rare).
            _ => {}
        }
    }
    out
}

fn node_text<'a>(node: Node<'_>, source: &'a str) -> Option<&'a str> {
    source.get(node.byte_range())
}

/// Normalize a self parameter source slice to one of:
/// `self`, `&self`, `&mut self`. Any lifetime (e.g. `&'a self`) is stripped.
fn normalize_self(text: &str) -> String {
    let no_lifetime = strip_lifetimes(text);
    let stripped: String = no_lifetime.chars().filter(|c| !c.is_whitespace()).collect();
    // Coerce to one of three canonical forms; defensive against odd shapes.
    if stripped.starts_with("&mut") {
        "&mut self".to_string()
    } else if stripped.starts_with('&') {
        "&self".to_string()
    } else {
        "self".to_string()
    }
}

/// Normalize a regular parameter type per §2.10:
/// remove lifetimes (must come first — lifetime token boundary is whitespace
/// or punctuation, so doing it after whitespace-removal would over-eat
/// adjacent ident chars), then remove all whitespace.
fn normalize_type(text: &str) -> String {
    let no_lt = strip_lifetimes(text);
    no_lt.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Remove Rust lifetime tokens (`'a`, `'static`, ...) from a source slice.
///
/// A lifetime is `'` followed by an identifier; it terminates at the first
/// non-ident char (which may be whitespace, `,`, `>`, etc.). The ident itself
/// is dropped along with the apostrophe; the terminator is preserved.
///
/// Why: callers either feed raw source (`normalize_type`) or an already-merged
/// snippet. Acting on raw source is the only way to use whitespace as a
/// lifetime boundary — if whitespace is stripped first, `&'a str` collapses
/// to `&'astr` and the ident eats the type name.
///
/// We intentionally keep this regex-free to avoid adding a dep just for this
/// (§2.13 dependency list is locked).
fn strip_lifetimes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\'' && i + 1 < bytes.len() && is_ident_start(bytes[i + 1]) {
            // Skip the apostrophe + the following ident chars; leave terminator.
            i += 1;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
        } else {
            out.push(c as char);
            i += 1;
        }
    }
    out
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
