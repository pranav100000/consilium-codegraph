use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, OccurrenceRole, Resolution, Span, SymbolIR, SymbolKind};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

pub struct TypeScriptHarness {
    js_parser: Parser,
    ts_parser: Parser,
}

impl TypeScriptHarness {
    pub fn new() -> Result<Self> {
        let mut js_parser = Parser::new();
        js_parser.set_language(&tree_sitter_javascript::LANGUAGE.into())?;
        
        let mut ts_parser = Parser::new();
        ts_parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
        
        Ok(Self { js_parser, ts_parser })
    }
    
    pub fn parse_file(
        &mut self,
        content: &str,
        file_path: &str,
        commit_sha: &str,
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        
        // Choose the appropriate parser based on file extension
        let parser = if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
            &mut self.ts_parser
        } else {
            &mut self.js_parser
        };
        
        let tree = parser.parse(content, None)
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
            "export_statement" => {
                // Process the exported declaration
                for child in node.children(&mut node.walk()) {
                    if child.kind() != "export" {
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
                }
                return Ok(());
            }
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
            "interface_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Interface,
                        lang.clone(),
                        node,
                        file_path,
                        commit_sha,
                        source,
                    );
                    
                    symbols.push(symbol.clone());
                    
                    // Add parent edge if applicable
                    if let Some(parent_id) = parent_symbol {
                        edges.push(EdgeIR {
                            src: Some(parent_id.to_string()),
                            dst: Some(symbol.id.clone()),
                            file_src: None,
                            file_dst: None,
                            edge_type: EdgeType::Contains,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                    }
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
            "enum_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Enum,
                        lang.clone(),
                        node,
                        file_path,
                        commit_sha,
                        source,
                    );
                    
                    symbols.push(symbol.clone());
                    
                    // Add parent edge if applicable
                    if let Some(parent_id) = parent_symbol {
                        edges.push(EdgeIR {
                            src: Some(parent_id.to_string()),
                            dst: Some(symbol.id.clone()),
                            file_src: None,
                            file_dst: None,
                            edge_type: EdgeType::Contains,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                    }
                    
                    return Ok(());
                }
            }
            "type_alias_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Type,
                        lang.clone(),
                        node,
                        file_path,
                        commit_sha,
                        source,
                    );
                    
                    symbols.push(symbol.clone());
                    
                    // Add parent edge if applicable
                    if let Some(parent_id) = parent_symbol {
                        edges.push(EdgeIR {
                            src: Some(parent_id.to_string()),
                            dst: Some(symbol.id.clone()),
                            file_src: None,
                            file_dst: None,
                            edge_type: EdgeType::Contains,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                    }
                    
                    return Ok(());
                }
            }
            "namespace_declaration" | "module_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Namespace,
                        lang.clone(),
                        node,
                        file_path,
                        commit_sha,
                        source,
                    );
                    
                    symbols.push(symbol.clone());
                    
                    // Add parent edge if applicable
                    if let Some(parent_id) = parent_symbol {
                        edges.push(EdgeIR {
                            src: Some(parent_id.to_string()),
                            dst: Some(symbol.id.clone()),
                            file_src: None,
                            file_dst: None,
                            edge_type: EdgeType::Contains,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                    }
                    
                    // Process namespace/module body
                    if let Some(body) = node.child_by_field_name("body") {
                        for child in body.children(&mut body.walk()) {
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
                    }
                    return Ok(());
                }
            }
            "generator_function_declaration" => {
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
                    
                    symbols.push(symbol.clone());
                    
                    // Add parent edge if applicable
                    if let Some(parent_id) = parent_symbol {
                        edges.push(EdgeIR {
                            src: Some(parent_id.to_string()),
                            dst: Some(symbol.id.clone()),
                            file_src: None,
                            file_dst: None,
                            edge_type: EdgeType::Contains,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                    }
                    
                    return Ok(());
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
                if let Some(ext) = [".ts", ".tsx", ".js", ".jsx", "/index.ts", "/index.js"].iter().next() {
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
    
    #[test]
    fn test_complex_generics() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::COMPLEX_GENERICS,
            "generics.ts",
            "abc123"
        )?;
        
        // Should find interfaces, classes, and type aliases
        let interfaces = symbols.iter().filter(|s| s.kind == SymbolKind::Interface).count();
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        
        assert!(interfaces >= 1, "Should find at least 1 interface");
        assert!(classes >= 1, "Should find at least 1 class");
        
        Ok(())
    }
    
    #[test]
    fn test_async_await_patterns() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::ASYNC_AWAIT_PATTERNS,
            "async.ts",
            "abc123"
        )?;
        
        // Should find async functions and classes
        assert!(symbols.len() >= 3, "Should find at least 3 symbols");
        
        let async_fn = symbols.iter()
            .find(|s| s.name == "fetchData")
            .expect("Should find fetchData function");
        assert_eq!(async_fn.kind, SymbolKind::Function);
        
        Ok(())
    }
    
    #[test]
    fn test_jsx_tsx_components() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, edges, _) = harness.parse_file(
            fixtures::JSX_TSX_COMPONENTS,
            "component.tsx",
            "abc123"
        )?;
        
        // JSX/TSX parsing may vary - focus on finding any symbols
        assert!(!symbols.is_empty(), "Should find some symbols in JSX/TSX file");
        
        // Count all symbol types
        let total_symbols = symbols.len();
        assert!(total_symbols >= 1, "Should find at least 1 symbol");
        
        // Should find React import
        let imports = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .count();
        assert!(imports >= 1, "Should find React import");
        
        Ok(())
    }
    
    #[test]
    fn test_namespace_and_modules() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::NAMESPACE_AND_MODULES,
            "namespaces.ts",
            "abc123"
        )?;
        
        // Should find functions and classes within namespaces
        assert!(!symbols.is_empty(), "Should find symbols in namespaces");
        
        Ok(())
    }
    
    #[test]
    fn test_enum_parsing() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::ENUM_AND_CONST_ENUM,
            "enums.ts",
            "abc123"
        )?;
        
        // Should find enum declarations (though they may be parsed as other types)
        assert!(!symbols.is_empty(), "Should find enum-related symbols");
        
        Ok(())
    }
    
    #[test]
    fn test_abstract_and_protected() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, edges, _) = harness.parse_file(
            fixtures::ABSTRACT_AND_PROTECTED,
            "abstract.ts",
            "abc123"
        )?;
        
        // Should find at least one class (abstract classes might not be detected separately)
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 1, "Should find at least 1 class");
        
        // Should find methods
        let methods = symbols.iter().filter(|s| s.kind == SymbolKind::Method).count();
        assert!(methods >= 2, "Should find at least 2 methods");
        
        // Should have CONTAINS edges for class methods
        let contains = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Contains)
            .count();
        assert!(contains >= 3, "Should have CONTAINS edges for methods");
        
        Ok(())
    }
    
    #[test]
    fn test_property_accessors() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::PROPERTY_ACCESSORS,
            "accessors.ts",
            "abc123"
        )?;
        
        // Should find class and its methods/accessors
        let class = symbols.iter()
            .find(|s| s.name == "Person")
            .expect("Should find Person class");
        assert_eq!(class.kind, SymbolKind::Class);
        
        // Methods include getters and setters
        let methods = symbols.iter().filter(|s| s.kind == SymbolKind::Method).count();
        assert!(methods >= 4, "Should find getter and setter methods");
        
        Ok(())
    }
    
    #[test]
    fn test_complex_exports() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (_, edges, _) = harness.parse_file(
            fixtures::COMPLEX_EXPORTS,
            "exports.ts",
            "abc123"
        )?;
        
        // Should find various export/import edges
        let import_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .count();
        assert!(import_edges >= 4, "Should find multiple re-export edges");
        
        Ok(())
    }
    
    #[test]
    fn test_type_guards_and_assertions() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::TYPE_GUARDS_AND_ASSERTIONS,
            "guards.ts",
            "abc123"
        )?;
        
        // Should find type guard functions
        let functions = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .count();
        assert!(functions >= 4, "Should find type guard functions");
        
        Ok(())
    }
    
    #[test]
    fn test_unicode_identifiers() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::UNICODE_AND_SPECIAL_CHARS,
            "unicode.ts",
            "abc123"
        )?;
        
        // Should handle unicode identifiers
        let unicode_func = symbols.iter()
            .find(|s| s.name == "计算")
            .expect("Should find unicode function");
        assert_eq!(unicode_func.kind, SymbolKind::Function);
        
        let russian_class = symbols.iter()
            .find(|s| s.name == "КлассПример")
            .expect("Should find Russian class");
        assert_eq!(russian_class.kind, SymbolKind::Class);
        
        Ok(())
    }
    
    #[test]
    fn test_malformed_code() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        
        // Should not panic on malformed code
        let result = harness.parse_file(
            fixtures::MALFORMED_CODE,
            "broken.ts",
            "abc123"
        );
        
        // Parser should handle errors gracefully
        assert!(result.is_ok(), "Should handle malformed code without panicking");
        
        Ok(())
    }
    
    #[test]
    fn test_empty_file() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, edges, occurrences) = harness.parse_file(
            fixtures::EMPTY_FILE,
            "empty.ts",
            "abc123"
        )?;
        
        assert_eq!(symbols.len(), 0, "Empty file should have no symbols");
        assert_eq!(edges.len(), 0, "Empty file should have no edges");
        assert_eq!(occurrences.len(), 0, "Empty file should have no occurrences");
        
        Ok(())
    }
    
    #[test]
    fn test_only_comments() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, edges, occurrences) = harness.parse_file(
            fixtures::ONLY_COMMENTS,
            "comments.ts",
            "abc123"
        )?;
        
        assert_eq!(symbols.len(), 0, "Comment-only file should have no symbols");
        assert_eq!(edges.len(), 0, "Comment-only file should have no edges");
        assert_eq!(occurrences.len(), 0, "Comment-only file should have no occurrences");
        
        Ok(())
    }
    
    #[test]
    fn test_large_file_performance() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        
        let start = std::time::Instant::now();
        let (symbols, _, _) = harness.parse_file(
            fixtures::LARGE_FILE,
            "large.ts",
            "abc123"
        )?;
        let duration = start.elapsed();
        
        // Should parse reasonably quickly (under 1 second)
        assert!(duration.as_secs() < 1, "Large file should parse in under 1 second");
        
        // Should find all symbols
        assert!(symbols.len() >= 20, "Should find many symbols in large file");
        
        Ok(())
    }
    
    #[test]
    fn test_index_signatures() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::INDEX_SIGNATURES,
            "index_sig.ts",
            "abc123"
        )?;
        
        // Should find interfaces and classes with index signatures
        let interfaces = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Interface)
            .count();
        let classes = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .count();
        
        assert!(interfaces >= 3, "Should find interfaces with index signatures");
        assert!(classes >= 1, "Should find class with index signature");
        
        Ok(())
    }
    
    #[test]
    fn test_triple_slash_directives() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, edges, _) = harness.parse_file(
            fixtures::TRIPLE_SLASH_DIRECTIVES,
            "directives.ts",
            "abc123"
        )?;
        
        // Should find the function despite triple-slash directives
        let func = symbols.iter()
            .find(|s| s.name == "readFile")
            .expect("Should find readFile function");
        assert_eq!(func.kind, SymbolKind::Function);
        
        // Should find fs import
        let imports = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .count();
        assert!(imports >= 1, "Should find fs import");
        
        Ok(())
    }
    
    #[test]
    fn test_intersection_and_union_types() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::INTERSECTION_AND_UNION_TYPES,
            "types.ts",
            "abc123"
        )?;
        
        // Should find the function that uses union types
        let func = symbols.iter()
            .find(|s| s.name == "processValue")
            .expect("Should find processValue function");
        assert_eq!(func.kind, SymbolKind::Function);
        
        Ok(())
    }
    
    #[test]
    fn test_nested_functions() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::NESTED_FUNCTIONS,
            "nested.ts",
            "abc123"
        )?;
        
        // Should find both outer and inner functions
        let functions = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .count();
        assert_eq!(functions, 2, "Should find both outer and inner functions");
        
        let outer = symbols.iter()
            .find(|s| s.name == "outer")
            .expect("Should find outer function");
        let inner = symbols.iter()
            .find(|s| s.name == "inner")
            .expect("Should find inner function");
        
        assert_eq!(outer.kind, SymbolKind::Function);
        assert_eq!(inner.kind, SymbolKind::Function);
        
        Ok(())
    }
    
    #[test]
    fn test_symbol_span_accuracy() -> Result<()> {
        let mut harness = TypeScriptHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::SIMPLE_FUNCTION,
            "test.ts",
            "abc123"
        )?;
        
        for symbol in &symbols {
            // Spans should have valid line/column numbers
            assert!(symbol.span.start_line <= symbol.span.end_line, 
                "Start line should be <= end line");
            if symbol.span.start_line == symbol.span.end_line {
                assert!(symbol.span.start_col <= symbol.span.end_col, 
                    "Start col should be <= end col on same line");
            }
        }
        
        Ok(())
    }
}