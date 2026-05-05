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
    pub is_test_like: bool,
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
    visit(root, source, rel_file, "", false, &mut cursor, &mut out);
    out
}

/// Recursively descend, picking up every `function_item` and tagging it with
/// the nearest enclosing `impl_item`'s context (empty string for free fns).
///
/// `function_signature_item` and `closure_expression` are not recursed into for
/// extraction — but we still descend into their bodies because nested
/// `function_item`s could appear (e.g. a fn defined inside a closure body).
///
/// When entering an `impl_item` we extract its context once and pass it down.
/// If extraction fails (malformed AST under tree-sitter ERROR recovery), we
/// log a warn and skip the entire impl block — silent fallback to empty
/// context would re-introduce fingerprint collisions, defeating the purpose.
fn visit<'a>(
    node: Node<'a>,
    source: &str,
    rel_file: &str,
    impl_context: &str,
    in_test_context: bool,
    cursor: &mut tree_sitter::TreeCursor<'a>,
    out: &mut Vec<ParsedFunc>,
) {
    if node.kind() == "impl_item" {
        match extract_impl_context(node, source) {
            Some(ctx) => {
                for child in node.children(cursor) {
                    let mut child_cursor = child.walk();
                    visit(
                        child,
                        source,
                        rel_file,
                        &ctx,
                        in_test_context,
                        &mut child_cursor,
                        out,
                    );
                }
            }
            None => {
                log::warn!(
                    "{}:{}: cannot extract impl context, skipping impl block",
                    rel_file,
                    node.start_position().row + 1
                );
            }
        }
        return;
    }

    if node.kind() == "mod_item" {
        let nested_test_context = in_test_context || is_test_module(node, source);
        for child in node.children(cursor) {
            let mut child_cursor = child.walk();
            visit(
                child,
                source,
                rel_file,
                impl_context,
                nested_test_context,
                &mut child_cursor,
                out,
            );
        }
        return;
    }

    if node.kind() == "function_item" {
        if let Some(parsed) =
            extract_function(node, source, rel_file, impl_context, in_test_context)
        {
            out.push(parsed);
        }
    }

    // Always descend; nested function_items may exist anywhere.
    for child in node.children(cursor) {
        let mut child_cursor = child.walk();
        visit(
            child,
            source,
            rel_file,
            impl_context,
            in_test_context,
            &mut child_cursor,
            out,
        );
    }
}

fn extract_function(
    node: Node<'_>,
    source: &str,
    rel_file: &str,
    impl_context: &str,
    in_test_context: bool,
) -> Option<ParsedFunc> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?.to_string();
    let attrs = collect_attribute_texts(node, source);
    let cfg_context = extract_cfg_context(&attrs);
    let is_test_like = in_test_context || is_test_attr_set(&attrs) || cfg_context_is_test(&cfg_context);

    let body_node = node.child_by_field_name("body")?;
    let body_range = body_node.range();

    let params_node = node.child_by_field_name("parameters")?;
    let param_types = collect_param_types(params_node, source);

    // tree-sitter Point.row is 0-indexed; spec wants 1-indexed lines.
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;

    let function = Function {
        file: rel_file.to_string(),
        impl_context: impl_context.to_string(),
        cfg_context,
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
            statement_count: 0,
            max_condition_ops: 0,
            mutable_bindings: 0,
        },
    };

    Some(ParsedFunc {
        function,
        body_range,
        is_test_like,
    })
}

/// Extract a canonical impl-block context string for fingerprint disambiguation.
///
/// Returns `None` only when tree-sitter's ERROR recovery left the impl_item
/// without a readable `type` field (rare but possible on malformed source).
///
/// Forms:
/// - `impl Foo { ... }`              -> `"Foo"`
/// - `impl<T> Foo<T> { ... }`        -> `"Foo"`            (generics stripped)
/// - `impl Trait for Foo { ... }`    -> `"Trait for Foo"`
/// - `impl<'a, T> Display for Foo<T>` -> `"Display for Foo"`
fn extract_impl_context(impl_node: Node<'_>, source: &str) -> Option<String> {
    let type_node = impl_node.child_by_field_name("type")?;
    let self_type = strip_generic_args(node_text(type_node, source)?);
    let self_type = strip_lifetimes(&self_type);
    let self_type: String = self_type.chars().filter(|c| !c.is_whitespace()).collect();
    if self_type.is_empty() {
        return None;
    }

    if let Some(trait_node) = impl_node.child_by_field_name("trait") {
        let trait_name = strip_generic_args(node_text(trait_node, source)?);
        let trait_name = strip_lifetimes(&trait_name);
        let trait_name: String = trait_name.chars().filter(|c| !c.is_whitespace()).collect();
        if !trait_name.is_empty() {
            return Some(format!("{trait_name} for {self_type}"));
        }
    }
    Some(self_type)
}

/// Extract a canonical context string from contiguous preceding `#[cfg(...)]`
/// attributes attached to this function.
///
/// Only plain `#[cfg(...)]` participates in identity. Other attributes such
/// as `#[inline]`, `#[allow]`, doc comments, or `cfg_attr(...)` are ignored so
/// formatting and lint controls do not perturb fingerprint stability.
fn collect_attribute_texts(node: Node<'_>, source: &str) -> Vec<String> {
    let mut attrs: Vec<String> = Vec::new();
    let mut cursor = node.prev_named_sibling();

    while let Some(node) = cursor {
        if node.kind() != "attribute_item" {
            break;
        }
        if let Some(text) = node_text(node, source) {
            attrs.push(text.to_string());
        }
        cursor = node.prev_named_sibling();
    }

    attrs.reverse();
    attrs
}

fn extract_cfg_context(attrs: &[String]) -> String {
    attrs.iter()
        .filter_map(|text| canonicalize_cfg_attr(text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_test_module(node: Node<'_>, source: &str) -> bool {
    let attrs = collect_attribute_texts(node, source);
    if is_test_attr_set(&attrs) {
        return true;
    }
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Some(name) = node_text(name_node, source) {
            if name == "tests" {
                return true;
            }
        }
    }
    false
}

fn is_test_attr_set(attrs: &[String]) -> bool {
    attrs.iter().any(|text| {
        let normalized: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        normalized == "#[test]"
            || normalized.ends_with("::test]")
            || cfg_attr_is_test(&normalized)
    })
}

fn cfg_context_is_test(cfg_context: &str) -> bool {
    cfg_context
        .lines()
        .any(|line| cfg_attr_is_test(line))
}

fn cfg_attr_is_test(normalized: &str) -> bool {
    normalized == "#[cfg(test)]"
        || normalized.contains("(test,")
        || normalized.contains(",test,")
        || normalized.contains(",test)")
        || normalized.contains("(test)")
}

fn canonicalize_cfg_attr(text: &str) -> Option<String> {
    let normalized: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if normalized.starts_with("#[cfg(") && normalized.ends_with(")]") {
        return Some(normalized);
    }
    None
}


/// Truncate a type expression at the first `<` so generic parameters do not
/// participate in the impl_context. `Foo<T>` and `Foo<U>` should map to the
/// same context — fingerprint already disambiguates by file+name+params.
fn strip_generic_args(s: &str) -> String {
    match s.find('<') {
        Some(i) => s[..i].to_string(),
        None => s.to_string(),
    }
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
