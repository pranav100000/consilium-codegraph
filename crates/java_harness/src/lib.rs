use anyhow::{Context, Result};
use protocol::{EdgeIR, EdgeType, OccurrenceIR, OccurrenceRole, SymbolIR, SymbolKind, Language as ProtoLanguage, Span};
use std::collections::HashMap;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_java() -> Language;
}

pub fn get_language() -> Language {
    unsafe { tree_sitter_java() }
}

pub struct JavaHarness {
    parser: Parser,
}

impl JavaHarness {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = get_language();
        parser
            .set_language(&language)
            .context("Failed to set Java language")?;
        Ok(Self { parser })
    }

    pub fn parse(
        &mut self,
        file_path: &str,
        content: &str,
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let tree = self
            .parser
            .parse(content, None)
            .context("Failed to parse Java file")?;

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
            "package_declaration" => {
                self.handle_package(node, content, context)?;
            }
            "import_declaration" => {
                self.handle_import(node, content, file_path, edges, occurrences)?;
            }
            "class_declaration" => {
                self.handle_class(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "interface_declaration" => {
                self.handle_interface(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "enum_declaration" => {
                self.handle_enum(node, content, file_path, symbols, edges, occurrences, context)?;
            }
            "method_declaration" | "constructor_declaration" => {
                self.handle_method(node, content, file_path, symbols, occurrences, context)?;
            }
            "field_declaration" => {
                self.handle_field(node, content, file_path, symbols, occurrences, context)?;
            }
            "annotation_type_declaration" => {
                self.handle_annotation(node, content, file_path, symbols, occurrences, context)?;
            }
            "method_invocation" => {
                self.handle_method_call(node, content, file_path, edges, occurrences)?;
            }
            _ => {
                // Recursively walk children for unhandled nodes
                for child in node.children(&mut node.walk()) {
                    self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                }
            }
        }

        Ok(())
    }

    fn handle_package(&self, node: Node, content: &str, context: &mut ParseContext) -> Result<()> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
                let package_name = self.get_text(child, content);
                context.package = Some(package_name);
                break;
            }
        }
        Ok(())
    }

    fn handle_import(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        let import_path = self.extract_import_path(node, content);
        if !import_path.is_empty() {
            let from_id = format!("{}#{}", file_path, self.get_file_fqn(file_path));
            
            edges.push(EdgeIR {
                edge_type: EdgeType::Imports,
                src: Some(from_id),
                dst: Some(import_path.clone()),
                file_src: Some(file_path.to_string()),
                file_dst: None,
                resolution: protocol::Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            });

            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(import_path),
                role: OccurrenceRole::Reference,
                span: self.node_to_span(node),
                token: self.get_text(node, content),
            });
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
        let name_node = node.child_by_field_name("name").context("Class without name")?;
        let name = self.get_text(name_node, content);

        let fqn = context.build_fqn(&name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let modifiers = self.get_modifiers(node, content);
        let is_public = modifiers.iter().any(|m| m == "public");
        let is_abstract = modifiers.iter().any(|m| m == "abstract");
        let is_final = modifiers.iter().any(|m| m == "final");

        let mut properties = HashMap::new();
        if is_abstract {
            properties.insert("is_abstract".to_string(), "true".to_string());
        }
        if is_final {
            properties.insert("is_final".to_string(), "true".to_string());
        }

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            kind: SymbolKind::Class,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if is_public { Some("public".to_string()) } else { None },
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

        // Handle superclass
        if let Some(superclass) = node.child_by_field_name("superclass") {
            if let Some(type_node) = superclass.child(1) { // Skip "extends" keyword
                let super_type = self.get_text(type_node, content);
                edges.push(EdgeIR {
                    edge_type: EdgeType::Extends,
                    src: Some(symbol.id.clone()),
                    dst: Some(super_type),
                    file_src: Some(file_path.to_string()),
                    file_dst: None,
                    resolution: protocol::Resolution::Syntactic,
                    meta: HashMap::new(),
                    provenance: HashMap::new(),
                });
            }
        }

        // Handle interfaces  
        if let Some(interfaces) = node.child_by_field_name("interfaces") {
            // The interfaces field contains a super_interfaces node
            if interfaces.kind() == "super_interfaces" {
                // Find the type_list child which contains the interface types
                for child in interfaces.children(&mut interfaces.walk()) {
                    if child.kind() == "type_list" {
                        // Iterate through all interface types in the type_list
                        for type_child in child.children(&mut child.walk()) {
                            if type_child.kind() == "type_identifier" || type_child.kind() == "scoped_type_identifier" {
                                let interface_type = self.get_text(type_child, content);
                                edges.push(EdgeIR {
                                    edge_type: EdgeType::Implements,
                                    src: Some(symbol.id.clone()),
                                    dst: Some(interface_type),
                                    file_src: Some(file_path.to_string()),
                                    file_dst: None,
                                    resolution: protocol::Resolution::Syntactic,
                                    meta: HashMap::new(),
                                    provenance: HashMap::new(),
                                });
                            }
                        }
                        break; // We found and processed the type_list
                    }
                }
            }
        }

        // Process class body
        context.push_class(name.clone());
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
            }
        }
        context.pop_class();

        Ok(())
    }

    fn handle_interface(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        let name_node = node.child_by_field_name("name").context("Interface without name")?;
        let name = self.get_text(name_node, content);

        let fqn = context.build_fqn(&name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let modifiers = self.get_modifiers(node, content);
        let is_public = modifiers.iter().any(|m| m == "public");

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            kind: SymbolKind::Interface,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if is_public { Some("public".to_string()) } else { None },
            doc: None,
            sig_hash,
        };

        symbols.push(symbol.clone());

        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(name_node),
            token: name.clone(),
        });

        // Process interface body
        context.push_class(name.clone());
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
            }
        }
        context.pop_class();

        Ok(())
    }

    fn handle_enum(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        let name_node = node.child_by_field_name("name").context("Enum without name")?;
        let name = self.get_text(name_node, content);

        let fqn = context.build_fqn(&name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let modifiers = self.get_modifiers(node, content);
        let is_public = modifiers.iter().any(|m| m == "public");

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            kind: SymbolKind::Enum,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if is_public { Some("public".to_string()) } else { None },
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

        // Process enum body for constants
        context.push_class(name.clone());
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                if child.kind() == "enum_constant" {
                    self.handle_enum_constant(child, content, file_path, symbols, occurrences, context)?;
                } else {
                    self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                }
            }
        }
        context.pop_class();

        Ok(())
    }

    fn handle_enum_constant(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.get_text(name_node, content);
            let fqn = context.build_fqn(&name);
            let sig_hash = format!("{:x}", md5::compute(&fqn));

            let symbol = SymbolIR {
                id: format!("{}#{}", file_path, fqn),
                lang: ProtoLanguage::Java,
                kind: SymbolKind::Constant,
                name: name.clone(),
                fqn,
                signature: None,
                file_path: file_path.to_string(),
                span: self.node_to_span(name_node),
                visibility: Some("public".to_string()), // Enum constants are implicitly public
                doc: None,
                sig_hash,
            };

            symbols.push(symbol.clone());

            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(symbol.id),
                role: OccurrenceRole::Definition,
                span: self.node_to_span(name_node),
                token: name,
            });
        }
        Ok(())
    }

    fn handle_method(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        let name = if node.kind() == "constructor_declaration" {
            if let Some(class_name) = &context.current_class() {
                class_name.clone()
            } else {
                "<init>".to_string()
            }
        } else {
            let name_node = node.child_by_field_name("name").context("Method without name")?;
            self.get_text(name_node, content)
        };

        let fqn = context.build_fqn(&name);
        let signature = self.get_method_signature(node, content);
        let sig_hash = format!("{:x}", md5::compute(&format!("{}{}", fqn, signature)));

        let modifiers = self.get_modifiers(node, content);
        let is_public = modifiers.iter().any(|m| m == "public");
        let is_static = modifiers.iter().any(|m| m == "static");
        let is_abstract = modifiers.iter().any(|m| m == "abstract");
        let is_final = modifiers.iter().any(|m| m == "final");

        let mut properties = HashMap::new();
        if is_static {
            properties.insert("is_static".to_string(), "true".to_string());
        }
        if is_abstract {
            properties.insert("is_abstract".to_string(), "true".to_string());
        }
        if is_final {
            properties.insert("is_final".to_string(), "true".to_string());
        }

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            kind: if node.kind() == "constructor_declaration" {
                SymbolKind::Method
            } else {
                SymbolKind::Method
            },
            name: name.clone(),
            fqn,
            signature: Some(signature),
            file_path: file_path.to_string(),
            span: self.node_to_span(node),
            visibility: if is_public { Some("public".to_string()) } else { None },
            doc: None,
            sig_hash,
        };

        symbols.push(symbol.clone());

        let name_span = if node.kind() == "constructor_declaration" {
            self.node_to_span(node)
        } else {
            let name_node = node.child_by_field_name("name").unwrap();
            self.node_to_span(name_node)
        };

        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id),
            role: OccurrenceRole::Definition,
            span: name_span,
            token: name,
        });

        Ok(())
    }

    fn handle_field(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Look for variable_declarator nodes directly as children
        for child in node.children(&mut node.walk()) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = self.get_text(name_node, content);
                    let fqn = context.build_fqn(&name);
                    let sig_hash = format!("{:x}", md5::compute(&fqn));

                    let modifiers = self.get_modifiers(node, content);
                    let is_public = modifiers.iter().any(|m| m == "public");
                    let is_static = modifiers.iter().any(|m| m == "static");
                    let is_final = modifiers.iter().any(|m| m == "final");

                    let mut properties = HashMap::new();
                    if is_static {
                        properties.insert("is_static".to_string(), "true".to_string());
                    }
                    if is_final {
                        properties.insert("is_final".to_string(), "true".to_string());
                    }

                    // Get field type
                    if let Some(type_node) = node.child_by_field_name("type") {
                        let field_type = self.get_text(type_node, content);
                        properties.insert("field_type".to_string(), field_type);
                    }

                    let symbol = SymbolIR {
                        id: format!("{}#{}", file_path, fqn),
                        lang: ProtoLanguage::Java,
                        kind: SymbolKind::Field,
                        name: name.clone(),
                        fqn,
                        signature: None,
                        file_path: file_path.to_string(),
                        span: self.node_to_span(name_node),
                        visibility: if is_public { Some("public".to_string()) } else { None },
                        doc: None,
                        sig_hash,
                    };

                    symbols.push(symbol.clone());

                    occurrences.push(OccurrenceIR {
                        file_path: file_path.to_string(),
                        symbol_id: Some(symbol.id),
                        role: OccurrenceRole::Definition,
                        span: self.node_to_span(name_node),
                        token: name,
                    });
                }
            }
        }
        Ok(())
    }

    fn handle_annotation(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        let name_node = node.child_by_field_name("name").context("Annotation without name")?;
        let name = self.get_text(name_node, content);

        let fqn = context.build_fqn(&name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let modifiers = self.get_modifiers(node, content);
        let is_public = modifiers.iter().any(|m| m == "public");

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            kind: SymbolKind::Interface, // Annotations are a special kind of interface
            name: format!("@{}", name),
            fqn,
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if is_public { Some("public".to_string()) } else { None },
            doc: None,
            sig_hash,
        };

        symbols.push(symbol.clone());

        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(name_node),
            token: name,
        });

        Ok(())
    }

    fn handle_method_call(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let method_name = self.get_text(name_node, content);
            let from_id = format!("{}#{}", file_path, self.get_file_fqn(file_path));

            edges.push(EdgeIR {
                edge_type: EdgeType::Calls,
                src: Some(from_id),
                dst: Some(method_name.clone()),
                file_src: Some(file_path.to_string()),
                file_dst: None,
                resolution: protocol::Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            });

            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(method_name.clone()),
                role: OccurrenceRole::Call,
                span: self.node_to_span(name_node),
                token: method_name,
            });
        }
        Ok(())
    }

    // Helper methods

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

    fn get_modifiers(&self, node: Node, content: &str) -> Vec<String> {
        let mut modifiers = Vec::new();
        for child in node.children(&mut node.walk()) {
            if child.kind() == "modifiers" {
                for modifier in child.children(&mut child.walk()) {
                    modifiers.push(self.get_text(modifier, content));
                }
                break;
            }
        }
        modifiers
    }

    fn get_method_signature(&self, node: Node, content: &str) -> String {
        let mut sig = String::new();

        // Method name
        if let Some(name_node) = node.child_by_field_name("name") {
            sig.push_str(&self.get_text(name_node, content));
        } else if node.kind() == "constructor_declaration" {
            sig.push_str("<init>");
        }

        // Parameters
        if let Some(params_node) = node.child_by_field_name("parameters") {
            sig.push_str(&self.get_text(params_node, content));
        }

        // Return type
        if let Some(type_node) = node.child_by_field_name("type") {
            sig.push_str(" : ");
            sig.push_str(&self.get_text(type_node, content));
        }

        sig
    }

    fn extract_import_path(&self, node: Node, content: &str) -> String {
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "scoped_identifier" => return self.get_text(child, content),
                "identifier" => return self.get_text(child, content),
                "asterisk" => {} // Handle wildcard imports
                _ => {}
            }
        }
        String::new()
    }

    fn get_file_fqn(&self, file_path: &str) -> String {
        file_path.replace('/', ".").replace(".java", "")
    }
}

struct ParseContext {
    package: Option<String>,
    class_stack: Vec<String>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            package: None,
            class_stack: Vec::new(),
        }
    }

    fn push_class(&mut self, name: String) {
        self.class_stack.push(name);
    }

    fn pop_class(&mut self) {
        self.class_stack.pop();
    }

    fn current_class(&self) -> Option<String> {
        self.class_stack.last().cloned()
    }

    fn build_fqn(&self, name: &str) -> String {
        let mut parts = Vec::new();
        
        if let Some(package) = &self.package {
            parts.push(package.clone());
        }
        
        for class in &self.class_stack {
            parts.push(class.clone());
        }
        
        parts.push(name.to_string());
        parts.join(".")
    }
}

#[cfg(test)]
mod debug;
#[cfg(test)]
mod edge_cases;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_class() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
package com.example;

public class Calculator {
    private int value;
    
    public Calculator() {
        this.value = 0;
    }
    
    public void add(int n) {
        this.value += n;
    }
}
"#;

        let (symbols, _, occurrences) = harness.parse("Calculator.java", content)?;

        assert!(symbols.iter().any(|s| s.name == "Calculator" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "value" && s.kind == SymbolKind::Field));
        assert!(symbols.iter().any(|s| s.name == "Calculator" && s.kind == SymbolKind::Method)); // Constructor
        assert!(symbols.iter().any(|s| s.name == "add" && s.kind == SymbolKind::Method));

        assert!(!occurrences.is_empty());

        Ok(())
    }

    #[test]
    fn test_parse_interface() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
public interface Runnable {
    void run();
}
"#;

        let (symbols, _, _) = harness.parse("Runnable.java", content)?;

        assert!(symbols.iter().any(|s| s.name == "Runnable" && s.kind == SymbolKind::Interface));
        assert!(symbols.iter().any(|s| s.name == "run" && s.kind == SymbolKind::Method));

        Ok(())
    }

    #[test]
    fn test_parse_enum() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
public enum Color {
    RED, GREEN, BLUE;
    
    private String hex;
    
    public String getHex() {
        return hex;
    }
}
"#;

        let (symbols, _, _) = harness.parse("Color.java", content)?;

        assert!(symbols.iter().any(|s| s.name == "Color" && s.kind == SymbolKind::Enum));
        assert!(symbols.iter().any(|s| s.name == "RED" && s.kind == SymbolKind::Constant));
        assert!(symbols.iter().any(|s| s.name == "GREEN" && s.kind == SymbolKind::Constant));
        assert!(symbols.iter().any(|s| s.name == "BLUE" && s.kind == SymbolKind::Constant));
        assert!(symbols.iter().any(|s| s.name == "hex" && s.kind == SymbolKind::Field));
        assert!(symbols.iter().any(|s| s.name == "getHex" && s.kind == SymbolKind::Method));

        Ok(())
    }

    #[test]
    fn test_parse_inheritance() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
public class Animal {
    protected String name;
}

public class Dog extends Animal implements Runnable {
    public void run() {
        System.out.println("Running!");
    }
}
"#;

        let (symbols, edges, _) = harness.parse("Animals.java", content)?;

        assert!(symbols.iter().any(|s| s.name == "Animal" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "Dog" && s.kind == SymbolKind::Class));
        
        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Extends));
        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Implements));

        Ok(())
    }

    #[test]
    fn test_parse_imports() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
import java.util.List;
import java.util.ArrayList;
import java.io.*;

public class Test {
}
"#;

        let (_, edges, occurrences) = harness.parse("Test.java", content)?;

        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Imports));
        assert!(occurrences.iter().any(|o| o.role == OccurrenceRole::Reference));

        Ok(())
    }

    #[test]
    fn test_parse_annotations() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
@Deprecated
public class OldClass {
    @Override
    public String toString() {
        return "old";
    }
}

@interface CustomAnnotation {
    String value();
}
"#;

        let (symbols, _, _) = harness.parse("Annotated.java", content)?;

        assert!(symbols.iter().any(|s| s.name == "OldClass" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "@CustomAnnotation" && s.kind == SymbolKind::Interface));

        Ok(())
    }

    #[test]
    fn test_parse_nested_classes() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
public class Outer {
    private int x;
    
    public class Inner {
        public void method() {
            System.out.println(x);
        }
    }
    
    public static class StaticNested {
        public void staticMethod() {
        }
    }
}
"#;

        let (symbols, _, _) = harness.parse("Outer.java", content)?;

        assert!(symbols.iter().any(|s| s.name == "Outer" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "Inner" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "StaticNested" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.fqn.contains("Outer.Inner")));
        assert!(symbols.iter().any(|s| s.fqn.contains("Outer.StaticNested")));

        Ok(())
    }

    #[test]
    fn test_parse_generics() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
public class Box<T> {
    private T value;
    
    public T get() {
        return value;
    }
    
    public void set(T value) {
        this.value = value;
    }
}
"#;

        let (symbols, _, _) = harness.parse("Box.java", content)?;

        assert!(symbols.iter().any(|s| s.name == "Box" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "value" && s.kind == SymbolKind::Field));
        assert!(symbols.iter().any(|s| s.name == "get" && s.kind == SymbolKind::Method));
        assert!(symbols.iter().any(|s| s.name == "set" && s.kind == SymbolKind::Method));

        Ok(())
    }

    #[test]
    fn test_empty_file() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = "";

        let (symbols, edges, occurrences) = harness.parse("empty.java", content)?;

        assert_eq!(symbols.len(), 0);
        assert_eq!(edges.len(), 0);
        assert_eq!(occurrences.len(), 0);

        Ok(())
    }

    #[test]
    fn test_malformed_java() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let content = r#"
public class {
    this is not valid java
}
"#;

        // Should not panic, just return what it can parse
        let result = harness.parse("malformed.java", content);
        assert!(result.is_ok());

        Ok(())
    }
}