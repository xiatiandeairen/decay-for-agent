use decay::metric::statements;
use tree_sitter::Parser;

fn compute_on_body(source: &str) -> u32 {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let body = find_body(tree.root_node()).unwrap();
    statements::compute(&tree, source, body.range())
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
fn empty_function_has_zero_statements() {
    assert_eq!(compute_on_body("fn f() {}\n"), 0);
}

#[test]
fn sequential_statements_are_counted() {
    let source = "fn f() { let a = 1; let b = 2; a + b; }\n";
    assert_eq!(compute_on_body(source), 3);
}

#[test]
fn nested_branches_count_inner_statements() {
    let source = r#"
fn f(x: i32) {
    if x > 0 {
        let y = x;
        y + 1;
    } else {
        return;
    }
}
"#;
    assert_eq!(compute_on_body(source), 5);
}
