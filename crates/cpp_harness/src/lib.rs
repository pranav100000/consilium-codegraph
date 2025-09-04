use anyhow::{Context, Result};
use protocol::{EdgeIR, EdgeType, Language as ProtoLanguage, OccurrenceIR, OccurrenceRole, Resolution, Span, SymbolIR, SymbolKind};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

#[cfg(test)]
mod debug;
#[cfg(test)]
mod edge_cases;
#[cfg(test)]
mod complex_tests;

pub struct CppHarness {
    parser: Parser,
    is_cpp: bool, // true for C++, false for C
}

impl CppHarness {
    pub fn new_cpp() -> Result<Self> {
        let mut parser = Parser::new();
        let lang = tree_sitter_cpp::language();
        parser.set_language(lang).context("Failed to set C++ language")?;
        Ok(Self { parser, is_cpp: true })
    }

    pub fn new_c() -> Result<Self> {
        let mut parser = Parser::new();
        let lang = tree_sitter_c::language();
        parser.set_language(lang).context("Failed to set C language")?;
        Ok(Self { parser, is_cpp: false })
    }

    pub fn parse(
        &mut self,
        file_path: &str,
        content: &str,
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let tree = self.parser.parse(content, None)
            .context("Failed to parse file")?;

        let root_node = tree.root_node();
        
        let mut symbols = Vec::new();
        let mut edges = Vec::new();
        let mut occurrences = Vec::new();
        
        let mut context = ParseContext::new();
        
        self.walk_node(
            root_node,
            content,
            file_path,
            &mut symbols,
            &mut edges,
            &mut occurrences,
            &mut context,
        )?;
        
        Ok((symbols, edges, occurrences))
    }

    fn walk_node(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                self.handle_function(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "class_specifier" if self.is_cpp => {
                self.handle_class(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "struct_specifier" => {
                self.handle_struct(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "enum_specifier" => {
                self.handle_enum(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "namespace_definition" if self.is_cpp => {
                self.handle_namespace(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "declaration" => {
                // Handle global variables, typedefs, function declarations etc.
                self.handle_declaration(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "preproc_include" => {
                self.handle_include(node, content, file_path, edges)?;
            }
            _ => {
                // Recursively walk children
                for child in node.children(&mut node.walk()) {
                    self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                }
            }
        }
        Ok(())
    }

    fn handle_function(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Get function declarator
        let declarator = node.child_by_field_name("declarator")
            .context("Function without declarator")?;
        
        let name = self.get_function_name(declarator, content)?;
        let return_type = node.child_by_field_name("type")
            .map(|n| self.get_text(n, content))
            .unwrap_or_else(|| "void".to_string());
        
        let fqn = context.build_fqn(&name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));
        
        let params = self.get_function_params(declarator, content);
        let signature = format!("{} {}({})", return_type, name, params.join(", "));
        
        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: if self.is_cpp { ProtoLanguage::Cpp } else { ProtoLanguage::C },
            kind: SymbolKind::Function,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: Some(signature),
            file_path: file_path.to_string(),
            span: self.node_to_span(declarator),
            visibility: None, // Will be set based on context
            doc: None,
            sig_hash,
        };
        
        symbols.push(symbol.clone());
        
        // Add occurrence for definition
        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id.clone()),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(declarator),
            token: name.clone(),
        });
        
        // Process function body for references
        if let Some(body) = node.child_by_field_name("body") {
            self.process_function_body(body, content, file_path, edges, occurrences, &symbol.id)?;
        }
        
        Ok(())
    }

    fn handle_class(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        let name_node = node.child_by_field_name("name")
            .context("Class without name")?;
        let name = self.get_text(name_node, content);
        
        let fqn = context.build_fqn(&name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));
        
        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Cpp,
            kind: SymbolKind::Class,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: None,
            doc: None,
            sig_hash,
        };
        
        symbols.push(symbol.clone());
        
        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id.clone()),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(name_node),
            token: name.clone(),
        });
        
        // Handle base classes - base_class_clause is a direct child
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "base_class_clause" {
                    // The base class name is the last type_identifier in the clause
                    for j in 0..child.child_count() {
                        if let Some(subchild) = child.child(j) {
                            if subchild.kind() == "type_identifier" || subchild.kind() == "qualified_identifier" {
                                let base_name = self.get_text(subchild, content);
                                edges.push(EdgeIR {
                                    edge_type: EdgeType::Extends,
                                    src: Some(symbol.id.clone()),
                                    dst: Some(base_name),
                                    file_src: Some(file_path.to_string()),
                                    file_dst: None,
                                    resolution: Resolution::Syntactic,
                                    meta: HashMap::new(),
                                    provenance: HashMap::new(),
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Process class body
        context.push_class(name.clone());
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                match child.kind() {
                    "function_definition" => {
                        self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                    }
                    "declaration" => {
                        self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                    }
                    "field_declaration" => {
                        // Handle class fields/member variables
                        self.handle_field_declaration(child, content, file_path, symbols, occurrences, context)?;
                    }
                    "access_specifier" => {
                        // Track public/private/protected sections
                        let access = self.get_text(child, content);
                        context.set_access(&access);
                    }
                    _ => {}
                }
            }
        }
        context.pop_class();
        
        Ok(())
    }

    fn handle_struct(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        _edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.get_text(name_node, content);
            let fqn = context.build_fqn(&name);
            let sig_hash = format!("{:x}", md5::compute(&fqn));
            
            let symbol = SymbolIR {
                id: format!("{}#{}", file_path, fqn),
                lang: if self.is_cpp { ProtoLanguage::Cpp } else { ProtoLanguage::C },
                kind: SymbolKind::Struct,
                name: name.clone(),
                fqn: fqn.clone(),
                signature: None,
                file_path: file_path.to_string(),
                span: self.node_to_span(name_node),
                visibility: None,
                doc: None,
                sig_hash,
            };
            
            symbols.push(symbol.clone());
            
            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(symbol.id.clone()),
                role: OccurrenceRole::Definition,
                span: self.node_to_span(name_node),
                token: name,
            });
        }
        
        Ok(())
    }

    fn handle_enum(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        _edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.get_text(name_node, content);
            let fqn = context.build_fqn(&name);
            let sig_hash = format!("{:x}", md5::compute(&fqn));
            
            let symbol = SymbolIR {
                id: format!("{}#{}", file_path, fqn),
                lang: if self.is_cpp { ProtoLanguage::Cpp } else { ProtoLanguage::C },
                kind: SymbolKind::Enum,
                name: name.clone(),
                fqn: fqn.clone(),
                signature: None,
                file_path: file_path.to_string(),
                span: self.node_to_span(name_node),
                visibility: None,
                doc: None,
                sig_hash,
            };
            
            symbols.push(symbol.clone());
            
            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(symbol.id.clone()),
                role: OccurrenceRole::Definition,
                span: self.node_to_span(name_node),
                token: name.clone(),
            });
            
            // Process enum values
            if let Some(body) = node.child_by_field_name("body") {
                for child in body.children(&mut body.walk()) {
                    if child.kind() == "enumerator" {
                        if let Some(enum_val_node) = child.child_by_field_name("name") {
                            let enum_val = self.get_text(enum_val_node, content);
                            let enum_fqn = format!("{}.{}", fqn, enum_val);
                            let enum_sig_hash = format!("{:x}", md5::compute(&enum_fqn));
                            
                            let enum_symbol = SymbolIR {
                                id: format!("{}#{}", file_path, enum_fqn),
                                lang: if self.is_cpp { ProtoLanguage::Cpp } else { ProtoLanguage::C },
                                kind: SymbolKind::EnumMember,
                                name: enum_val.clone(),
                                fqn: enum_fqn,
                                signature: None,
                                file_path: file_path.to_string(),
                                span: self.node_to_span(enum_val_node),
                                visibility: None,
                                doc: None,
                                sig_hash: enum_sig_hash,
                            };
                            
                            symbols.push(enum_symbol.clone());
                            
                            occurrences.push(OccurrenceIR {
                                file_path: file_path.to_string(),
                                symbol_id: Some(enum_symbol.id),
                                role: OccurrenceRole::Definition,
                                span: self.node_to_span(enum_val_node),
                                token: enum_val,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn handle_namespace(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        let name = if let Some(name_node) = node.child_by_field_name("name") {
            self.get_text(name_node, content)
        } else {
            // Anonymous namespace
            "<anonymous>".to_string()
        };
        
        context.push_namespace(name.clone());
        
        // Process namespace body
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
            }
        }
        
        context.pop_namespace();
        Ok(())
    }

    fn handle_declaration(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        _edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Check if this is a function declaration (prototype)
        if let Some(declarator) = node.child_by_field_name("declarator") {
            if declarator.kind() == "function_declarator" {
                // This is a function declaration
                let name = self.get_function_name(declarator, content)?;
                let return_type = node.child_by_field_name("type")
                    .map(|n| self.get_text(n, content))
                    .unwrap_or_else(|| "void".to_string());
                
                let fqn = context.build_fqn(&name);
                let sig_hash = format!("{:x}", md5::compute(&fqn));
                
                let params = self.get_function_params(declarator, content);
                let signature = format!("{} {}({})", return_type, name, params.join(", "));
                
                let symbol = SymbolIR {
                    id: format!("{}#{}", file_path, fqn),
                    lang: if self.is_cpp { ProtoLanguage::Cpp } else { ProtoLanguage::C },
                    kind: SymbolKind::Function,
                    name: name.clone(),
                    fqn: fqn.clone(),
                    signature: Some(signature),
                    file_path: file_path.to_string(),
                    span: self.node_to_span(declarator),
                    visibility: context.current_access.clone(),
                    doc: None,
                    sig_hash,
                };
                
                symbols.push(symbol.clone());
                
                occurrences.push(OccurrenceIR {
                    file_path: file_path.to_string(),
                    symbol_id: Some(symbol.id),
                    role: OccurrenceRole::Definition,
                    span: self.node_to_span(declarator),
                    token: name,
                });
                
                return Ok(());
            }
        }
        
        // Handle variable declarations, typedefs, etc.
        for child in node.children(&mut node.walk()) {
            if child.kind() == "init_declarator" {
                if let Some(declarator) = child.child_by_field_name("declarator") {
                    if let Some(name) = self.extract_identifier(declarator, content) {
                        let fqn = context.build_fqn(&name);
                        let sig_hash = format!("{:x}", md5::compute(&fqn));
                        
                        let symbol = SymbolIR {
                            id: format!("{}#{}", file_path, fqn),
                            lang: if self.is_cpp { ProtoLanguage::Cpp } else { ProtoLanguage::C },
                            kind: SymbolKind::Variable,
                            name: name.clone(),
                            fqn: fqn.clone(),
                            signature: None,
                            file_path: file_path.to_string(),
                            span: self.node_to_span(declarator),
                            visibility: context.current_access.clone(),
                            doc: None,
                            sig_hash,
                        };
                        
                        symbols.push(symbol.clone());
                        
                        occurrences.push(OccurrenceIR {
                            file_path: file_path.to_string(),
                            symbol_id: Some(symbol.id),
                            role: OccurrenceRole::Definition,
                            span: self.node_to_span(declarator),
                            token: name,
                        });
                    }
                }
            }
        }
        
        Ok(())
    }

    fn handle_field_declaration(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Get the field declarator
        if let Some(declarator) = node.child_by_field_name("declarator") {
            let name = self.get_text(declarator, content);
            let fqn = context.build_fqn(&name);
            let sig_hash = format!("{:x}", md5::compute(&fqn));
            
            // Get the type
            let field_type = node.child_by_field_name("type")
                .map(|n| self.get_text(n, content))
                .unwrap_or_else(|| "unknown".to_string());
            
            let symbol = SymbolIR {
                id: format!("{}#{}", file_path, fqn),
                lang: if self.is_cpp { ProtoLanguage::Cpp } else { ProtoLanguage::C },
                kind: SymbolKind::Field,
                name: name.clone(),
                fqn: fqn.clone(),
                signature: Some(format!("{} {}", field_type, name)),
                file_path: file_path.to_string(),
                span: self.node_to_span(declarator),
                visibility: context.current_access.clone(),
                doc: None,
                sig_hash,
            };
            
            symbols.push(symbol.clone());
            
            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(symbol.id),
                role: OccurrenceRole::Definition,
                span: self.node_to_span(declarator),
                token: name,
            });
        }
        
        Ok(())
    }

    fn handle_include(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        edges: &mut Vec<EdgeIR>,
    ) -> Result<()> {
        if let Some(path_node) = node.child_by_field_name("path") {
            let include_path = self.get_text(path_node, content)
                .trim_matches(|c| c == '"' || c == '<' || c == '>')
                .to_string();
            
            edges.push(EdgeIR {
                edge_type: EdgeType::Imports,
                src: Some(file_path.to_string()),
                dst: Some(include_path),
                file_src: Some(file_path.to_string()),
                file_dst: None,
                resolution: Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            });
        }
        
        Ok(())
    }

    fn process_function_body(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        caller_id: &str,
    ) -> Result<()> {
        // Walk through the function body looking for function calls
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "call_expression" {
                if let Some(func_node) = child.child_by_field_name("function") {
                    if let Some(name) = self.extract_identifier(func_node, content) {
                        // Add call edge
                        edges.push(EdgeIR {
                            edge_type: EdgeType::Calls,
                            src: Some(caller_id.to_string()),
                            dst: Some(name.clone()),
                            file_src: Some(file_path.to_string()),
                            file_dst: None,
                            resolution: Resolution::Syntactic,
                            meta: HashMap::new(),
                            provenance: HashMap::new(),
                        });
                        
                        // Add reference occurrence
                        occurrences.push(OccurrenceIR {
                            file_path: file_path.to_string(),
                            symbol_id: None, // Will be resolved later
                            role: OccurrenceRole::Reference,
                            span: self.node_to_span(func_node),
                            token: name,
                        });
                    }
                }
            }
            
            // Recursively process nested blocks
            self.process_function_body(child, content, file_path, edges, occurrences, caller_id)?;
        }
        
        Ok(())
    }

    fn get_function_name(&self, declarator: Node, content: &str) -> Result<String> {
        // Handle different declarator types (pointer, reference, etc.)
        let mut current = declarator;
        loop {
            match current.kind() {
                "function_declarator" => {
                    if let Some(decl) = current.child_by_field_name("declarator") {
                        current = decl;
                    } else {
                        break;
                    }
                }
                "pointer_declarator" | "reference_declarator" => {
                    if let Some(decl) = current.child_by_field_name("declarator") {
                        current = decl;
                    } else {
                        break;
                    }
                }
                "identifier" => {
                    return Ok(self.get_text(current, content));
                }
                "field_identifier" => {
                    return Ok(self.get_text(current, content));
                }
                "destructor_name" | "qualified_identifier" => {
                    return Ok(self.get_text(current, content));
                }
                _ => break,
            }
        }
        
        Err(anyhow::anyhow!("Could not extract function name"))
    }

    fn get_function_params(&self, declarator: Node, content: &str) -> Vec<String> {
        let mut params = Vec::new();
        
        // Find the function_declarator node
        let mut func_decl = None;
        let mut current = declarator;
        loop {
            if current.kind() == "function_declarator" {
                func_decl = Some(current);
                break;
            }
            if let Some(child) = current.child_by_field_name("declarator") {
                current = child;
            } else {
                break;
            }
        }
        
        if let Some(func) = func_decl {
            if let Some(param_list) = func.child_by_field_name("parameters") {
                for child in param_list.children(&mut param_list.walk()) {
                    if child.kind() == "parameter_declaration" {
                        let param_text = self.get_text(child, content);
                        params.push(param_text);
                    }
                }
            }
        }
        
        params
    }

    fn extract_identifier(&self, node: Node, content: &str) -> Option<String> {
        match node.kind() {
            "identifier" | "field_identifier" => Some(self.get_text(node, content)),
            "qualified_identifier" => Some(self.get_text(node, content)),
            _ => {
                // Try to find an identifier child
                for child in node.children(&mut node.walk()) {
                    if let Some(id) = self.extract_identifier(child, content) {
                        return Some(id);
                    }
                }
                None
            }
        }
    }

    fn get_text(&self, node: Node, content: &str) -> String {
        content[node.byte_range()].to_string()
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

struct ParseContext {
    namespaces: Vec<String>,
    classes: Vec<String>,
    current_access: Option<String>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            namespaces: Vec::new(),
            classes: Vec::new(),
            current_access: None,
        }
    }

    fn push_namespace(&mut self, name: String) {
        self.namespaces.push(name);
    }

    fn pop_namespace(&mut self) {
        self.namespaces.pop();
    }

    fn push_class(&mut self, name: String) {
        self.classes.push(name);
        self.current_access = Some("private".to_string()); // Default for C++ classes
    }

    fn pop_class(&mut self) {
        self.classes.pop();
        self.current_access = None;
    }

    fn set_access(&mut self, access: &str) {
        self.current_access = Some(access.trim_end_matches(':').to_string());
    }

    fn build_fqn(&self, name: &str) -> String {
        let mut parts = Vec::new();
        
        // Add namespaces
        for ns in &self.namespaces {
            if ns != "<anonymous>" {
                parts.push(ns.clone());
            }
        }
        
        // Add classes
        for class in &self.classes {
            parts.push(class.clone());
        }
        
        // Add the name itself
        parts.push(name.to_string());
        
        if self.namespaces.is_empty() && self.classes.is_empty() {
            name.to_string()
        } else {
            parts.join("::")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_simple_function() -> Result<()> {
        let mut harness = CppHarness::new_c()?;
        let source = r#"
int add(int a, int b) {
    return a + b;
}
"#;
        
        let (symbols, edges, occurrences) = harness.parse("test.c", source)?;
        
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "add");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert!(symbols[0].signature.as_ref().unwrap().contains("int add(int a, int b)"));
        
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].role, OccurrenceRole::Definition);
        
        Ok(())
    }

    #[test]
    fn test_parse_cpp_class() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
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
        
        let (symbols, _edges, occurrences) = harness.parse("test.cpp", source)?;
        
        // Should have: Calculator class, add method, value field
        assert_eq!(symbols.len(), 3);
        
        let class_sym = symbols.iter().find(|s| s.name == "Calculator").unwrap();
        assert_eq!(class_sym.kind, SymbolKind::Class);
        
        let method_sym = symbols.iter().find(|s| s.name == "add").unwrap();
        assert_eq!(method_sym.kind, SymbolKind::Function);
        assert_eq!(method_sym.fqn, "Calculator::add");
        
        let field_sym = symbols.iter().find(|s| s.name == "value").unwrap();
        assert_eq!(field_sym.kind, SymbolKind::Field);
        assert_eq!(field_sym.fqn, "Calculator::value");
        
        assert_eq!(occurrences.len(), 3);
        
        Ok(())
    }

    #[test]
    fn test_parse_inheritance() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
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
        
        let (symbols, edges, _occurrences) = harness.parse("test.cpp", source)?;
        
        assert_eq!(symbols.len(), 4); // Base, Base::foo, Derived, Derived::foo
        
        // Check inheritance edge
        let extends_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Extends)
            .collect();
        assert_eq!(extends_edges.len(), 1);
        assert_eq!(extends_edges[0].dst, Some("Base".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_parse_struct() -> Result<()> {
        let mut harness = CppHarness::new_c()?;
        let source = r#"
struct Point {
    int x;
    int y;
};
"#;
        
        let (symbols, _edges, _occurrences) = harness.parse("test.c", source)?;
        
        let struct_sym = symbols.iter().find(|s| s.name == "Point").unwrap();
        assert_eq!(struct_sym.kind, SymbolKind::Struct);
        
        Ok(())
    }

    #[test]
    fn test_parse_enum() -> Result<()> {
        let mut harness = CppHarness::new_c()?;
        let source = r#"
enum Color {
    RED,
    GREEN,
    BLUE
};
"#;
        
        let (symbols, _edges, _occurrences) = harness.parse("test.c", source)?;
        
        assert_eq!(symbols.len(), 4); // Color enum + 3 values
        
        let enum_sym = symbols.iter().find(|s| s.name == "Color").unwrap();
        assert_eq!(enum_sym.kind, SymbolKind::Enum);
        
        let red = symbols.iter().find(|s| s.name == "RED").unwrap();
        assert_eq!(red.kind, SymbolKind::EnumMember);
        assert_eq!(red.fqn, "Color.RED");
        
        Ok(())
    }

    #[test]
    fn test_parse_namespace() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
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
        
        let (symbols, _edges, _occurrences) = harness.parse("test.cpp", source)?;
        
        let add_sym = symbols.iter().find(|s| s.name == "add").unwrap();
        assert_eq!(add_sym.fqn, "math::add");
        
        let print_sym = symbols.iter().find(|s| s.name == "print").unwrap();
        assert_eq!(print_sym.fqn, "math::utils::print");
        
        Ok(())
    }

    #[test]
    fn test_parse_includes() -> Result<()> {
        let mut harness = CppHarness::new_c()?;
        let source = r#"
#include <stdio.h>
#include "myheader.h"
"#;
        
        let (_symbols, edges, _occurrences) = harness.parse("test.c", source)?;
        
        let import_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Imports)
            .collect();
        
        assert_eq!(import_edges.len(), 2);
        assert!(import_edges.iter().any(|e| e.dst == Some("stdio.h".to_string())));
        assert!(import_edges.iter().any(|e| e.dst == Some("myheader.h".to_string())));
        
        Ok(())
    }

    #[test]
    fn test_parse_function_calls() -> Result<()> {
        let mut harness = CppHarness::new_c()?;
        let source = r#"
void foo() {
    printf("Hello");
}

void bar() {
    foo();
    foo();
}
"#;
        
        let (_symbols, edges, occurrences) = harness.parse("test.c", source)?;
        
        // Check call edges
        let call_edges: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Calls)
            .collect();
        
        assert_eq!(call_edges.len(), 3); // printf, foo, foo
        
        // Check reference occurrences
        let refs: Vec<_> = occurrences.iter()
            .filter(|o| o.role == OccurrenceRole::Reference)
            .collect();
        
        assert_eq!(refs.len(), 3);
        
        Ok(())
    }

    #[test]
    fn test_empty_file() -> Result<()> {
        let mut harness = CppHarness::new_c()?;
        let source = "";
        
        let (symbols, edges, occurrences) = harness.parse("empty.c", source)?;
        
        assert_eq!(symbols.len(), 0);
        assert_eq!(edges.len(), 0);
        assert_eq!(occurrences.len(), 0);
        
        Ok(())
    }

    #[test]
    fn test_complex_cpp_features() -> Result<()> {
        let mut harness = CppHarness::new_cpp()?;
        let source = r#"
template<typename T>
class Vector {
public:
    void push_back(const T& value);
    T& operator[](size_t index);
};

class String : public std::string {
public:
    String() = default;
    ~String() {}
};
"#;
        
        let (symbols, edges, _occurrences) = harness.parse("test.cpp", source)?;
        
        // Should parse template class and methods
        let vector = symbols.iter().find(|s| s.name == "Vector").unwrap();
        assert_eq!(vector.kind, SymbolKind::Class);
        
        let string = symbols.iter().find(|s| s.name == "String").unwrap();
        assert_eq!(string.kind, SymbolKind::Class);
        
        // Check inheritance
        let extends: Vec<_> = edges.iter()
            .filter(|e| e.edge_type == EdgeType::Extends)
            .collect();
        assert_eq!(extends.len(), 1);
        
        Ok(())
    }
}