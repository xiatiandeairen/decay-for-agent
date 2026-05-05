use decay::metric::condition_ops;
use tree_sitter::Parser;

fn compute_on_body(source: &str) -> u32 {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let body = find_body(tree.root_node()).unwrap();
    condition_ops::compute(&tree, source, body.range())
}

fn find_body(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    if node.kind() == "function_item" {
        return node.child_by_field_name("body");
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_body(child) {
            return Some(found);
        }
    }
    None
}

#[test]
fn single_condition_has_zero_ops() {
    assert_eq!(compute_on_body("fn f(a: bool) { if a {} }\n"), 0);
}

#[test]
fn counts_and_or_ops_in_condition() {
    let source = "fn f(a: bool, b: bool, c: bool) { if a && b || c {} }\n";
    assert_eq!(compute_on_body(source), 2);
}

#[test]
fn takes_max_across_multiple_conditions() {
    let source = r#"
fn f(a: bool, b: bool, c: bool, d: bool) {
    if a && b {}
    while a && b || c && d {}
}
"#;
    assert_eq!(compute_on_body(source), 3);
}
