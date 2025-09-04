use anyhow::{Context, Result};
use protocol::{EdgeIR, EdgeType, OccurrenceIR, OccurrenceRole, SymbolIR, SymbolKind, Language as ProtoLanguage, Span};
use std::collections::HashMap;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_rust() -> Language;
}

pub fn get_language() -> Language {
    unsafe { tree_sitter_rust() }
}

pub struct RustHarness {
    parser: Parser,
}

impl RustHarness {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = get_language();
        parser
            .set_language(&language)
            .context("Failed to set Rust language")?;
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
            .context("Failed to parse Rust file")?;

        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        let mut edges = Vec::new();
        let mut occurrences = Vec::new();
        let mut module_stack = vec![];
        let mut impl_context = None;

        self.walk_node(
            root_node,
            content,
            file_path,
            &mut symbols,
            &mut edges,
            &mut occurrences,
            &mut module_stack,
            &mut impl_context,
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
        module_stack: &mut Vec<String>,
        impl_context: &mut Option<String>,
    ) -> Result<()> {
        match node.kind() {
            "function_item" | "function_signature_item" => {
                self.handle_function(
                    node,
                    content,
                    file_path,
                    symbols,
                    occurrences,
                    module_stack,
                    impl_context.as_deref(),
                )?;
            }
            "struct_item" => {
                self.handle_struct(
                    node,
                    content,
                    file_path,
                    symbols,
                    occurrences,
                    module_stack,
                )?;
            }
            "enum_item" => {
                self.handle_enum(
                    node,
                    content,
                    file_path,
                    symbols,
                    occurrences,
                    module_stack,
                )?;
            }
            "impl_item" => {
                self.handle_impl(
                    node,
                    content,
                    file_path,
                    symbols,
                    edges,
                    occurrences,
                    module_stack,
                    impl_context,
                )?;
                return Ok(()); // impl_item handles its own children
            }
            "trait_item" => {
                self.handle_trait(
                    node,
                    content,
                    file_path,
                    symbols,
                    occurrences,
                    module_stack,
                )?;
            }
            "mod_item" => {
                self.handle_module(
                    node,
                    content,
                    file_path,
                    symbols,
                    edges,
                    occurrences,
                    module_stack,
                    impl_context,
                )?;
                return Ok(()); // mod_item handles its own children
            }
            "use_declaration" => {
                self.handle_use(node, content, file_path, edges, occurrences)?;
            }
            "const_item" | "static_item" => {
                self.handle_const_or_static(
                    node,
                    content,
                    file_path,
                    symbols,
                    occurrences,
                    module_stack,
                )?;
            }
            "type_item" => {
                self.handle_type_alias(
                    node,
                    content,
                    file_path,
                    symbols,
                    occurrences,
                    module_stack,
                )?;
            }
            "call_expression" => {
                self.handle_call(node, content, file_path, edges, occurrences)?;
            }
            _ => {}
        }

        // Recursively walk children
        for child in node.children(&mut node.walk()) {
            self.walk_node(
                child,
                content,
                file_path,
                symbols,
                edges,
                occurrences,
                module_stack,
                impl_context,
            )?;
        }

        Ok(())
    }

    fn handle_function(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &[String],
        impl_type: Option<&str>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .context("Function without name")?;
        let name = self.get_text(name_node, content);

        // Skip test functions in test modules
        if name.starts_with("test_") || name == "it_works" {
            if self.find_attribute(node, "test", content).is_some() {
                return Ok(());
            }
        }

        let fqn = self.build_fqn(module_stack, impl_type, &name);
        
        // Generate signature for hash
        let signature = self.get_function_signature(node, content);
        let sig_hash = format!("{:x}", md5::compute(&signature));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Rust,
            kind: if impl_type.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            },
            name: name.clone(),
            fqn: fqn.clone(),
            signature: Some(signature),
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

    fn handle_struct(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &[String],
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .context("Struct without name")?;
        let name = self.get_text(name_node, content);

        let fqn = self.build_fqn(module_stack, None, &name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Rust,
            kind: SymbolKind::Struct,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

        // Handle fields
        if let Some(body) = node.child_by_field_name("body") {
            for field in body.children(&mut body.walk()) {
                if field.kind() == "field_declaration" {
                    self.handle_field(
                        field,
                        content,
                        file_path,
                        symbols,
                        occurrences,
                        &fqn,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn handle_field(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        parent_fqn: &str,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.get_text(name_node, content);
            let fqn = format!("{}.{}", parent_fqn, name);
            let sig_hash = format!("{:x}", md5::compute(&fqn));

            let symbol = SymbolIR {
                id: format!("{}#{}", file_path, fqn),
                lang: ProtoLanguage::Rust,
                kind: SymbolKind::Field,
                name: name.clone(),
                fqn,
                signature: None,
                file_path: file_path.to_string(),
                span: self.node_to_span(name_node),
                visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

    fn handle_enum(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &[String],
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .context("Enum without name")?;
        let name = self.get_text(name_node, content);

        let fqn = self.build_fqn(module_stack, None, &name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Rust,
            kind: SymbolKind::Enum,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

        // Handle enum variants
        if let Some(body) = node.child_by_field_name("body") {
            for variant in body.children(&mut body.walk()) {
                if variant.kind() == "enum_variant" {
                    self.handle_enum_variant(
                        variant,
                        content,
                        file_path,
                        symbols,
                        occurrences,
                        &fqn,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn handle_enum_variant(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        parent_fqn: &str,
    ) -> Result<()> {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.get_text(name_node, content);
            let fqn = format!("{}::{}", parent_fqn, name);
            let sig_hash = format!("{:x}", md5::compute(&fqn));

            let symbol = SymbolIR {
                id: format!("{}#{}", file_path, fqn),
                lang: ProtoLanguage::Rust,
                kind: SymbolKind::Enum, // Using Enum as EnumMember doesn't exist
                name: name.clone(),
                fqn,
                signature: None,
                file_path: file_path.to_string(),
                span: self.node_to_span(name_node),
                visibility: None, // Enum variants inherit visibility from the enum
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

    fn handle_impl(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &mut Vec<String>,
        impl_context: &mut Option<String>,
    ) -> Result<()> {
        // Get the type being implemented for
        let type_node = node.child_by_field_name("type");
        let impl_type = type_node.map(|t| self.get_text(t, content));

        // Check if this is a trait implementation
        let trait_node = node.child_by_field_name("trait");
        let trait_name = trait_node.map(|t| self.get_text(t, content));

        // Set impl context for nested functions
        *impl_context = impl_type.clone();

        // If implementing a trait, create an edge
        if let (Some(impl_type), Some(trait_name)) = (&impl_type, &trait_name) {
            let from_fqn = self.build_fqn(module_stack, None, impl_type);
            let from_id = format!("{}#{}", file_path, from_fqn);

            edges.push(EdgeIR {
                edge_type: EdgeType::Implements,
                src: Some(from_id),
                dst: Some(trait_name.clone()),
                file_src: Some(file_path.to_string()),
                file_dst: None,
                resolution: protocol::Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            });
        }

        // Process impl body
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                self.walk_node(
                    child,
                    content,
                    file_path,
                    symbols,
                    edges,
                    occurrences,
                    module_stack,
                    impl_context,
                )?;
            }
        }

        // Clear impl context
        *impl_context = None;

        Ok(())
    }

    fn handle_trait(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &[String],
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .context("Trait without name")?;
        let name = self.get_text(name_node, content);

        let fqn = self.build_fqn(module_stack, None, &name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Rust,
            kind: SymbolKind::Trait,
            name: name.clone(),
            fqn,
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

    fn handle_module(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &mut Vec<String>,
        impl_context: &mut Option<String>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .context("Module without name")?;
        let name = self.get_text(name_node, content);

        let fqn = self.build_fqn(module_stack, None, &name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Rust,
            kind: SymbolKind::Module,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

        // Push module to stack and process children
        module_stack.push(name);
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                self.walk_node(
                    child,
                    content,
                    file_path,
                    symbols,
                    edges,
                    occurrences,
                    module_stack,
                    impl_context,
                )?;
            }
        }
        module_stack.pop();

        Ok(())
    }

    fn handle_use(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        // Extract the import path
        if let Some(tree_node) = node.child_by_field_name("argument") {
            let import_path = self.get_import_path(tree_node, content);
            if !import_path.is_empty() {
                edges.push(EdgeIR {
                    edge_type: EdgeType::Imports,
                    src: Some(format!("{}#root", file_path)),
                    dst: Some(import_path.clone()),
                    file_src: Some(file_path.to_string()),
                    file_dst: None,
                    resolution: protocol::Resolution::Syntactic,
                    meta: HashMap::new(),
                    provenance: HashMap::new(),
                });

                // Add occurrence for the imported item
                occurrences.push(OccurrenceIR {
                    file_path: file_path.to_string(),
                    symbol_id: Some(import_path),
                    role: OccurrenceRole::Reference,
                    span: self.node_to_span(tree_node),
                    token: self.get_text(tree_node, content),
                });
            }
        }

        Ok(())
    }

    fn handle_const_or_static(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &[String],
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .context("Const/static without name")?;
        let name = self.get_text(name_node, content);

        let fqn = self.build_fqn(module_stack, None, &name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Rust,
            kind: SymbolKind::Constant,
            name: name.clone(),
            fqn,
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

    fn handle_type_alias(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        module_stack: &[String],
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .context("Type alias without name")?;
        let name = self.get_text(name_node, content);

        let fqn = self.build_fqn(module_stack, None, &name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Rust,
            kind: SymbolKind::Type, // Using Type for type aliases
            name: name.clone(),
            fqn,
            signature: None,
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if self.is_public(node) { Some("public".to_string()) } else { None },
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

    fn handle_call(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        if let Some(function_node) = node.child_by_field_name("function") {
            let call_text = self.get_text(function_node, content);

            // Skip macro invocations
            if call_text.ends_with('!') {
                return Ok(());
            }

            edges.push(EdgeIR {
                edge_type: EdgeType::Calls,
                src: Some(format!("{}#root", file_path)),
                dst: Some(call_text.clone()),
                file_src: Some(file_path.to_string()),
                file_dst: None,
                resolution: protocol::Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            });

            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(call_text.clone()),
                role: OccurrenceRole::Call,
                span: self.node_to_span(function_node),
                token: call_text,
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

    fn build_fqn(&self, module_stack: &[String], impl_type: Option<&str>, name: &str) -> String {
        let mut parts = module_stack.to_vec();
        
        if let Some(impl_type) = impl_type {
            parts.push(impl_type.to_string());
        }
        
        parts.push(name.to_string());
        
        if parts.is_empty() {
            name.to_string()
        } else {
            parts.join("::")
        }
    }

    fn get_import_path(&self, node: Node, content: &str) -> String {
        match node.kind() {
            "identifier" | "type_identifier" => self.get_text(node, content),
            "scoped_identifier" => {
                let mut parts = Vec::new();
                self.collect_scoped_parts(node, content, &mut parts);
                parts.join("::")
            }
            "use_list" => {
                // For use statements with braces, get the parent path
                if let Some(parent) = node.parent() {
                    self.get_import_path(parent, content)
                } else {
                    String::new()
                }
            }
            _ => {
                // Try to find identifiers in children
                for child in node.children(&mut node.walk()) {
                    let path = self.get_import_path(child, content);
                    if !path.is_empty() {
                        return path;
                    }
                }
                String::new()
            }
        }
    }

    fn collect_scoped_parts(&self, node: Node, content: &str, parts: &mut Vec<String>) {
        if let Some(path) = node.child_by_field_name("path") {
            self.collect_scoped_parts(path, content, parts);
        }
        
        if let Some(name) = node.child_by_field_name("name") {
            parts.push(self.get_text(name, content));
        }
    }

    fn find_attribute<'a>(&self, node: Node<'a>, attr_name: &str, content: &str) -> Option<Node<'a>> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "attribute_item" {
                let attr_text = self.get_text(child, content);
                if attr_text.contains(attr_name) {
                    return Some(child);
                }
            }
        }
        None
    }

    fn is_public(&self, node: Node) -> bool {
        node.children(&mut node.walk())
            .any(|child| child.kind() == "visibility_modifier")
    }

    fn get_function_signature(&self, node: Node, content: &str) -> String {
        let mut sig = String::new();
        
        // Get function name
        if let Some(name_node) = node.child_by_field_name("name") {
            sig.push_str(&self.get_text(name_node, content));
        }
        
        // Get parameters
        if let Some(params_node) = node.child_by_field_name("parameters") {
            sig.push_str(&self.get_text(params_node, content));
        }
        
        // Get return type
        if let Some(return_type_node) = node.child_by_field_name("return_type") {
            sig.push_str(" -> ");
            if let Some(type_node) = return_type_node.child(1) {
                sig.push_str(&self.get_text(type_node, content));
            }
        }
        
        sig
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    x + y
}
"#;

        let (symbols, _, occurrences) = harness.parse("test.rs", content)?;

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "calculate");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[0].visibility, Some("public".to_string()));

        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].role, OccurrenceRole::Definition);

        Ok(())
    }

    #[test]
    fn test_parse_struct() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
pub struct Point {
    pub x: f64,
    y: f64,
}
"#;

        let (symbols, _, occurrences) = harness.parse("test.rs", content)?;

        assert_eq!(symbols.len(), 3); // struct + 2 fields
        
        let struct_sym = &symbols[0];
        assert_eq!(struct_sym.name, "Point");
        assert_eq!(struct_sym.kind, SymbolKind::Struct);
        assert_eq!(struct_sym.visibility, Some("public".to_string()));

        let field_x = &symbols[1];
        assert_eq!(field_x.name, "x");
        assert_eq!(field_x.kind, SymbolKind::Field);
        assert_eq!(field_x.visibility, Some("public".to_string()));

        let field_y = &symbols[2];
        assert_eq!(field_y.name, "y");
        assert_eq!(field_y.kind, SymbolKind::Field);
        assert_eq!(field_y.visibility, None);

        assert_eq!(occurrences.len(), 3);

        Ok(())
    }

    #[test]
    fn test_parse_enum() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
enum Color {
    Red,
    Green,
    Blue,
}
"#;

        let (symbols, _, occurrences) = harness.parse("test.rs", content)?;

        assert_eq!(symbols.len(), 4); // enum + 3 variants
        
        let enum_sym = &symbols[0];
        assert_eq!(enum_sym.name, "Color");
        assert_eq!(enum_sym.kind, SymbolKind::Enum);

        assert_eq!(symbols[1].name, "Red");
        assert_eq!(symbols[1].kind, SymbolKind::Enum);
        
        assert_eq!(occurrences.len(), 4);

        Ok(())
    }

    #[test]
    fn test_parse_impl() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
struct Circle {
    radius: f64,
}

impl Circle {
    pub fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}
"#;

        let (symbols, _, _) = harness.parse("test.rs", content)?;

        assert!(symbols.iter().any(|s| s.name == "Circle" && s.kind == SymbolKind::Struct));
        assert!(symbols.iter().any(|s| s.name == "area" && s.kind == SymbolKind::Method));

        Ok(())
    }

    #[test]
    fn test_parse_trait() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
pub trait Display {
    fn fmt(&self) -> String;
}
"#;

        let (symbols, _, _) = harness.parse("test.rs", content)?;

        assert!(symbols.iter().any(|s| s.name == "Display" && s.kind == SymbolKind::Trait));
        assert!(symbols.iter().any(|s| s.name == "fmt" && s.kind == SymbolKind::Function));

        Ok(())
    }

    #[test]
    fn test_parse_use_statement() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
use std::collections::HashMap;
use std::io::{Read, Write};
"#;

        let (_, edges, occurrences) = harness.parse("test.rs", content)?;

        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Imports));
        assert!(occurrences.iter().any(|o| o.role == OccurrenceRole::Reference));

        Ok(())
    }

    #[test]
    fn test_parse_module() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
mod utils {
    pub fn helper() {
        println!("Helper");
    }
}
"#;

        let (symbols, _, _) = harness.parse("test.rs", content)?;

        assert!(symbols.iter().any(|s| s.name == "utils" && s.kind == SymbolKind::Module));
        assert!(symbols.iter().any(|s| s.name == "helper" && s.fqn == "utils::helper"));

        Ok(())
    }

    #[test]
    fn test_parse_const_and_static() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
const PI: f64 = 3.14159;
static mut COUNTER: u32 = 0;
"#;

        let (symbols, _, _) = harness.parse("test.rs", content)?;

        assert!(symbols.iter().any(|s| s.name == "PI" && s.kind == SymbolKind::Constant));
        assert!(symbols.iter().any(|s| s.name == "COUNTER" && s.kind == SymbolKind::Constant));

        Ok(())
    }

    #[test]
    fn test_parse_type_alias() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
type Result<T> = std::result::Result<T, Error>;
"#;

        let (symbols, _, _) = harness.parse("test.rs", content)?;

        let type_alias = symbols.iter().find(|s| s.name == "Result").unwrap();
        assert_eq!(type_alias.kind, SymbolKind::Type);

        Ok(())
    }

    #[test]
    fn test_empty_file() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = "";

        let (symbols, edges, occurrences) = harness.parse("empty.rs", content)?;

        assert_eq!(symbols.len(), 0);
        assert_eq!(edges.len(), 0);
        assert_eq!(occurrences.len(), 0);

        Ok(())
    }

    #[test]
    fn test_malformed_rust() -> Result<()> {
        let mut harness = RustHarness::new()?;
        let content = r#"
fn broken {
    this is not valid rust
}
"#;

        // Should not panic, just return what it can parse
        let result = harness.parse("malformed.rs", content);
        assert!(result.is_ok());

        Ok(())
    }
}