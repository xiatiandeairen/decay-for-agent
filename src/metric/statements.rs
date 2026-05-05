use tree_sitter::{Node, Range, Tree};

pub fn compute(tree: &Tree, _source: &str, body_range: Range) -> u32 {
    let Some(body_node) = tree
        .root_node()
        .descendant_for_byte_range(body_range.start_byte, body_range.end_byte)
    else {
        return 0;
    };
    count_statements(body_node)
}

fn count_statements(node: Node<'_>) -> u32 {
    let mut score = 0u32;
    if is_statement_node(node) {
        score += 1;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        score += count_statements(child);
    }
    score
}

fn is_statement_node(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "let_declaration"
            | "expression_statement"
            | "empty_statement"
            | "return_expression"
            | "break_expression"
            | "continue_expression"
    )
}
