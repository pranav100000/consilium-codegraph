use tree_sitter::{Parser, Node};

fn print_tree(node: Node, source: &[u8], indent: usize) {
    let node_text: String = if node.child_count() == 0 {
        std::str::from_utf8(&source[node.byte_range()])
            .unwrap_or("")
            .chars()
            .take(30)
            .collect()
    } else {
        String::new()
    };
    
    println!("{}{} [{}] {}", 
        " ".repeat(indent), 
        node.kind(),
        if node.is_named() { "N" } else { " " },
        node_text
    );
    
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            print_tree(child, source, indent + 2);
        }
    }
}

fn main() {
    let code = r#"
package main

type User struct {
    Name  string
    Email string
}
"#;
    
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_go::LANGUAGE.into()).unwrap();
    
    let tree = parser.parse(code, None).unwrap();
    print_tree(tree.root_node(), code.as_bytes(), 0);
}