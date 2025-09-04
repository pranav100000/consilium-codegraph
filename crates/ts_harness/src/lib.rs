use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, OccurrenceRole, Resolution, Span, SymbolIR, SymbolKind};
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Query, QueryCursor, QueryCapture};

pub struct TypeScriptHarness {
    parser: Parser,
}

impl TypeScriptHarness {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_javascript::LANGUAGE.into())?;
        Ok(Self { parser })
    }
    
    pub fn parse_file(
        &mut self,
        content: &str,
        file_path: &str,
        commit_sha: &str,
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse file"))?;
        
        let mut symbols = vec![];
        let mut edges = vec![];
        let mut occurrences = vec![];
        
        let root_node = tree.root_node();
        let source_bytes = content.as_bytes();
        
        // Extract symbols and relationships
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
        
        // Extract imports as file-to-file edges
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
        
        // Determine language based on file extension
        let lang = if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
            Language::TypeScript
        } else {
            Language::JavaScript
        };
        
        match node_kind {
            "function_declaration" | "function_expression" | "arrow_function" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Function,
                        lang.clone(),
                        node,
                        file_path,
                        commit_sha,
                        source,
                    );
                    
                    // Add CONTAINS edge from parent if exists
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
                    
                    // Add occurrence for definition
                    occurrences.push(OccurrenceIR {
                        file_path: file_path.to_string(),
                        symbol_id: Some(symbol.id.clone()),
                        role: OccurrenceRole::Definition,
                        span: self.node_to_span(name_node),
                        token: name.clone(),
                    });
                    
                    symbols.push(symbol.clone());
                    
                    // Process children with this as parent
                    for child in node.children(&mut node.walk()) {
                        self.extract_symbols_recursive(
                            child,
                            source,
                            file_path,
                            commit_sha,
                            Some(&symbol.id),
                            symbols,
                            edges,
                            occurrences,
                        )?;
                    }
                    return Ok(());
                }
            }
            "class_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Class,
                        lang.clone(),
                        node,
                        file_path,
                        commit_sha,
                        source,
                    );
                    
                    symbols.push(symbol.clone());
                    
                    // Process class body for methods
                    if let Some(body) = node.child_by_field_name("body") {
                        for child in body.children(&mut body.walk()) {
                            if child.kind() == "method_definition" {
                                self.extract_method(
                                    child,
                                    source,
                                    file_path,
                                    commit_sha,
                                    &symbol.id,
                                    lang.clone(),
                                    symbols,
                                    edges,
                                    occurrences,
                                )?;
                            }
                        }
                    }
                    return Ok(());
                }
            }
            "variable_declaration" | "lexical_declaration" => {
                for decl in node.children(&mut node.walk()) {
                    if decl.kind() == "variable_declarator" {
                        if let Some(name_node) = decl.child_by_field_name("name") {
                            let name = self.node_text(name_node, source);
                            
                            // Check if the value is an arrow function
                            let kind = if let Some(value) = decl.child_by_field_name("value") {
                                if value.kind() == "arrow_function" {
                                    SymbolKind::Function
                                } else {
                                    SymbolKind::Variable
                                }
                            } else {
                                SymbolKind::Variable
                            };
                            
                            let symbol = self.create_symbol(
                                &name,
                                kind,
                                lang.clone(),
                                decl,
                                file_path,
                                commit_sha,
                                source,
                            );
                            
                            // Add occurrence for definition
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
                    
                    // Create a CALLS edge (unresolved for now)
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
    
    fn extract_method(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        commit_sha: &str,
        class_id: &str,
        lang: Language,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.node_text(name_node, source);
            let symbol = self.create_symbol(
                &name,
                SymbolKind::Method,
                lang,
                node,
                file_path,
                commit_sha,
                source,
            );
            
            // Add CONTAINS edge from class
            edges.push(EdgeIR {
                edge_type: EdgeType::Contains,
                src: Some(class_id.to_string()),
                dst: Some(symbol.id.clone()),
                file_src: None,
                file_dst: None,
                resolution: Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            });
            
            symbols.push(symbol);
        }
        Ok(())
    }
    
    fn extract_imports(&self, node: Node, source: &[u8], file_path: &str, edges: &mut Vec<EdgeIR>) -> Result<()> {
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            if child.kind() == "import_statement" {
                if let Some(source_node) = child.child_by_field_name("source") {
                    let import_path = self.node_text(source_node, source);
                    let import_path = import_path.trim_matches(|c| c == '\'' || c == '"');
                    
                    // Create file-to-file import edge
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
            } else if child.kind() == "export_statement" {
                // Handle re-exports
                if let Some(source_node) = child.child_by_field_name("source") {
                    let import_path = self.node_text(source_node, source);
                    let import_path = import_path.trim_matches(|c| c == '\'' || c == '"');
                    
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
        }
        
        Ok(())
    }
    
    fn resolve_import_path(&self, current_file: &str, import_path: &str) -> String {
        // Simple resolution for relative imports
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let current_dir = std::path::Path::new(current_file)
                .parent()
                .unwrap_or(std::path::Path::new(""));
            
            let resolved = current_dir.join(import_path);
            
            // Add .ts/.tsx/.js extension if missing
            let path_str = resolved.to_string_lossy();
            if !path_str.ends_with(".ts") && !path_str.ends_with(".tsx") && 
               !path_str.ends_with(".js") && !path_str.ends_with(".jsx") {
                // Try common extensions
                for ext in &[".ts", ".tsx", ".js", ".jsx", "/index.ts", "/index.js"] {
                    let with_ext = format!("{}{}", path_str, ext);
                    return with_ext;
                }
            }
            
            path_str.to_string()
        } else {
            // Node module import
            import_path.to_string()
        }
    }
    
    fn create_symbol(
        &self,
        name: &str,
        kind: SymbolKind,
        lang: Language,
        node: Node,
        file_path: &str,
        commit_sha: &str,
        source: &[u8],
    ) -> SymbolIR {
        let fqn = format!("{}/{}", file_path.trim_end_matches(".ts").trim_end_matches(".tsx").trim_end_matches(".js"), name);
        let sig_hash = format!("{:x}", name.len()); // Simple hash for now
        
        let id = SymbolIR::generate_id(commit_sha, file_path, &lang, &fqn, &sig_hash);
        
        SymbolIR {
            id,
            lang,
            kind,
            name: name.to_string(),
            fqn,
            signature: None, // Will be enhanced later
            file_path: file_path.to_string(),
            span: self.node_to_span(node),
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash,
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

mod test_fixtures;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::fixtures;
    
    #[test]
    fn test_create_harness() -> Result<()> {
        let _harness = TypeScriptHarness::new()?;
        Ok(())
    }
    
    #[test]
    fn test_parse_simple_function() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, edges, occurrences) = harness.parse_file(
            fixtures::SIMPLE_FUNCTION,
            "test.ts",
            "abc123"
        )?;
        
        assert_eq!(symbols.len(), 2, "Should find 2 functions (add and multiply)");
        
        let add_fn = symbols.iter().find(|s| s.name == "add").expect("Should find add function");
        assert_eq!(add_fn.kind, SymbolKind::Function);
        assert_eq!(add_fn.lang, Language::TypeScript);
        
        let multiply_fn = symbols.iter().find(|s| s.name == "multiply").expect("Should find multiply function");
        assert_eq!(multiply_fn.kind, SymbolKind::Function);
        
        Ok(())
    }
    
    #[test]
    fn test_parse_class_with_methods() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, edges, _) = harness.parse_file(
            fixtures::CLASS_WITH_METHODS,
            "calculator.ts",
            "abc123"
        )?;
        
        assert_eq!(symbols.len(), 3, "Should find 1 class and 2 methods");
        
        let class = symbols.iter().find(|s| s.name == "Calculator").expect("Should find Calculator class");
        assert_eq!(class.kind, SymbolKind::Class);
        
        let methods: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Method).collect();
        assert_eq!(methods.len(), 2, "Should find 2 methods");
        
        // Check CONTAINS edges
        let contains_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Contains)
            .collect();
        assert_eq!(contains_edges.len(), 2, "Should have 2 CONTAINS edges (class->method)");
        
        Ok(())
    }
    
    #[test]
    fn test_parse_imports() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (_, edges, _) = harness.parse_file(
            fixtures::IMPORTS_EXAMPLE,
            "components/index.ts",
            "abc123"
        )?;
        
        let import_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .collect();
        
        assert!(import_edges.len() >= 3, "Should find at least 3 import edges");
        
        // Check that imports have file paths
        for edge in import_edges {
            assert!(edge.file_src.is_some(), "Import edge should have source file");
            assert!(edge.file_dst.is_some(), "Import edge should have destination file");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_stable_symbol_ids() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        
        // Parse the same file twice with same commit
        let (symbols1, _, _) = harness.parse_file(
            fixtures::SIMPLE_FUNCTION,
            "test.ts",
            "commit1"
        )?;
        
        let (symbols2, _, _) = harness.parse_file(
            fixtures::SIMPLE_FUNCTION,
            "test.ts",
            "commit1"
        )?;
        
        // Symbol IDs should be identical for same commit
        assert_eq!(symbols1.len(), symbols2.len());
        for i in 0..symbols1.len() {
            assert_eq!(symbols1[i].id, symbols2[i].id, "Symbol IDs should be stable");
        }
        
        // Parse with different commit
        let (symbols3, _, _) = harness.parse_file(
            fixtures::SIMPLE_FUNCTION,
            "test.ts",
            "commit2"
        )?;
        
        // Symbol IDs should be different for different commits
        assert_ne!(symbols1[0].id, symbols3[0].id, "Symbol IDs should differ across commits");
        
        Ok(())
    }
    
    #[test] 
    fn test_occurrences_tracking() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, occurrences) = harness.parse_file(
            fixtures::SIMPLE_FUNCTION,
            "test.ts",
            "abc123"
        )?;
        
        // Should have definition occurrences for each symbol
        let def_occurrences: Vec<_> = occurrences.iter()
            .filter(|o| o.role == OccurrenceRole::Definition)
            .collect();
        
        assert_eq!(def_occurrences.len(), symbols.len(), 
            "Should have one definition occurrence per symbol");
        
        Ok(())
    }
}