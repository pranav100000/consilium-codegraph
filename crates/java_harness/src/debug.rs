#[cfg(test)]
mod debug_tests {
    use crate::*;
    use tree_sitter::Node;

    fn print_tree(node: Node, source: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        let text = if node.child_count() == 0 {
            format!(" \"{}\"", &source[node.byte_range()].trim())
        } else {
            String::new()
        };
        
        println!("{}{}{}", indent, node.kind(), text);
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                print_tree(child, source, depth + 1);
            }
        }
    }

    #[test]
    fn debug_two_classes() {
        let source = r#"
public class Animal {
    protected String name;
}

public class Dog extends Animal implements Runnable {
    public void run() {
        System.out.println("Running!");
    }
}
"#;

        let mut harness = JavaHarness::new().unwrap();
        let (symbols, edges, _) = harness.parse("test.java", source).unwrap();
        
        println!("\n=== All Symbols ===");
        for sym in &symbols {
            println!("  {} ({:?})", sym.name, sym.kind);
        }
        
        println!("\n=== All Edges ===");
        for edge in &edges {
            println!("  {:?}: {:?} -> {:?}", edge.edge_type, edge.src, edge.dst);
        }
        
        assert!(true);
    }

    #[test]
    fn debug_inheritance_parsing() {
        let source = r#"
public class Dog extends Animal implements Runnable {
    public void run() {
        System.out.println("Running!");
    }
}
"#;

        let mut parser = Parser::new();
        let lang = get_language();
        parser.set_language(&lang).unwrap();
        
        let tree = parser.parse(source, None).unwrap();
        println!("\n=== Tree structure for inheritance test ===");
        print_tree(tree.root_node(), source, 0);
        
        // Also test what our parser finds
        let mut harness = JavaHarness::new().unwrap();
        let (symbols, edges, _) = harness.parse("test.java", source).unwrap();
        
        println!("\n=== Symbols found ===");
        for sym in &symbols {
            println!("  {} ({:?}) - fqn: {}", sym.name, sym.kind, sym.fqn);
        }
        
        println!("\n=== Edges found ===");
        for edge in &edges {
            println!("  {:?}: {:?} -> {:?}", edge.edge_type, edge.src, edge.dst);
        }
        
        // Debug the super_interfaces structure
        let root = tree.root_node();
        if let Some(class_decl) = root.child(0) {
            if class_decl.kind() == "class_declaration" {
                if let Some(interfaces) = class_decl.child_by_field_name("super_interfaces") {
                    println!("\n=== Debug super_interfaces ===");
                    println!("super_interfaces has {} children", interfaces.child_count());
                    for i in 0..interfaces.child_count() {
                        if let Some(child) = interfaces.child(i) {
                            println!("  Child {}: kind={}", i, child.kind());
                        }
                    }
                }
            }
        }
        
        // This test is for debugging output, not assertions
        assert!(true);
    }
}