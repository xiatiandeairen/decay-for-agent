use tree_sitter::{Node, Range, Tree};

pub fn compute(tree: &Tree, source: &str, body_range: Range) -> u32 {
    let Some(body_node) = tree
        .root_node()
        .descendant_for_byte_range(body_range.start_byte, body_range.end_byte)
    else {
        return 0;
    };
    max_condition_ops(body_node, source)
}

fn max_condition_ops(node: Node<'_>, source: &str) -> u32 {
    let mut max_ops = condition_ops_for_node(node, source);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        max_ops = max_ops.max(max_condition_ops(child, source));
    }
    max_ops
}

fn condition_ops_for_node(node: Node<'_>, source: &str) -> u32 {
    let Some(condition) = condition_node(node) else {
        return 0;
    };
    count_logical_ops(condition, source)
}

fn condition_node(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "if_expression" | "while_expression" | "while_let_expression" => {
            node.child_by_field_name("condition")
        }
        "match_arm" => node.child_by_field_name("guard"),
        _ => None,
    }
}

fn count_logical_ops(node: Node<'_>, source: &str) -> u32 {
    let mut score = 0u32;
    if node.kind() == "binary_expression" {
        if let Some(op_node) = node.child_by_field_name("operator") {
            if matches!(source.get(op_node.byte_range()), Some("&&") | Some("||")) {
                score += 1;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        score += count_logical_ops(child, source);
    }
    score
}
