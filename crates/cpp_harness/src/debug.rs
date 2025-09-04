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
        
        // Print fields
        let mut cursor = node.walk();
        for field_name in &["name", "declarator", "type", "body", "parameters", "value"] {
            if let Some(field_node) = node.child_by_field_name(field_name) {
                println!("{}  [{}]:", indent, field_name);
                print_tree(field_node, source, depth + 2);
            }
        }
        
        // Print all children if no fields printed
        let has_fields = ["name", "declarator", "type", "body", "parameters", "value"]
            .iter()
            .any(|f| node.child_by_field_name(f).is_some());
            
        if !has_fields {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    print_tree(child, source, depth + 1);
                }
            }
        }
    }

    #[test]
    fn debug_cpp_class() {
        let source = r#"
class Calculator {
public:
    int add(int a, int b) {
        return a + b;
    }
private:
    int value;
};
"#;

        let mut parser = Parser::new();
        let lang = tree_sitter_cpp::language();
        parser.set_language(lang).unwrap();
        
        let tree = parser.parse(source, None).unwrap();
        println!("\n=== C++ Class AST ===");
        print_tree(tree.root_node(), source, 0);
        
        // Also test what our parser finds
        let mut harness = CppHarness::new_cpp().unwrap();
        let (symbols, edges, occurrences) = harness.parse("test.cpp", source).unwrap();
        
        println!("\n=== Symbols found ===");
        for sym in &symbols {
            println!("  {} ({:?}) - fqn: {}", sym.name, sym.kind, sym.fqn);
        }
        
        println!("\n=== Edges found ===");
        for edge in &edges {
            println!("  {:?}: {:?} -> {:?}", edge.edge_type, edge.src, edge.dst);
        }
        
        println!("\n=== Occurrences found ===");
        for occ in &occurrences {
            println!("  {} ({:?})", occ.token, occ.role);
        }
    }

    #[test]
    fn debug_namespace() {
        let source = r#"
namespace math {
    int add(int a, int b) {
        return a + b;
    }
    
    namespace utils {
        void print(int x);
    }
}
"#;

        let mut parser = Parser::new();
        let lang = tree_sitter_cpp::language();
        parser.set_language(lang).unwrap();
        
        let tree = parser.parse(source, None).unwrap();
        println!("\n=== Namespace AST ===");
        print_tree(tree.root_node(), source, 0);
        
        let mut harness = CppHarness::new_cpp().unwrap();
        let (symbols, _, _) = harness.parse("test.cpp", source).unwrap();
        
        println!("\n=== Symbols found ===");
        for sym in &symbols {
            println!("  {} ({:?}) - fqn: {}", sym.name, sym.kind, sym.fqn);
        }
    }

    #[test]
    fn debug_inheritance() {
        let source = r#"
class Base {
public:
    virtual void foo() {}
};

class Derived : public Base {
public:
    void foo() override {}
};
"#;

        let mut parser = Parser::new();
        let lang = tree_sitter_cpp::language();
        parser.set_language(lang).unwrap();
        
        let tree = parser.parse(source, None).unwrap();
        println!("\n=== Inheritance AST ===");
        
        // Find the Derived class
        let root = tree.root_node();
        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                if child.kind() == "class_specifier" {
                    if let Some(name) = child.child_by_field_name("name") {
                        let name_text = &source[name.byte_range()];
                        if name_text == "Derived" {
                            println!("=== Derived class structure ===");
                            print_tree(child, source, 0);
                            
                            // Look for base clause
                            println!("\n=== All children of Derived class ===");
                            for j in 0..child.child_count() {
                                if let Some(subchild) = child.child(j) {
                                    println!("Child {}: {} - text: '{}'", j, subchild.kind(), &source[subchild.byte_range()]);
                                    if subchild.kind().contains("base") || subchild.kind() == ":" {
                                        print_tree(subchild, source, 1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        let mut harness = CppHarness::new_cpp().unwrap();
        let (symbols, edges, _) = harness.parse("test.cpp", source).unwrap();
        
        println!("\n=== Symbols found ===");
        for sym in &symbols {
            println!("  {} ({:?}) - fqn: {}", sym.name, sym.kind, sym.fqn);
        }
        
        println!("\n=== Edges found ===");
        for edge in &edges {
            println!("  {:?}: {:?} -> {:?}", edge.edge_type, edge.src, edge.dst);
        }
    }
}