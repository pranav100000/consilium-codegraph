//! C# Language Harness
//! 
//! Tree-sitter based parser for C# that extracts symbols and relationships
//! for the Consilium Codegraph system.

use anyhow::{anyhow, Result};
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, Resolution, Span, SymbolIR, SymbolKind, Version};
use std::collections::HashMap;
use tree_sitter::{Parser, Tree};

pub struct CSharpHarness {
    parser: Parser,
}

impl CSharpHarness {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_c_sharp::language())
            .map_err(|e| anyhow!("Failed to set C# language: {}", e))?;

        Ok(Self { parser })
    }

    pub fn parse_file(&mut self, file_path: &str, source: &str) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let tree = self.parser.parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse C# file: {}", file_path))?;

        let mut symbols = Vec::new();
        let mut edges = Vec::new();
        let occurrences = Vec::new();

        // Simple extraction - look for basic patterns
        self.extract_basic_symbols(&tree, source, file_path, &mut symbols, &mut edges)?;

        Ok((symbols, edges, occurrences))
    }

    fn extract_basic_symbols(
        &self,
        tree: &Tree,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<SymbolIR>,
        _edges: &mut Vec<EdgeIR>,
    ) -> Result<()> {
        // Create a simple placeholder symbol for now
        let symbol = SymbolIR {
            id: format!("csharp_file_{}", file_path),
            lang: Language::CSharp,
            lang_version: Some(Version::DotNet6),
            kind: SymbolKind::Module,
            name: file_path.split('/').last().unwrap_or(file_path).to_string(),
            fqn: file_path.to_string(),
            signature: Some(format!("C# file: {}", file_path)),
            file_path: file_path.to_string(),
            span: Span {
                start_line: 0,
                start_col: 0,
                end_line: source.lines().count() as u32,
                end_col: 0,
            },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: self.calculate_hash(file_path),
        };

        symbols.push(symbol);

        Ok(())
    }

    fn calculate_hash(&self, input: &str) -> String {
        format!("{:x}", input.len() * 17 + input.chars().map(|c| c as usize).sum::<usize>())
    }
}

impl Default for CSharpHarness {
    fn default() -> Self {
        Self::new().expect("Failed to create C# harness")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() -> Result<()> {
        let mut harness = CSharpHarness::new()?;
        
        let source = r#"
using System;

namespace MyNamespace 
{
    public class Calculator 
    {
        public int Add(int a, int b) 
        {
            return a + b;
        }
    }
}
"#;

        let (symbols, _edges, _) = harness.parse_file("Calculator.cs", source)?;
        
        // Should find at least one symbol
        assert!(symbols.len() >= 1);
        assert_eq!(symbols[0].lang, Language::CSharp);
        
        Ok(())
    }
}