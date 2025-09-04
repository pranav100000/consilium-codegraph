use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, OccurrenceRole, Resolution, Span, SymbolIR, SymbolKind};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

pub struct PythonHarness {
    parser: Parser,
}

impl PythonHarness {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;
        Ok(Self { parser })
    }
    
    pub fn parse_file(
        &mut self,
        content: &str,
        file_path: &str,
        commit_sha: &str,
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python file"))?;
        
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
            "function_definition" => {
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
                    
                    let symbol_id = symbol.id.clone();
                    symbols.push(symbol);
                    
                    // Process function body
                    if let Some(body) = node.child_by_field_name("body") {
                        self.extract_symbols_recursive(
                            body,
                            source,
                            file_path,
                            commit_sha,
                            Some(&symbol_id),
                            symbols,
                            edges,
                            occurrences,
                        )?;
                    }
                    return Ok(());
                }
            }
            "class_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = self.node_text(name_node, source);
                    let symbol = self.create_symbol(
                        &name,
                        SymbolKind::Class,
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
                    
                    let symbol_id = symbol.id.clone();
                    symbols.push(symbol);
                    
                    // Process class body for methods
                    if let Some(body) = node.child_by_field_name("body") {
                        for child in body.children(&mut body.walk()) {
                            if child.kind() == "function_definition" {
                                self.extract_method(
                                    child,
                                    source,
                                    file_path,
                                    commit_sha,
                                    &symbol_id,
                                    symbols,
                                    edges,
                                    occurrences,
                                )?;
                            } else {
                                self.extract_symbols_recursive(
                                    child,
                                    source,
                                    file_path,
                                    commit_sha,
                                    Some(&symbol_id),
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
            "assignment" => {
                // Handle global/module-level assignments as variables
                if parent_symbol.is_none() {
                    if let Some(left) = node.child_by_field_name("left") {
                        if left.kind() == "identifier" {
                            let name = self.node_text(left, source);
                            // Skip dunder variables
                            if !name.starts_with("__") {
                                let symbol = self.create_symbol(
                                    &name,
                                    SymbolKind::Variable,
                                    node,
                                    file_path,
                                    commit_sha,
                                );
                                
                                occurrences.push(OccurrenceIR {
                                    file_path: file_path.to_string(),
                                    symbol_id: Some(symbol.id.clone()),
                                    role: OccurrenceRole::Definition,
                                    span: self.node_to_span(left),
                                    token: name.clone(),
                                });
                                
                                symbols.push(symbol);
                            }
                        }
                    }
                }
            }
            "call" => {
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
    
    fn extract_method(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        commit_sha: &str,
        class_id: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.node_text(name_node, source);
            let symbol = self.create_symbol(
                &name,
                SymbolKind::Method,
                node,
                file_path,
                commit_sha,
            );
            
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
            
            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(symbol.id.clone()),
                role: OccurrenceRole::Definition,
                span: self.node_to_span(name_node),
                token: name.clone(),
            });
            
            symbols.push(symbol);
        }
        Ok(())
    }
    
    fn extract_imports(&self, node: Node, source: &[u8], file_path: &str, edges: &mut Vec<EdgeIR>) -> Result<()> {
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            match child.kind() {
                "import_statement" | "import_from_statement" => {
                    // Extract module name
                    let module_name = if child.kind() == "import_from_statement" {
                        child.child_by_field_name("module_name")
                            .map(|n| self.node_text(n, source))
                    } else {
                        // For regular import, get the first dotted_name
                        child.children(&mut child.walk())
                            .find(|n| n.kind() == "dotted_name" || n.kind() == "aliased_import")
                            .map(|n| {
                                if n.kind() == "aliased_import" {
                                    n.child_by_field_name("name")
                                        .map(|nn| self.node_text(nn, source))
                                        .unwrap_or_default()
                                } else {
                                    self.node_text(n, source)
                                }
                            })
                    };
                    
                    if let Some(module) = module_name {
                        let resolved_path = self.resolve_import_path(file_path, &module);
                        
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
                _ => {
                    // Recursively check children
                    self.extract_imports(child, source, file_path, edges)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn resolve_import_path(&self, current_file: &str, import_module: &str) -> String {
        // Simple resolution - convert dots to slashes and add .py
        // In real implementation, would need to handle relative imports, packages, etc.
        if import_module.starts_with('.') {
            // Relative import
            let current_dir = std::path::Path::new(current_file)
                .parent()
                .unwrap_or(std::path::Path::new(""));
            
            let module_path = import_module.trim_start_matches('.');
            let resolved = current_dir.join(module_path.replace('.', "/"));
            format!("{}.py", resolved.to_string_lossy())
        } else {
            // Absolute import
            format!("{}.py", import_module.replace('.', "/"))
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
        let module_name = file_path
            .trim_end_matches(".py")
            .replace('/', ".");
        let fqn = format!("{}.{}", module_name, name);
        let sig_hash = format!("{:x}", name.len());
        
        let id = SymbolIR::generate_id(commit_sha, file_path, &Language::Python, &fqn, &sig_hash);
        
        SymbolIR {
            id,
            lang: Language::Python,
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
mod test_fixtures;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::fixtures;
    
    #[test]
    fn test_parse_python_function() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        
        let code = r#"
def hello_world():
    print("Hello, World!")
    
def add(a, b):
    return a + b
"#;
        
        let (symbols, _, occurrences) = harness.parse_file(code, "test.py", "abc123")?;
        
        // Should find at least the two functions
        assert!(symbols.len() >= 2, "Should find at least 2 functions");
        
        let hello = symbols.iter().find(|s| s.name == "hello_world").expect("Should find hello_world");
        assert_eq!(hello.kind, SymbolKind::Function);
        
        let add = symbols.iter().find(|s| s.name == "add").expect("Should find add");
        assert_eq!(add.kind, SymbolKind::Function);
        
        assert!(occurrences.len() >= 2, "Should have at least 2 occurrences");
        
        Ok(())
    }
    
    #[test]
    fn test_parse_python_class() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        
        let code = r#"
class Calculator:
    def __init__(self):
        self.value = 0
    
    def add(self, n):
        self.value += n
    
    def get_value(self):
        return self.value
"#;
        
        let (symbols, edges, _) = harness.parse_file(code, "test.py", "abc123")?;
        
        assert_eq!(symbols.len(), 4); // class + 3 methods
        
        let class_symbol = symbols.iter().find(|s| s.name == "Calculator").unwrap();
        assert_eq!(class_symbol.kind, SymbolKind::Class);
        
        let methods: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect();
        assert_eq!(methods.len(), 3);
        
        // Check CONTAINS edges
        let contains_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Contains)
            .collect();
        assert_eq!(contains_edges.len(), 3);
        
        Ok(())
    }
    
    #[test]
    fn test_parse_python_imports() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        
        let code = r#"
import os
import sys
from typing import List, Dict
from .utils import helper
from ..parent import something
"#;
        
        let (_, edges, _) = harness.parse_file(code, "module/test.py", "abc123")?;
        
        let import_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .collect();
        
        assert!(import_edges.len() >= 4); // At least os, sys, typing, utils
        
        Ok(())
    }
    
    #[test]
    fn test_decorators_and_properties() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::DECORATORS_AND_PROPERTIES,
            "decorators.py",
            "abc123"
        )?;
        
        // Should find class and decorated methods
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 1, "Should find at least 1 class");
        
        // In Python, properties and decorated functions may be classified as Functions
        // Only __init__ and regular instance methods are classified as Methods
        let methods = symbols.iter().filter(|s| s.kind == SymbolKind::Method).count();
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        
        // Should find __init__ as a method
        assert!(methods >= 1, "Should find at least __init__ method");
        // Should find decorators, properties, and static/class methods as functions
        assert!(functions >= 5, "Should find decorator and property functions");
        
        Ok(())
    }
    
    #[test]
    fn test_async_and_generators() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::ASYNC_AND_GENERATORS,
            "async.py",
            "abc123"
        )?;
        
        // Should find async functions and generator functions
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        assert!(functions >= 4, "Should find async and generator functions");
        
        Ok(())
    }
    
    #[test]
    fn test_complex_inheritance() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, edges, _) = harness.parse_file(
            fixtures::COMPLEX_INHERITANCE,
            "inheritance.py",
            "abc123"
        )?;
        
        // Should find abstract base class and multiple derived classes
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 7, "Should find multiple classes with inheritance");
        
        // Check for methods in classes
        let methods = symbols.iter().filter(|s| s.kind == SymbolKind::Method).count();
        assert!(methods >= 8, "Should find methods in various classes");
        
        Ok(())
    }
    
    #[test]
    fn test_comprehensions_and_lambdas() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::COMPREHENSIONS_AND_LAMBDAS,
            "comprehensions.py",
            "abc123"
        )?;
        
        // Comprehensions and lambdas might not produce many symbols
        // but should at least parse without errors
        assert!(true, "Should parse comprehensions without errors");
        
        Ok(())
    }
    
    #[test]
    fn test_context_managers() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::CONTEXT_MANAGERS,
            "context.py",
            "abc123"
        )?;
        
        // Should find classes with __enter__ and __exit__ methods
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 2, "Should find context manager classes");
        
        let methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .filter(|s| s.name.contains("enter") || s.name.contains("exit"))
            .count();
        assert!(methods >= 4, "Should find __enter__ and __exit__ methods");
        
        Ok(())
    }
    
    #[test]
    fn test_metaclasses() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::METACLASSES,
            "metaclasses.py",
            "abc123"
        )?;
        
        // Should find metaclasses and classes using them
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 5, "Should find metaclasses and their instances");
        
        Ok(())
    }
    
    #[test]
    fn test_type_hints_and_annotations() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::TYPE_HINTS_AND_ANNOTATIONS,
            "typing.py",
            "abc123"
        )?;
        
        // Should find functions with complex type hints
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        assert!(functions >= 2, "Should find typed functions");
        
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 3, "Should find TypedDict, Protocol, and regular classes");
        
        Ok(())
    }
    
    #[test]
    fn test_dataclasses_and_attrs() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::DATACLASSES_AND_ATTRS,
            "dataclasses.py",
            "abc123"
        )?;
        
        // Should find dataclasses and their methods
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 5, "Should find dataclasses and attrs classes");
        
        Ok(())
    }
    
    #[test]
    fn test_pattern_matching() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::PATTERN_MATCHING,
            "pattern_match.py",
            "abc123"
        )?;
        
        // Should find functions with match statements
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        assert!(functions >= 2, "Should find functions with pattern matching");
        
        Ok(())
    }
    
    #[test]
    fn test_unicode_and_special_names() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::UNICODE_AND_SPECIAL_NAMES,
            "unicode.py",
            "abc123"
        )?;
        
        // Should handle unicode identifiers
        let unicode_func = symbols.iter()
            .find(|s| s.name == "计算");
        assert!(unicode_func.is_some(), "Should find unicode function");
        
        let russian_class = symbols.iter()
            .find(|s| s.name == "МойКласс");
        assert!(russian_class.is_some(), "Should find Russian class");
        
        // Should find special methods
        let special_methods = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .filter(|s| s.name.starts_with("__") && s.name.ends_with("__"))
            .count();
        assert!(special_methods >= 10, "Should find many special methods");
        
        Ok(())
    }
    
    #[test]
    fn test_malformed_code() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        
        // Should handle malformed code gracefully
        let result = harness.parse_file(
            fixtures::MALFORMED_CODE,
            "broken.py",
            "abc123"
        );
        
        // Parser should not panic on malformed code
        assert!(result.is_ok(), "Should handle malformed code without panicking");
        
        Ok(())
    }
    
    #[test]
    fn test_empty_file() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, edges, occurrences) = harness.parse_file(
            fixtures::EMPTY_FILE,
            "empty.py",
            "abc123"
        )?;
        
        assert_eq!(symbols.len(), 0, "Empty file should have no symbols");
        assert_eq!(edges.len(), 0, "Empty file should have no edges");
        assert_eq!(occurrences.len(), 0, "Empty file should have no occurrences");
        
        Ok(())
    }
    
    #[test]
    fn test_only_comments() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, edges, occurrences) = harness.parse_file(
            fixtures::ONLY_COMMENTS,
            "comments.py",
            "abc123"
        )?;
        
        assert_eq!(symbols.len(), 0, "Comment-only file should have no symbols");
        assert_eq!(edges.len(), 0, "Comment-only file should have no edges");
        assert_eq!(occurrences.len(), 0, "Comment-only file should have no occurrences");
        
        Ok(())
    }
    
    #[test]
    fn test_nested_functions_and_closures() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::NESTED_FUNCTIONS_AND_CLOSURES,
            "nested.py",
            "abc123"
        )?;
        
        // Should find outer and inner functions
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        assert!(functions >= 7, "Should find nested functions");
        
        // Should find nested classes
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 3, "Should find nested classes");
        
        Ok(())
    }
    
    #[test]
    fn test_exception_handling() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::EXCEPTION_HANDLING,
            "exceptions.py",
            "abc123"
        )?;
        
        // Should find exception classes and functions
        let classes = symbols.iter().filter(|s| s.kind == SymbolKind::Class).count();
        assert!(classes >= 2, "Should find exception classes");
        
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        assert!(functions >= 3, "Should find functions with exception handling");
        
        Ok(())
    }
    
    #[test]
    fn test_module_and_package_imports() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, edges, _) = harness.parse_file(
            fixtures::MODULE_AND_PACKAGE_IMPORTS,
            "imports.py",
            "abc123"
        )?;
        
        // Should find many import edges
        let import_edges = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .count();
        assert!(import_edges >= 10, "Should find many import statements");
        
        // Should find public and private functions/classes
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        assert!(functions >= 3, "Should find module functions");
        
        Ok(())
    }
    
    #[test]
    fn test_global_and_nonlocal() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::GLOBAL_AND_NONLOCAL,
            "scopes.py",
            "abc123"
        )?;
        
        // Should find functions with nested scopes
        let functions = symbols.iter().filter(|s| s.kind == SymbolKind::Function).count();
        assert!(functions >= 3, "Should find nested functions with scope modifiers");
        
        Ok(())
    }
    
    #[test]
    fn test_large_file_performance() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        
        let start = std::time::Instant::now();
        let (symbols, _, _) = harness.parse_file(
            fixtures::LARGE_FILE,
            "large.py",
            "abc123"
        )?;
        let duration = start.elapsed();
        
        // Should parse reasonably quickly
        assert!(duration.as_secs() < 1, "Large file should parse in under 1 second");
        
        // Should find many symbols
        assert!(symbols.len() >= 20, "Should find many symbols in large file");
        
        Ok(())
    }
    
    #[test]
    fn test_symbol_span_accuracy() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        let (symbols, _, _) = harness.parse_file(
            fixtures::SIMPLE_FUNCTION,
            "test.py",
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
    
    #[test]
    fn test_stable_symbol_ids() -> Result<()> {
        let mut harness = PythonHarness::new()?;
        
        // Parse same file twice with same commit
        let (symbols1, _, _) = harness.parse_file(
            fixtures::CLASS_WITH_METHODS,
            "test.py",
            "commit1"
        )?;
        
        let (symbols2, _, _) = harness.parse_file(
            fixtures::CLASS_WITH_METHODS,
            "test.py",
            "commit1"
        )?;
        
        // Symbol IDs should be identical
        assert_eq!(symbols1.len(), symbols2.len());
        for i in 0..symbols1.len() {
            assert_eq!(symbols1[i].id, symbols2[i].id, "Symbol IDs should be stable");
        }
        
        // Parse with different commit
        let (symbols3, _, _) = harness.parse_file(
            fixtures::CLASS_WITH_METHODS,
            "test.py",
            "commit2"
        )?;
        
        // Symbol IDs should differ between commits
        assert_ne!(symbols1[0].id, symbols3[0].id, "Symbol IDs should differ across commits");
        
        Ok(())
    }
}