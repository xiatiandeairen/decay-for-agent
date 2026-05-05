//! Cyclomatic complexity per v0.1 plan §2.5 + T6 brief.
//!
//! Why simplified McCabe (not classical edge-graph McCabe):
//! v0.1 only needs decay-detection signal, not absolute correctness vs textbook
//! formulas. Counting decision-introducing tokens (if / loop heads / match arms
//! after the first / `&&` / `||` / `?`) gives a stable, monotonic score that
//! grows with branching density — which is what the tool reports against
//! thresholds.
//!
//! Counting rules (T6 brief):
//! - base = 1
//! - if_expression: +1 each (covers both standalone `if` and the inner
//!   `if_expression` produced by `else if`, since tree-sitter models
//!   `else if` as `else_clause > if_expression`). A plain `else` clause is
//!   not a decision point and does not contribute.
//! - while_expression / for_expression / loop_expression: +1 each
//! - match_arm: every arm except the first under each `match_block` adds +1
//! - `&&` / `||` (logical operators inside a `binary_expression`): +1 each
//! - `?` (try_expression): +1 each
//! - closure_expression itself: 0 (v0.1 simplification — closures' inner
//!   branches still count via normal traversal of descendants)

use tree_sitter::{Node, Range, Tree};

/// Compute cyclomatic complexity for the function body delimited by
/// `body_range`. Walks every node whose byte span lies inside the range.
pub fn compute(tree: &Tree, _source: &str, body_range: Range) -> u32 {
    let mut score: u32 = 1;
    let mut cursor = tree.walk();
    visit(tree.root_node(), &body_range, &mut cursor, &mut score);
    score
}

/// Recursive descent. `node` is considered "in body" when its byte span is
/// fully within `body_range`. The body block itself is included; its parent
/// `function_item` is not (so we can start from the tree root and let the
/// range filter exclude outside-body siblings).
fn visit<'a>(
    node: Node<'a>,
    body_range: &Range,
    cursor: &mut tree_sitter::TreeCursor<'a>,
    score: &mut u32,
) {
    if !node_in_range(node, body_range) {
        // Still recurse — a sibling outside the range could itself contain
        // the body node as a descendant only if the tree is malformed; in
        // practice, descending only when the node intersects the range is
        // enough. We use overlap (not strict containment) so the body block
        // node — which has the exact same range as body_range — passes.
        if !node_overlaps_range(node, body_range) {
            return;
        }
    } else {
        count_node(node, score);
    }

    for child in node.children(cursor) {
        let mut child_cursor = child.walk();
        visit(child, body_range, &mut child_cursor, score);
    }
}

fn count_node(node: Node<'_>, score: &mut u32) {
    match node.kind() {
        "if_expression" | "while_expression" | "for_expression" | "loop_expression"
        | "try_expression" => {
            *score += 1;
        }
        "match_block" => {
            // First match_arm child is the baseline path; every subsequent
            // arm is an additional decision.
            let mut cur = node.walk();
            let arm_count = node
                .children(&mut cur)
                .filter(|c| c.kind() == "match_arm")
                .count() as u32;
            if arm_count > 1 {
                *score += arm_count - 1;
            }
        }
        "binary_expression" => {
            // tree-sitter-rust models `a && b` as binary_expression with an
            // anonymous `&&` (or `||`) child token; the operator kind appears
            // verbatim as the child node kind.
            let mut cur = node.walk();
            for child in node.children(&mut cur) {
                if matches!(child.kind(), "&&" | "||") {
                    *score += 1;
                }
            }
        }
        _ => {}
    }
}

fn node_in_range(node: Node<'_>, range: &Range) -> bool {
    node.start_byte() >= range.start_byte && node.end_byte() <= range.end_byte
}

fn node_overlaps_range(node: Node<'_>, range: &Range) -> bool {
    node.start_byte() < range.end_byte && node.end_byte() > range.start_byte
}
