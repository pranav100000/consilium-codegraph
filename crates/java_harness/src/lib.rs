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
            "record_declaration" => {
                self.handle_record(node, content, file_path, symbols, edges, occurrences, context)?;
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
            "annotation" | "marker_annotation" => {
                self.handle_annotation_usage(node, content, file_path, occurrences)?;
            }
            "method_invocation" => {
                self.handle_method_call(node, content, file_path, edges, occurrences)?;
            }
            "lambda_expression" => {
                self.handle_lambda(node, content, file_path, symbols, occurrences, context)?;
            }
            "method_reference" => {
                self.handle_method_reference(node, content, file_path, edges, occurrences)?;
            }
            "static_initializer" => {
                self.handle_static_initializer(node, content, file_path, symbols, occurrences, context)?;
            }
            "instance_initializer" | "block" => {
                // Check if this is an instance initializer (block at class level)
                if context.current_class().is_some() {
                    self.handle_instance_initializer(node, content, file_path, symbols, occurrences, context)?;
                } else {
                    // Regular block, walk children
                    for child in node.children(&mut node.walk()) {
                        self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                    }
                }
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

            // Don't create occurrences for imports - they're not symbols
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

        // Build signature with generic type parameters
        let mut signature = String::new();
        if let Some(type_params_node) = node.child_by_field_name("type_parameters") {
            signature.push('<');
            let mut type_params = Vec::new();
            for child in type_params_node.children(&mut type_params_node.walk()) {
                if child.kind() == "type_parameter" {
                    let mut param = String::new();
                    if let Some(name_node) = child.child_by_field_name("name") {
                        param = self.get_text(name_node, content);
                    }
                    // Check for bounds (extends clause)
                    for bound_child in child.children(&mut child.walk()) {
                        if bound_child.kind() == "type_bound" {
                            param.push_str(" extends ");
                            let bounds = bound_child.children(&mut bound_child.walk())
                                .filter(|n| n.kind() != "extends")
                                .map(|n| self.get_text(n, content))
                                .collect::<Vec<_>>()
                                .join(" & ");
                            param.push_str(&bounds);
                        }
                    }
                    type_params.push(param);
                }
            }
            signature.push_str(&type_params.join(", "));
            signature.push('>');
        }

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            lang_version: None,
            kind: SymbolKind::Class,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: if signature.is_empty() { None } else { Some(signature) },
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if is_public { Some("public".to_string()) } else { None },
            doc: self.get_preceding_comment(node, content),
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

        // Build signature with generic type parameters
        let mut signature = String::new();
        if let Some(type_params_node) = node.child_by_field_name("type_parameters") {
            signature.push('<');
            let mut type_params = Vec::new();
            for child in type_params_node.children(&mut type_params_node.walk()) {
                if child.kind() == "type_parameter" {
                    let mut param = String::new();
                    if let Some(name_node) = child.child_by_field_name("name") {
                        param = self.get_text(name_node, content);
                    }
                    // Check for bounds (extends clause)
                    for bound_child in child.children(&mut child.walk()) {
                        if bound_child.kind() == "type_bound" {
                            param.push_str(" extends ");
                            let bounds = bound_child.children(&mut bound_child.walk())
                                .filter(|n| n.kind() != "extends")
                                .map(|n| self.get_text(n, content))
                                .collect::<Vec<_>>()
                                .join(" & ");
                            param.push_str(&bounds);
                        }
                    }
                    type_params.push(param);
                }
            }
            signature.push_str(&type_params.join(", "));
            signature.push('>');
        }

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            lang_version: None,
            kind: SymbolKind::Interface,
            name: name.clone(),
            fqn: fqn.clone(),
            signature: if signature.is_empty() { None } else { Some(signature) },
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

        // Handle extended interfaces (interface A extends B, C)
        // Look for extends_interfaces child node
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "extends_interfaces" {
                    // Found the extends clause
                    for type_child in child.children(&mut child.walk()) {
                        if type_child.kind() == "type_identifier" || type_child.kind() == "scoped_type_identifier" {
                            let extended_interface = self.get_text(type_child, content);
                            edges.push(EdgeIR {
                                edge_type: EdgeType::Extends,
                                src: Some(symbol.id.clone()),
                                dst: Some(extended_interface),
                                file_src: Some(file_path.to_string()),
                                file_dst: None,
                                resolution: protocol::Resolution::Syntactic,
                                meta: HashMap::new(),
                                provenance: HashMap::new(),
                            });
                        } else if type_child.kind() == "type_list" {
                            // Sometimes the interfaces are in a type_list
                            for interface_node in type_child.children(&mut type_child.walk()) {
                                if interface_node.kind() == "type_identifier" || interface_node.kind() == "scoped_type_identifier" {
                                    let extended_interface = self.get_text(interface_node, content);
                                    edges.push(EdgeIR {
                                        edge_type: EdgeType::Extends,
                                        src: Some(symbol.id.clone()),
                                        dst: Some(extended_interface),
                                        file_src: Some(file_path.to_string()),
                                        file_dst: None,
                                        resolution: protocol::Resolution::Syntactic,
                                        meta: HashMap::new(),
                                        provenance: HashMap::new(),
                                    });
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }

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
            lang_version: None,
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
                lang_version: None,
                kind: SymbolKind::EnumMember,
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
        let sig_hash = format!("{:x}", md5::compute(format!("{}{}", fqn, signature)));

        let modifiers = self.get_modifiers(node, content);
        let is_public = modifiers.iter().any(|m| m == "public");
        let is_protected = modifiers.iter().any(|m| m == "protected");
        let is_private = modifiers.iter().any(|m| m == "private");
        let is_static = modifiers.iter().any(|m| m == "static");
        let is_abstract = modifiers.iter().any(|m| m == "abstract");
        let is_final = modifiers.iter().any(|m| m == "final");

        // Determine visibility
        let visibility = if is_public {
            Some("public".to_string())
        } else if is_protected {
            Some("protected".to_string())
        } else if is_private {
            Some("private".to_string())
        } else {
            Some("package".to_string()) // Default package-private visibility in Java
        };

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
            lang_version: None,
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
            visibility,
            doc: self.get_preceding_comment(node, content),
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
                    let is_protected = modifiers.iter().any(|m| m == "protected");
                    let is_private = modifiers.iter().any(|m| m == "private");
                    let is_static = modifiers.iter().any(|m| m == "static");
                    let is_final = modifiers.iter().any(|m| m == "final");

                    // Determine visibility
                    let visibility = if is_public {
                        Some("public".to_string())
                    } else if is_protected {
                        Some("protected".to_string())
                    } else if is_private {
                        Some("private".to_string())
                    } else {
                        Some("package".to_string()) // Default package-private visibility in Java
                    };

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
                        lang_version: None,
                        kind: SymbolKind::Field,
                        name: name.clone(),
                        fqn,
                        signature: None,
                        file_path: file_path.to_string(),
                        span: self.node_to_span(name_node),
                        visibility,
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

    fn handle_record(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        let name_node = node.child_by_field_name("name").context("Record without name")?;
        let name = self.get_text(name_node, content);

        let fqn = context.build_fqn(&name);
        let sig_hash = format!("{:x}", md5::compute(&fqn));

        let modifiers = self.get_modifiers(node, content);
        let is_public = modifiers.iter().any(|m| m == "public");

        // Get record parameters (components)
        let mut params = Vec::new();
        if let Some(param_list) = node.child_by_field_name("parameters") {
            for param in param_list.children(&mut param_list.walk()) {
                if param.kind() == "formal_parameter" || param.kind() == "record_component" {
                    let param_text = self.get_text(param, content);
                    params.push(param_text);
                }
            }
        }
        
        let signature = format!("record {}({})", name, params.join(", "));

        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            lang_version: None,
            kind: SymbolKind::Class, // Records are like classes
            name: name.clone(),
            fqn: fqn.clone(),
            signature: Some(signature),
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
        
        // Create canonical constructor for the record
        // Records automatically have a public constructor with all components as parameters
        context.push_class(name.clone());
        let constructor_fqn = context.build_fqn(&name);
        let constructor_sig_hash = format!("{:x}", md5::compute(format!("{}({})", constructor_fqn, params.join(", "))));
        
        let constructor_symbol = SymbolIR {
            id: format!("{}#{}_constructor", file_path, constructor_fqn),
            lang: ProtoLanguage::Java,
            lang_version: None,
            kind: SymbolKind::Method,
            name: name.clone(),
            fqn: constructor_fqn,
            signature: Some(format!("{}({})", name, params.join(", "))),
            file_path: file_path.to_string(),
            span: self.node_to_span(name_node),
            visibility: if is_public { Some("public".to_string()) } else { None },
            doc: None,
            sig_hash: constructor_sig_hash,
        };
        
        symbols.push(constructor_symbol);
        
        // Handle interfaces (implements clause)  
        if let Some(interfaces) = node.child_by_field_name("interfaces") {
            // The interfaces field contains a super_interfaces node
            if interfaces.kind() == "super_interfaces" {
                // Find the type_list child which contains the interface types
                for child in interfaces.children(&mut interfaces.walk()) {
                    if child.kind() == "type_list" {
                        // Iterate through all interface types in the type_list
                        for type_child in child.children(&mut child.walk()) {
                            if type_child.kind() == "type_identifier" || type_child.kind() == "generic_type" || type_child.kind() == "scoped_type_identifier" {
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

        // Process record body (methods, compact constructor, etc)
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                match child.kind() {
                    "method_declaration" => {
                        self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                    }
                    "constructor_declaration" | "compact_constructor_declaration" => {
                        // Handle explicit or compact constructor
                        self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                    }
                    _ => {
                        // Walk other nodes too
                        self.walk_node(child, content, file_path, symbols, edges, occurrences, context)?;
                    }
                }
            }
        }
        context.pop_class();

        Ok(())
    }

    fn handle_annotation_usage(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        // Extract annotation name
        let annotation_name = if let Some(name_node) = node.child_by_field_name("name") {
            self.get_text(name_node, content)
        } else {
            // For marker annotations, get the identifier directly
            for child in node.children(&mut node.walk()) {
                if child.kind() == "identifier" {
                    return Ok({
                        let name = self.get_text(child, content);
                        occurrences.push(OccurrenceIR {
                            file_path: file_path.to_string(),
                            symbol_id: Some(format!("@{}", name)),
                            role: OccurrenceRole::Reference,
                            span: self.node_to_span(child),
                            token: format!("@{}", name),
                        });
                    });
                }
            }
            return Ok(());
        };
        
        // Create reference occurrence for the annotation
        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(format!("@{}", annotation_name)),
            role: OccurrenceRole::Reference,
            span: self.node_to_span(node),
            token: format!("@{}", annotation_name),
        });
        
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
            lang_version: None,
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
            symbol_id: Some(symbol.id.clone()),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(name_node),
            token: name.clone(),
        });

        // Process annotation body for annotation methods (element declarations)
        context.push_class(name.clone());
        if let Some(body) = node.child_by_field_name("body") {
            for child in body.children(&mut body.walk()) {
                if child.kind() == "annotation_type_element_declaration" || 
                   child.kind() == "method_declaration" ||
                   child.kind() == "element_value_pair" {
                    // Handle annotation method/element
                    self.handle_annotation_method(child, content, file_path, symbols, occurrences, context)?;
                } else {
                    self.walk_node(child, content, file_path, symbols, &mut Vec::new(), occurrences, context)?;
                }
            }
        }
        context.pop_class();

        Ok(())
    }
    
    fn handle_annotation_method(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Get method name
        let name_node = node.child_by_field_name("name");
        if let Some(name_node) = name_node {
            let name = self.get_text(name_node, content);
            let fqn = context.build_fqn(&name);
            let sig_hash = format!("{:x}", md5::compute(&fqn));
            
            // Get return type if available
            let return_type = node.child_by_field_name("type")
                .map(|n| self.get_text(n, content))
                .unwrap_or_else(|| "String".to_string());
            
            // Check for default value
            let has_default = node.children(&mut node.walk())
                .any(|child| child.kind() == "default" || child.kind() == "element_value");
            
            let signature = if has_default {
                format!("{} {}() default ...", return_type, name)
            } else {
                format!("{} {}()", return_type, name)
            };
            
            let symbol = SymbolIR {
                id: format!("{}#{}", file_path, fqn),
                lang: ProtoLanguage::Java,
                lang_version: None,
                kind: SymbolKind::Method,
                name: name.clone(),
                fqn,
                signature: Some(signature),
                file_path: file_path.to_string(),
                span: self.node_to_span(name_node),
                visibility: Some("public".to_string()), // Annotation methods are implicitly public
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

    fn handle_lambda(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Create a unique ID for the lambda
        let lambda_id = format!("lambda_{}", node.start_position().row);
        let fqn = context.build_fqn(&lambda_id);
        let sig_hash = format!("{:x}", md5::compute(&fqn));
        
        // Extract parameters
        let mut params = Vec::new();
        if let Some(params_node) = node.child_by_field_name("parameters") {
            for child in params_node.children(&mut params_node.walk()) {
                if child.kind() == "identifier" || child.kind() == "formal_parameter" {
                    params.push(self.get_text(child, content));
                }
            }
        }
        
        // Build signature
        let signature = format!("({}) -> {{...}}", params.join(", "));
        
        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            lang_version: None,
            kind: SymbolKind::Function, // Lambdas are anonymous functions
            name: lambda_id.clone(),
            fqn,
            signature: Some(signature),
            file_path: file_path.to_string(),
            span: self.node_to_span(node),
            visibility: None,
            doc: None,
            sig_hash,
        };
        
        symbols.push(symbol.clone());
        
        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(node),
            token: lambda_id,
        });
        
        // Walk the body to find any calls or references inside
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_node(body, content, file_path, symbols, &mut Vec::new(), occurrences, context)?;
        }
        
        Ok(())
    }
    
    fn handle_method_reference(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        edges: &mut Vec<EdgeIR>,
        occurrences: &mut Vec<OccurrenceIR>,
    ) -> Result<()> {
        // Method references like String::toUpperCase or System.out::println
        let full_text = self.get_text(node, content);
        let from_id = format!("{}#{}", file_path, self.get_file_fqn(file_path));
        
        // Split on :: to get the method name
        let parts: Vec<&str> = full_text.split("::").collect();
        if parts.len() == 2 {
            let method_name = parts[1];
            
            edges.push(EdgeIR {
                edge_type: EdgeType::Calls,
                src: Some(from_id),
                dst: Some(method_name.to_string()),
                file_src: Some(file_path.to_string()),
                file_dst: None,
                resolution: protocol::Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            });
            
            occurrences.push(OccurrenceIR {
                file_path: file_path.to_string(),
                symbol_id: Some(full_text.clone()),
                role: OccurrenceRole::Reference,
                span: self.node_to_span(node),
                token: full_text,
            });
        }
        
        Ok(())
    }
    
    fn handle_static_initializer(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Create a symbol for the static initializer block
        let class_name = context.current_class().unwrap_or_else(|| "Unknown".to_string());
        let block_id = format!("static_init_{}", node.start_position().row);
        let fqn = context.build_fqn(&block_id);
        let sig_hash = format!("{:x}", md5::compute(&fqn));
        
        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            lang_version: None,
            kind: SymbolKind::Method, // Static initializers are like special methods
            name: "<clinit>".to_string(), // Java bytecode name for static initializer
            fqn,
            signature: Some("static {}".to_string()),
            file_path: file_path.to_string(),
            span: self.node_to_span(node),
            visibility: None, // Static initializers have no visibility modifier
            doc: None,
            sig_hash,
        };
        
        symbols.push(symbol.clone());
        
        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(node),
            token: "static".to_string(),
        });
        
        // Walk the body to find any method calls or references
        for child in node.children(&mut node.walk()) {
            if child.kind() != "static" && child.kind() != "{" && child.kind() != "}" {
                self.walk_node(child, content, file_path, symbols, &mut Vec::new(), occurrences, context)?;
            }
        }
        
        Ok(())
    }
    
    fn handle_instance_initializer(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        occurrences: &mut Vec<OccurrenceIR>,
        context: &mut ParseContext,
    ) -> Result<()> {
        // Create a symbol for the instance initializer block
        let class_name = context.current_class().unwrap_or_else(|| "Unknown".to_string());
        let block_id = format!("instance_init_{}", node.start_position().row);
        let fqn = context.build_fqn(&block_id);
        let sig_hash = format!("{:x}", md5::compute(&fqn));
        
        let symbol = SymbolIR {
            id: format!("{}#{}", file_path, fqn),
            lang: ProtoLanguage::Java,
            lang_version: None,
            kind: SymbolKind::Method, // Instance initializers are like special methods
            name: "<init>".to_string(), // Java bytecode name for instance initializer
            fqn,
            signature: Some("{}".to_string()),
            file_path: file_path.to_string(),
            span: self.node_to_span(node),
            visibility: None, // Instance initializers have no visibility modifier
            doc: None,
            sig_hash,
        };
        
        symbols.push(symbol.clone());
        
        occurrences.push(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(symbol.id),
            role: OccurrenceRole::Definition,
            span: self.node_to_span(node),
            token: "{".to_string(),
        });
        
        // Walk the body
        for child in node.children(&mut node.walk()) {
            if child.kind() != "{" && child.kind() != "}" {
                self.walk_node(child, content, file_path, symbols, &mut Vec::new(), occurrences, context)?;
            }
        }
        
        Ok(())
    }

    // Helper methods

    fn get_text(&self, node: Node, content: &str) -> String {
        content[node.byte_range()].to_string()
    }
    
    fn get_preceding_comment(&self, node: Node, content: &str) -> Option<String> {
        // Look for a comment immediately before this node
        if let Some(parent) = node.parent() {
            let node_start = node.start_position().row;
            
            // Check siblings before this node
            for i in 0..parent.child_count() {
                if let Some(child) = parent.child(i) {
                    // If we've reached our node, stop
                    if child.id() == node.id() {
                        break;
                    }
                    
                    // Check if this is a comment that ends right before our node
                    if child.kind() == "line_comment" || child.kind() == "block_comment" {
                        let comment_end = child.end_position().row;
                        // Comment should be on the line immediately before or same line
                        if comment_end == node_start || comment_end == node_start - 1 {
                            let comment_text = self.get_text(child, content);
                            // Clean up the comment
                            return Some(self.clean_comment(comment_text));
                        }
                    }
                }
            }
        }
        None
    }
    
    fn clean_comment(&self, comment: String) -> String {
        let comment = comment.trim();
        
        // Remove /** and */ for block comments
        let comment = if comment.starts_with("/**") && comment.ends_with("*/") {
            comment[3..comment.len()-2].trim().to_string()
        } else if comment.starts_with("/*") && comment.ends_with("*/") {
            comment[2..comment.len()-2].trim().to_string()
        } else if comment.starts_with("//") {
            comment[2..].trim().to_string()
        } else {
            comment.to_string()
        };
        
        // Remove leading asterisks from each line (common in Javadoc)
        comment.lines()
            .map(|line| {
                let line = line.trim();
                if line.starts_with("* ") {
                    &line[2..]
                } else if line.starts_with("*") {
                    &line[1..]
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
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

        // Generic type parameters
        if let Some(type_params_node) = node.child_by_field_name("type_parameters") {
            sig.push('<');
            let mut type_params = Vec::new();
            for child in type_params_node.children(&mut type_params_node.walk()) {
                if child.kind() == "type_parameter" {
                    let mut param = String::new();
                    if let Some(name_node) = child.child_by_field_name("name") {
                        param = self.get_text(name_node, content);
                    }
                    // Check for bounds (extends clause)
                    for bound_child in child.children(&mut child.walk()) {
                        if bound_child.kind() == "type_bound" {
                            param.push_str(" extends ");
                            let bounds = bound_child.children(&mut bound_child.walk())
                                .filter(|n| n.kind() != "extends")
                                .map(|n| self.get_text(n, content))
                                .collect::<Vec<_>>()
                                .join(" & ");
                            param.push_str(&bounds);
                        }
                    }
                    type_params.push(param);
                }
            }
            sig.push_str(&type_params.join(", "));
            sig.push_str("> ");
        }

        // Method name
        if let Some(name_node) = node.child_by_field_name("name") {
            sig.push_str(&self.get_text(name_node, content));
        } else if node.kind() == "constructor_declaration" {
            sig.push_str("<init>");
        }

        // Parameters - extract just the types for signature
        sig.push('(');
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut param_types = Vec::new();
            for child in params_node.children(&mut params_node.walk()) {
                if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
                    // Get the type of the parameter
                    if let Some(type_node) = child.child_by_field_name("type") {
                        let param_type = self.get_text(type_node, content);
                        if child.kind() == "spread_parameter" {
                            param_types.push(format!("{}...", param_type));
                        } else {
                            param_types.push(param_type);
                        }
                    }
                }
            }
            sig.push_str(&param_types.join(", "));
        }
        sig.push(')');

        // Return type
        if let Some(type_node) = node.child_by_field_name("type") {
            sig.push_str(" : ");
            sig.push_str(&self.get_text(type_node, content));
        }
        
        // Exception specifications (throws clause)
        if let Some(throws_node) = node.child_by_field_name("throws") {
            sig.push_str(" throws ");
            let mut exceptions = Vec::new();
            for child in throws_node.children(&mut throws_node.walk()) {
                if child.kind() == "type_identifier" || child.kind() == "scoped_type_identifier" {
                    exceptions.push(self.get_text(child, content));
                }
            }
            sig.push_str(&exceptions.join(", "));
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
mod complex_tests;
#[cfg(test)]
mod strict_tests;
#[cfg(test)]
mod error_handling_tests;
#[cfg(test)]
mod stress_tests;
#[cfg(test)]
mod edge_case_extreme_tests;

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
        assert!(symbols.iter().any(|s| s.name == "RED" && s.kind == SymbolKind::EnumMember));
        assert!(symbols.iter().any(|s| s.name == "GREEN" && s.kind == SymbolKind::EnumMember));
        assert!(symbols.iter().any(|s| s.name == "BLUE" && s.kind == SymbolKind::EnumMember));
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

        let (symbols, edges, occurrences) = harness.parse("Test.java", content)?;

        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Imports));
        // Import statements don't create occurrences - they're just edges
        // Only the Test class should create an occurrence
        assert_eq!(occurrences.len(), 1);
        assert!(occurrences[0].token == "Test");
        assert_eq!(occurrences[0].role, OccurrenceRole::Definition);

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
    fn test_documentation_comments() -> Result<()> {
        let mut harness = JavaHarness::new()?;
        let source = r#"
/**
 * This is a test class
 * with multiple lines
 */
public class TestClass {
    /**
     * This is a test method
     * @param x the input value
     * @return the result
     */
    public int testMethod(int x) {
        return x * 2;
    }
    
    // Single line comment
    private String field;
}
"#;
        
        let (symbols, _, _) = harness.parse("test.java", source)?;
        
        // Find the class and check its doc
        let test_class = symbols.iter().find(|s| s.name == "TestClass");
        assert!(test_class.is_some());
        let class_doc = test_class.unwrap().doc.as_ref();
        assert!(class_doc.is_some());
        assert!(class_doc.unwrap().contains("test class"));
        
        // Find the method and check its doc
        let test_method = symbols.iter().find(|s| s.name == "testMethod");
        assert!(test_method.is_some());
        let method_doc = test_method.unwrap().doc.as_ref();
        assert!(method_doc.is_some());
        assert!(method_doc.unwrap().contains("test method"));
        
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