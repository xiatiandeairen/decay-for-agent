//! Cognitive complexity per the SonarSource white paper, simplified per §2.12.
//!
//! Three rules in play:
//!   B1 (increment)        — every flow break adds 1
//!   B2 (nesting penalty)  — nested control structures add `1 + nesting`
//!   B3 (hybrid)           — `else`, repeated `match` arms, `&&`/`||` chain
//!                           heads add 1 without nesting, the simplification
//!                           SonarSource calls "doesn't increase nesting".
//!
//! Why deviate from a literal walk of §2.12 pseudocode:
//!   - `match_arm` increments don't deepen nesting (cases are siblings, not
//!     nesting). The pseudocode line `score += 1 + cognitive(child, nesting+1)`
//!     for control structures applies to `if/while/for/loop`; `match` is +1
//!     itself but its arm contents inherit the **outer** nesting (the +1 was
//!     already paid when entering match). This matches the worked example in
//!     T7 brief (sample 4 expected = 5).
//!   - `&&` / `||` chain detection: we treat a logical `binary_expression` as
//!     the chain head iff its parent is not a `binary_expression` with the
//!     same operator — so `a && b && c` (nested as `(a && b) && c`) counts 1.
//!
//! Simplifications kept (v0.1):
//!   - no recursive call detection
//!   - `?` chain not deduped (every `try_expression` adds 1)
//!   - no analysis inside macro invocations

use tree_sitter::{Node, Range, Tree};

/// Compute cognitive complexity for the function body identified by
/// `body_range`. Caller passes the whole tree + source; we locate the matching
/// node by byte range.
pub fn compute(tree: &Tree, source: &str, body_range: Range) -> u32 {
    let Some(body_node) = find_node_by_range(tree.root_node(), body_range) else {
        return 0;
    };
    walk_children(body_node, source, 0)
}

/// Locate a node whose byte range exactly matches `range`. Used to recover the
/// body block from the tree without keeping a Node alive across calls.
fn find_node_by_range<'a>(node: Node<'a>, range: Range) -> Option<Node<'a>> {
    if node.start_byte() == range.start_byte && node.end_byte() == range.end_byte {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.start_byte() <= range.start_byte && child.end_byte() >= range.end_byte {
            if let Some(found) = find_node_by_range(child, range) {
                return Some(found);
            }
        }
    }
    None
}

/// Walk every child of `node` at the given `nesting` depth, summing scores.
fn walk_children(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    let mut score = 0u32;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        score = score.saturating_add(score_node(child, source, nesting));
    }
    score
}

/// Score a single node + recurse, applying SonarSource increments per kind.
fn score_node(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    match node.kind() {
        // B2: control structures — +1+nesting, body deepens nesting by 1.
        "if_expression" => score_if(node, source, nesting),
        "while_expression" | "for_expression" | "loop_expression" => {
            score_loop(node, source, nesting)
        }
        // match: +1+nesting, but arm contents inherit the *outer* nesting
        // (cases are siblings, not deeper nesting). See header comment.
        "match_expression" => score_match(node, source, nesting),
        // B2: closure — same as control structure.
        "closure_expression" => score_closure(node, source, nesting),
        // B1: every `?` operator adds 1 (chain dedupe deferred to v0.2).
        "try_expression" => 1u32.saturating_add(walk_children(node, source, nesting)),
        // B3: logical chain head — +1 once per chain, descend without re-counting.
        "binary_expression" => score_binary(node, source, nesting),
        // Plain descent — pass nesting through unchanged.
        _ => walk_children(node, source, nesting),
    }
}

/// `if_expression` shape: condition, consequence, optional alternative
/// (`else_clause`). The condition is walked at the **current** nesting (it's
/// not "inside" the if), the consequence at `nesting+1`. The else_clause is
/// hybrid: +1 itself, contents at `nesting+1` (an `else if` re-enters here as
/// `if_expression` and pays another +1+nesting — that's intentional and
/// matches SonarSource).
fn score_if(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    let mut score = 1u32.saturating_add(nesting);

    if let Some(cond) = node.child_by_field_name("condition") {
        score = score.saturating_add(walk_children_or_self(cond, source, nesting));
    }
    if let Some(cons) = node.child_by_field_name("consequence") {
        score = score.saturating_add(walk_children(cons, source, nesting + 1));
    }
    if let Some(alt) = node.child_by_field_name("alternative") {
        // alt is an else_clause node; +1 hybrid, walk its contents at nesting+1.
        score = score.saturating_add(1);
        score = score.saturating_add(walk_children(alt, source, nesting + 1));
    }
    score
}

/// `while`/`for`/`loop`: +1+nesting, body at nesting+1, condition (if any) at
/// current nesting.
fn score_loop(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    let mut score = 1u32.saturating_add(nesting);

    if let Some(cond) = node.child_by_field_name("condition") {
        score = score.saturating_add(walk_children_or_self(cond, source, nesting));
    }
    if let Some(value) = node.child_by_field_name("value") {
        score = score.saturating_add(walk_children_or_self(value, source, nesting));
    }
    if let Some(body) = node.child_by_field_name("body") {
        score = score.saturating_add(walk_children(body, source, nesting + 1));
    }
    score
}

/// `match`: +1+nesting. Its `match_block` children are walked at the **outer**
/// nesting — every additional arm beyond the first adds +1, but contents
/// inherit the parent depth. The match expression itself (`x` in `match x`)
/// is also walked at the outer nesting.
fn score_match(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    let mut score = 1u32.saturating_add(nesting);
    let mut seen_first_arm = false;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "match_block" => {
                let mut arm_cursor = child.walk();
                for grand in child.children(&mut arm_cursor) {
                    if grand.kind() == "match_arm" {
                        if seen_first_arm {
                            score = score.saturating_add(1);
                        } else {
                            seen_first_arm = true;
                        }
                        // Walk arm contents at the outer nesting.
                        score = score.saturating_add(walk_children(grand, source, nesting));
                    }
                    // Punctuation (`{`, `}`, `,`) — no contribution.
                }
            }
            // Skip the literal `match` keyword; descend everything else (e.g.
            // the scrutinee expression) at current nesting.
            "match" => {}
            _ => score = score.saturating_add(score_node(child, source, nesting)),
        }
    }
    score
}

/// `closure_expression`: +1+nesting, body at nesting+1.
fn score_closure(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    let mut score = 1u32.saturating_add(nesting);
    if let Some(body) = node.child_by_field_name("body") {
        score = score.saturating_add(walk_children(body, source, nesting + 1));
    }
    score
}

/// `binary_expression`: only `&&` / `||` matter; everything else just descends.
/// A logical operator pays +1 iff this node is the **chain head** — i.e. its
/// parent is not a `binary_expression` with the same operator. That collapses
/// `a && b && c` (parsed `(a && b) && c`) to a single +1.
fn score_binary(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    let op = operator_text(node, source);
    let is_logical = matches!(op, Some("&&") | Some("||"));

    let mut score = 0u32;
    if is_logical && is_logical_chain_head(node, source, op.unwrap()) {
        score = score.saturating_add(1);
    }
    // Descend into operands regardless — they may contain nested control flow.
    score = score.saturating_add(walk_children(node, source, nesting));
    score
}

fn operator_text<'a>(node: Node<'_>, source: &'a str) -> Option<&'a str> {
    let op_node = node.child_by_field_name("operator")?;
    source.get(op_node.byte_range())
}

fn is_logical_chain_head(node: Node<'_>, source: &str, op: &str) -> bool {
    // Head iff parent isn't a binary_expression with the same logical op.
    let Some(parent) = node.parent() else {
        return true;
    };
    if parent.kind() != "binary_expression" {
        return true;
    }
    operator_text(parent, source) != Some(op)
}

/// Score a node's own contribution **and** descend into it. Used at the
/// "boundary" of nesting (e.g. `if` condition) so we don't double-walk.
fn walk_children_or_self(node: Node<'_>, source: &str, nesting: u32) -> u32 {
    score_node(node, source, nesting)
}
