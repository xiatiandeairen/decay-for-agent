//! Maximum control-flow nesting depth inside a function body.
//!
//! Definition (§2.11 + T5 brief):
//! - The function body itself sits at depth 0.
//! - The body of every control-flow construct increases depth by 1 relative to
//!   its enclosing depth: `if` / `else` / `else if` / `match` / `while` /
//!   `for` / `loop` / closure.
//! - Plain `{ }` blocks (statement blocks not attached to a control-flow head)
//!   do **not** add depth.
//!
//! The returned value is the maximum depth observed anywhere within
//! `body_range`.

use tree_sitter::{Node, Range, Tree};

/// Compute the maximum nesting depth of `body_range` inside `tree`.
///
/// `source` is unused today but kept in the signature per §2.5 (uniform metric
/// API; cyclomatic / cognitive use it).
///
/// Returns 0 for a body with no control-flow constructs (or when no node
/// covers `body_range` exactly — defensive: callers always pass a real body
/// range produced by `parser::parse_file`).
pub fn compute(tree: &Tree, _source: &str, body_range: Range) -> u32 {
    let root = tree.root_node();
    let Some(body_node) = root.descendant_for_byte_range(body_range.start_byte, body_range.end_byte)
    else {
        return 0;
    };

    let mut max_depth = 0u32;
    walk(body_node, 0, &mut max_depth);
    max_depth
}

/// Walk `node` tracking the current control-flow nesting depth.
///
/// Strategy: descend through every child at the same depth by default; when we
/// encounter a control-flow construct, recurse into its **body** field at
/// `depth + 1` and into any non-body children (condition / scrutinee /
/// pattern) at `depth`. `else_clause` is treated as a body-bearing alternative.
fn walk(node: Node<'_>, depth: u32, max_depth: &mut u32) {
    *max_depth = (*max_depth).max(depth);

    match node.kind() {
        "if_expression" => {
            // condition stays at current depth; consequence + alternative go +1.
            if let Some(cond) = node.child_by_field_name("condition") {
                walk(cond, depth, max_depth);
            }
            if let Some(consequence) = node.child_by_field_name("consequence") {
                walk(consequence, depth + 1, max_depth);
            }
            if let Some(alternative) = node.child_by_field_name("alternative") {
                walk_alternative(alternative, depth, max_depth);
            }
        }
        "match_expression" => {
            if let Some(value) = node.child_by_field_name("value") {
                walk(value, depth, max_depth);
            }
            if let Some(body) = node.child_by_field_name("body") {
                // body is `match_block`; its arms (and their guards/values) are inside the
                // match body, so everything in there is one level deeper.
                walk(body, depth + 1, max_depth);
            }
        }
        "while_expression" | "while_let_expression" => {
            if let Some(cond) = node.child_by_field_name("condition") {
                walk(cond, depth, max_depth);
            }
            if let Some(body) = node.child_by_field_name("body") {
                walk(body, depth + 1, max_depth);
            }
        }
        "for_expression" => {
            // pattern + iterator stay at current depth; only the body deepens.
            if let Some(pat) = node.child_by_field_name("pattern") {
                walk(pat, depth, max_depth);
            }
            if let Some(value) = node.child_by_field_name("value") {
                walk(value, depth, max_depth);
            }
            if let Some(body) = node.child_by_field_name("body") {
                walk(body, depth + 1, max_depth);
            }
        }
        "loop_expression" => {
            if let Some(body) = node.child_by_field_name("body") {
                walk(body, depth + 1, max_depth);
            }
        }
        "closure_expression" => {
            // Per T5 brief: a closure body is one nesting level deeper, matching the
            // cognitive metric's treatment.
            if let Some(body) = node.child_by_field_name("body") {
                walk(body, depth + 1, max_depth);
            }
        }
        _ => {
            // Plain block / expression / statement: pass through, depth unchanged.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk(child, depth, max_depth);
            }
        }
    }
}

/// Handle the `alternative` child of an `if_expression`.
///
/// In tree-sitter-rust the alternative is an `else_clause` whose payload is
/// either a block (plain `else { ... }`) or another `if_expression`
/// (`else if ...`). Either way the alternative content lives one level
/// deeper than the parent if (the `else` branch is a sibling body of the
/// consequence). For an `else if` chain the *next* if's condition therefore
/// already sits at depth+1, and its consequence at depth+2 — matching the
/// intuition that `if a { if b {} }` and `if a {} else if b {}` both bottom
/// out at depth 2 inside their innermost body.
fn walk_alternative(alternative: Node<'_>, depth: u32, max_depth: &mut u32) {
    let mut cursor = alternative.walk();
    for child in alternative.children(&mut cursor) {
        // Skip the literal `else` keyword token; descend into the payload.
        if child.kind() == "else" {
            continue;
        }
        walk(child, depth + 1, max_depth);
    }
}

pub fn _stub() {}
