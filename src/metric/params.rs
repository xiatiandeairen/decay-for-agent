//! Parameter count metric.
//!
//! Counts the named children of the enclosing `function_item`'s `parameters`
//! node — i.e. concrete parameters in the signature.
//!
//! Counted: each regular parameter; `self` / `&self` / `&mut self` (counted as 1).
//! Not counted: generic parameters (`<T>`), `where` clauses.
//!
//! The public API receives `body_range` (the function body), but parameters
//! live in the signature, not the body. We resolve the enclosing
//! `function_item` by descending to the body node via byte range and walking
//! ancestors, then read its `parameters` child.

use tree_sitter::{Node, Range, Tree};

/// Compute the parameter count for the function whose body covers `body_range`.
///
/// Returns 0 when the enclosing `function_item` cannot be located or has no
/// `parameters` child — defensive fallbacks; the parser only emits ranges for
/// extracted `function_item`s, so in practice the lookup succeeds.
pub fn compute(tree: &Tree, _source: &str, body_range: Range) -> u32 {
    let Some(body_node) = tree
        .root_node()
        .descendant_for_byte_range(body_range.start_byte, body_range.end_byte)
    else {
        return 0;
    };

    let Some(fn_item) = ancestor_function_item(body_node) else {
        return 0;
    };

    let Some(params_node) = fn_item.child_by_field_name("parameters") else {
        return 0;
    };

    // Named children of `parameters` are: `self_parameter` and `parameter`.
    // Punctuation (`,`, `(`, `)`) is anonymous and skipped by `named_children`.
    // Generics (`<T>`) live under `type_parameters`, a sibling of `parameters`,
    // and never appear here.
    let mut cursor = params_node.walk();
    params_node.named_children(&mut cursor).count() as u32
}

fn ancestor_function_item(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node;
    loop {
        if cur.kind() == "function_item" {
            return Some(cur);
        }
        cur = cur.parent()?;
    }
}
