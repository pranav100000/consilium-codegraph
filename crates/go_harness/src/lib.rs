use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, OccurrenceRole, Resolution, Span, SymbolIR, SymbolKind};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

pub struct GoHarness {
    parser: Parser,
}

impl GoHarness {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())?;
        Ok(Self { parser })
    }
    
    pub fn parse_file(
        &mut self,
        content: &str,
        file_path: &str,
        commit_sha: &str,
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Go file"))?;
        
        let mut symbols = vec![];
        let mut edges = vec![];
        let mut occurrences = vec![];
        
        let root_node = tree.root_node();
        let source_bytes = content.as_bytes();
        
        self.extract_symbols_recursive(
            root_node,
            source_bytes,
            file_path,
            commit_sha,
            None,
            &mut symbols,
            &mut edges,
            &mut occurrences,
        )?;
        
        self.extract_imports(root_node, source_bytes, file_path, &mut edges)?;
        
        Ok((symbols, edges, occurrences))
    }
    
    fn extract_symbols_recursive(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        commit_sha: &str,
        parent_symbol: Option<&str>,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        let node_kind = node.kind();
        
        match node_kind {
            "function_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Function,
                        node,
                        file_path,
                        commit_sha,
                    );
                    
                    if let Some(parent) = parent_symbol {
                        edges.push(EdgeIR {
                            edge_type: EdgeType::Contains,
                            src: Some(parent.to_string()),
                            dst: Some(symbol.id.clone()),
                            file_src: None,
                            file_dst: None,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                    }
                    
                    occurrences.push(OccurrenceIR {
                        file_path: file_path.to_string(),
                        symbol_id: Some(symbol.id.clone()),
                        role: OccurrenceRole::Definition,
                        span: self.node_to_span(name_node),
                        token: name.clone(),
                    });
                    
                    symbols.push(symbol);
                    return Ok(());
                }
            }
            "method_declaration" => {
                // Methods in Go have a receiver
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    
                    // Try to get receiver type for better naming
                    let receiver_type = node.child_by_field_name("receiver")
                        .and_then(|recv| recv.child_by_field_name("type"))
                        .map(|t| self.extract_type_name(t, source))
                        .unwrap_or_default();
                    
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Method,
                        node,
                        file_path,
                        commit_sha,
                    );
                    
                    // Add edge from receiver type if we can determine it
                    if !receiver_type.is_empty() {
                        let type_id = format!("{}:{}:{}", commit_sha, file_path, receiver_type);
                        edges.push(EdgeIR {
                            edge_type: EdgeType::Contains,
                            src: Some(type_id),
                            dst: Some(symbol.id.clone()),
                            file_src: None,
                            file_dst: None,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                    }
                    
                    occurrences.push(OccurrenceIR {
                        file_path: file_path.to_string(),
                        symbol_id: Some(symbol.id.clone()),
                        role: OccurrenceRole::Definition,
                        span: self.node_to_span(name_node),
                        token: name.clone(),
                    });
                    
                    symbols.push(symbol);
                    return Ok(());
                }
            }
            "type_declaration" => {
                // Handle type declarations (type MyType struct {...})
                // Go's type_declaration has type_spec children
                for spec in node.children(&mut node.walk()) {
                    if spec.kind() == "type_spec" {
                        if let Some(name_node) = spec.child_by_field_name("name") {
                        let name = self.node_text(name_node, source);
                        
                        let kind = if spec.child_by_field_name("type")
                            .map(|t| t.kind() == "struct_type")
                            .unwrap_or(false) {
                            SymbolKind::Class
                        } else if spec.child_by_field_name("type")
                            .map(|t| t.kind() == "interface_type")
                            .unwrap_or(false) {
                            SymbolKind::Interface
                        } else {
                            SymbolKind::Type
                        };
                        
                        let symbol = self.create_symbol(
                            &name,
                            kind,
                            node,
                            file_path,
                            commit_sha,
                        );
                        
                        if let Some(parent) = parent_symbol {
                            edges.push(EdgeIR {
                                edge_type: EdgeType::Contains,
                                src: Some(parent.to_string()),
                                dst: Some(symbol.id.clone()),
                                file_src: None,
                                file_dst: None,
                                resolution: Resolution::Syntactic,
                                meta: HashMap::new(),
                                provenance: HashMap::new(),
                            });
                        }
                        
                        occurrences.push(OccurrenceIR {
                            file_path: file_path.to_string(),
                            symbol_id: Some(symbol.id.clone()),
                            role: OccurrenceRole::Definition,
                            span: self.node_to_span(name_node),
                            token: name.clone(),
                        });
                        
                        let symbol_id = symbol.id.clone();
                        symbols.push(symbol);
                        
                        // Process struct fields
                        if let Some(type_node) = spec.child_by_field_name("type") {
                            if type_node.kind() == "struct_type" {
                                self.extract_struct_fields(
                                    type_node,
                                    source,
                                    file_path,
                                    commit_sha,
                                    &symbol_id,
                                    symbols,
                                    edges,
                                    occurrences,
                                )?;
                            }
                        }
                            return Ok(());
                        }
                    }
                }
            }
            "var_declaration" | "const_declaration" => {
                // Handle variable and constant declarations
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "var_spec" || child.kind() == "const_spec" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            let name = self.node_text(name_node, source);
                            let kind = if node_kind == "const_declaration" {
                                SymbolKind::Constant
                            } else {
                                SymbolKind::Variable
                            };
                            
                            let symbol = self.create_symbol(
                                &name,
                                kind,
                                child,
                                file_path,
                                commit_sha,
                            );
                            
                            occurrences.push(OccurrenceIR {
                                file_path: file_path.to_string(),
                                symbol_id: Some(symbol.id.clone()),
                                role: OccurrenceRole::Definition,
                                span: self.node_to_span(name_node),
                                token: name.clone(),
                            });
                            
                            symbols.push(symbol);
                        }
                    }
                }
            }
            "call_expression" => {
                if let Some(func) = node.child_by_field_name("function") {
                    let callee_name = self.node_text(func, source);
                    
                    occurrences.push(OccurrenceIR {
                        file_path: file_path.to_string(),
                        symbol_id: None,
                        role: OccurrenceRole::Call,
                        span: self.node_to_span(func),
                        token: callee_name,
                    });
                }
            }
            _ => {}
        }
        
        // Recursively process children
        for child in node.children(&mut node.walk()) {
            self.extract_symbols_recursive(
                child,
                source,
                file_path,
                commit_sha,
                parent_symbol,
                symbols,
                edges,
                occurrences,
            )?;
        }
        
        Ok(())
    }
    
    fn extract_struct_fields(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        commit_sha: &str,
        struct_id: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        // struct_type has a field_declaration_list child
        for list_child in node.children(&mut node.walk()) {
            if list_child.kind() == "field_declaration_list" {
                for field_decl in list_child.children(&mut list_child.walk()) {
                    if field_decl.kind() == "field_declaration" {
                        // Go field declarations can have multiple field_identifiers
                        for field_child in field_decl.children(&mut field_decl.walk()) {
                            if field_child.kind() == "field_identifier" {
                                let name = self.node_text(field_child, source);
                                let symbol = self.create_symbol(
                                    &name,
                                    SymbolKind::Field,
                                    field_decl,
                                    file_path,
                                    commit_sha,
                                );
                                
                                edges.push(EdgeIR {
                                    edge_type: EdgeType::Contains,
                                    src: Some(struct_id.to_string()),
                                    dst: Some(symbol.id.clone()),
                                    file_src: None,
                                    file_dst: None,
                                    resolution: Resolution::Syntactic,
                                    meta: HashMap::new(),
                                    provenance: HashMap::new(),
                                });
                                
                                occurrences.push(OccurrenceIR {
                                    file_path: file_path.to_string(),
                                    symbol_id: Some(symbol.id.clone()),
                                    role: OccurrenceRole::Definition,
                                    span: self.node_to_span(field_child),
                                    token: name.clone(),
                                });
                                
                                symbols.push(symbol);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    fn extract_imports(&self, node: Node, source: &[u8], file_path: &str, edges: &mut Vec<EdgeIR>) -> Result<()> {
        // Recursively walk the tree to find import specs
        self.extract_imports_recursive(node, source, file_path, edges)?;
        Ok(())
    }
    
    fn extract_imports_recursive(&self, node: Node, source: &[u8], file_path: &str, edges: &mut Vec<EdgeIR>) -> Result<()> {
        if node.kind() == "import_spec" {
            if let Some(path_node) = node.child_by_field_name("path") {
                let import_path = self.node_text(path_node, source);
                let import_path = import_path.trim_matches('"');
                
                let resolved_path = self.resolve_import_path(file_path, import_path);
                
                edges.push(EdgeIR {
                    edge_type: EdgeType::Imports,
                    src: None,
                    dst: None,
                    file_src: Some(file_path.to_string()),
                    file_dst: Some(resolved_path),
                    resolution: Resolution::Syntactic,
                    meta: HashMap::new(),
                    provenance: HashMap::new(),
                });
            }
        }
        
        // Recursively check children
        for child in node.children(&mut node.walk()) {
            self.extract_imports_recursive(child, source, file_path, edges)?;
        }
        
        Ok(())
    }
    
    fn resolve_import_path(&self, _current_file: &str, import_path: &str) -> String {
        // Simple resolution - Go imports are package paths
        // In a real implementation, would need to handle vendor, go.mod, etc.
        if import_path.starts_with("./") || import_path.starts_with("../") {
            // Relative import
            import_path.to_string()
        } else {
            // Standard library or external package
            import_path.to_string()
        }
    }
    
    fn extract_type_name(&self, node: Node, source: &[u8]) -> String {
        match node.kind() {
            "pointer_type" => {
                // For pointer types, get the underlying type
                if let Some(child) = node.child(0) {
                    self.extract_type_name(child, source)
                } else {
                    String::new()
                }
            }
            "type_identifier" | "identifier" => {
                self.node_text(node, source)
            }
            _ => String::new()
        }
    }
    
    fn create_symbol(
        &self,
        name: &str,
        kind: SymbolKind,
        node: Node,
        file_path: &str,
        commit_sha: &str,
    ) -> SymbolIR {
        let package_name = self.extract_package_name(file_path);
        let fqn = format!("{}.{}", package_name, name);
        let sig_hash = format!("{:x}", name.len());
        
        let id = SymbolIR::generate_id(commit_sha, file_path, &Language::Go, &fqn, &sig_hash);
        
        SymbolIR {
            id,
            lang: Language::Go,
            kind,
            name: name.to_string(),
            fqn,
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(node),
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash,
        }
    }
    
    fn extract_package_name(&self, file_path: &str) -> String {
        // Extract package name from file path
        // In real implementation, would parse the package declaration
        let path = std::path::Path::new(file_path);
        if let Some(parent) = path.parent() {
            parent.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("main")
                .to_string()
        } else {
            "main".to_string()
        }
    }
    
    fn node_text(&self, node: Node, source: &[u8]) -> String {
        std::str::from_utf8(&source[node.byte_range()])
            .unwrap_or("")
            .to_string()
    }
    
    fn node_to_span(&self, node: Node) -> Span {
        let start = node.start_position();
        let end = node.end_position();
        
        Span {
            start_line: start.row as u32,
            start_col: start.column as u32,
            end_line: end.row as u32,
            end_col: end.column as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_go_function() -> Result<()> {
        let mut harness = GoHarness::new()?;
        
        let code = r#"
package main

import "fmt"

func hello() {
    fmt.Println("Hello, World!")
}

func add(a int, b int) int {
    return a + b
}
"#;
        
        let (symbols, _, occurrences) = harness.parse_file(code, "test.go", "abc123")?;
        
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[1].name, "add");
        
        assert_eq!(occurrences.len(), 2);
        
        Ok(())
    }
    
    #[test]
    fn test_parse_go_struct() -> Result<()> {
        let mut harness = GoHarness::new()?;
        
        let code = r#"
package main

type User struct {
    Name  string
    Email string
    Age   int
}

func (u *User) GetName() string {
    return u.Name
}

func (u User) IsAdult() bool {
    return u.Age >= 18
}
"#;
        
        let (symbols, edges, _) = harness.parse_file(code, "test.go", "abc123")?;
        
        // Debug output
        println!("Found {} symbols", symbols.len());
        for sym in &symbols {
            println!("  {} ({:?})", sym.name, sym.kind);
        }
        
        // Should have struct + 3 fields + 2 methods
        assert!(symbols.len() >= 6, "Expected at least 6 symbols, found {}", symbols.len());
        
        let struct_symbol = symbols.iter().find(|s| s.name == "User").unwrap();
        assert_eq!(struct_symbol.kind, SymbolKind::Class); // Using Class for structs
        
        let fields: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Field)
            .collect();
        assert_eq!(fields.len(), 3);
        
        let methods: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect();
        assert_eq!(methods.len(), 2);
        
        // Check CONTAINS edges
        let contains_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Contains)
            .collect();
        assert!(contains_edges.len() >= 3); // At least 3 fields
        
        Ok(())
    }
    
    #[test]
    fn test_parse_go_imports() -> Result<()> {
        let mut harness = GoHarness::new()?;
        
        let code = r#"
package main

import (
    "fmt"
    "os"
    "strings"
    "github.com/user/repo/pkg"
)
"#;
        
        let (_, edges, _) = harness.parse_file(code, "test.go", "abc123")?;
        
        let import_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .collect();
        
        assert_eq!(import_edges.len(), 4);
        
        Ok(())
    }
    
    #[test]
    fn test_parse_go_interface() -> Result<()> {
        let mut harness = GoHarness::new()?;
        
        let code = r#"
package main

type Writer interface {
    Write([]byte) (int, error)
}

type Reader interface {
    Read([]byte) (int, error)
}
"#;
        
        let (symbols, _, _) = harness.parse_file(code, "test.go", "abc123")?;
        
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].kind, SymbolKind::Interface);
        assert_eq!(symbols[1].kind, SymbolKind::Interface);
        
        Ok(())
    }
}