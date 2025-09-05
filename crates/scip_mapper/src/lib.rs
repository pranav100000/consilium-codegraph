use anyhow::Result;
use protocol::{EdgeIR, EdgeType, OccurrenceIR, OccurrenceRole, Resolution, SymbolIR, SymbolKind, Language, Span};
use std::collections::HashMap;
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScipIndex {
    pub metadata: ScipMetadata,
    pub documents: Vec<ScipDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScipMetadata {
    pub version: String,
    pub tool_info: ScipToolInfo,
    pub project_root: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScipToolInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScipDocument {
    pub relative_path: String,
    pub symbols: Vec<ScipSymbol>,
    pub occurrences: Vec<ScipOccurrence>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScipSymbol {
    pub symbol: String,
    pub documentation: Option<Vec<String>>,
    pub relationships: Option<Vec<ScipRelationship>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScipRelationship {
    pub symbol: String,
    pub is_implementation: bool,
    pub is_reference: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScipOccurrence {
    pub range: Vec<i32>,
    pub symbol: String,
    pub symbol_roles: i32,
}

pub struct ScipMapper {
    provenance: HashMap<String, String>,
    scip_cli_path: String,
}

impl ScipMapper {
    pub fn new(indexer_name: &str, indexer_version: &str) -> Self {
        let mut provenance = HashMap::new();
        provenance.insert("source".to_string(), format!("{}@{}", indexer_name, indexer_version));
        
        Self { 
            provenance,
            scip_cli_path: "./scip".to_string(), // Default path
        }
    }
    
    pub fn with_scip_cli_path(mut self, path: String) -> Self {
        self.scip_cli_path = path;
        self
    }
    
    pub fn run_scip_typescript(&self, project_path: &str) -> Result<String> {
        info!("Running scip-typescript on {}", project_path);
        
        let output = Command::new("scip-typescript")
            .arg("index")
            .current_dir(project_path)
            .output()?;
        
        if !output.status.success() {
            anyhow::bail!("scip-typescript failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        Ok(format!("{}/index.scip", project_path))
    }
    
    pub fn parse_scip_index(&self, scip_file: &str) -> Result<ScipIndex> {
        info!("Parsing SCIP index from {}", scip_file);
        
        // Use the scip CLI to convert to JSON
        let output = Command::new(&self.scip_cli_path)
            .args(["print", "--json", scip_file])
            .output()?;
        
        if !output.status.success() {
            anyhow::bail!("scip print failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let json_str = String::from_utf8(output.stdout)?;
        let index: ScipIndex = serde_json::from_str(&json_str)?;
        
        Ok(index)
    }
    
    pub fn map_scip_to_ir(
        &self,
        scip_index: &ScipIndex,
        commit_sha: &str,
    ) -> Result<(Vec<SymbolIR>, Vec<EdgeIR>, Vec<OccurrenceIR>)> {
        let mut symbols = vec![];
        let mut edges = vec![];
        let mut occurrences = vec![];
        
        for doc in &scip_index.documents {
            // Process symbols
            for scip_sym in &doc.symbols {
                if let Some(symbol) = self.convert_symbol(scip_sym, &doc.relative_path, commit_sha) {
                    symbols.push(symbol);
                    
                    // Process relationships as edges
                    if let Some(rels) = &scip_sym.relationships {
                        for rel in rels {
                            if let Some(edge) = self.convert_relationship(&scip_sym.symbol, &rel.symbol, rel.is_implementation) {
                                edges.push(edge);
                            }
                        }
                    }
                }
            }
            
            // Process occurrences
            for scip_occ in &doc.occurrences {
                if let Some(occ) = self.convert_occurrence(scip_occ, &doc.relative_path) {
                    occurrences.push(occ);
                }
            }
        }
        
        Ok((symbols, edges, occurrences))
    }
    
    fn convert_symbol(&self, scip_sym: &ScipSymbol, file_path: &str, commit_sha: &str) -> Option<SymbolIR> {
        // Parse SCIP symbol string (e.g., "scip-typescript npm . . `main.ts`/createTestUser().")
        let parts: Vec<&str> = scip_sym.symbol.split_whitespace().collect();
        if parts.len() < 5 {
            return None;
        }
        
        // Extract name from the symbol path
        let symbol_path = parts.last()?;
        let name = symbol_path
            .trim_end_matches('.')
            .trim_end_matches("()")
            .trim_end_matches('#')
            .split('/')
            .next_back()?
            .to_string();
        
        // Determine kind based on symbol format
        let kind = if symbol_path.contains("#") {
            SymbolKind::Class
        } else if symbol_path.contains("().") {
            SymbolKind::Function
        } else if symbol_path.ends_with("()") {
            SymbolKind::Method
        } else {
            SymbolKind::Variable
        };
        
        let fqn = format!("{}.{}", file_path.trim_end_matches(".ts").trim_end_matches(".tsx"), name);
        let sig_hash = format!("{:x}", name.len());
        let id = SymbolIR::generate_id(commit_sha, file_path, &Language::TypeScript, &fqn, &sig_hash);
        
        Some(SymbolIR {
            id,
            lang: Language::TypeScript,
            kind,
            name,
            fqn,
            signature: None,
            file_path: file_path.to_string(),
            span: Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 }, // Will be filled from occurrences
            visibility: None,
            doc: scip_sym.documentation.as_ref().map(|d| d.join("\n")),
            sig_hash,
        })
    }
    
    fn convert_relationship(&self, from_symbol: &str, to_symbol: &str, is_implementation: bool) -> Option<EdgeIR> {
        let edge_type = if is_implementation {
            EdgeType::Implements
        } else {
            EdgeType::Calls
        };
        
        Some(EdgeIR {
            edge_type,
            src: Some(from_symbol.to_string()),
            dst: Some(to_symbol.to_string()),
            file_src: None,
            file_dst: None,
            resolution: Resolution::Semantic,
            meta: HashMap::new(),
            provenance: self.provenance.clone(),
        })
    }
    
    fn convert_occurrence(&self, scip_occ: &ScipOccurrence, file_path: &str) -> Option<OccurrenceIR> {
        // SCIP range format: [startLine, startCol, endLine, endCol]
        if scip_occ.range.len() != 4 {
            return None;
        }
        
        let role = match scip_occ.symbol_roles {
            1 => OccurrenceRole::Definition,
            2 => OccurrenceRole::Reference,
            4 => OccurrenceRole::Write,
            _ => OccurrenceRole::Reference,
        };
        
        Some(OccurrenceIR {
            file_path: file_path.to_string(),
            symbol_id: Some(scip_occ.symbol.clone()),
            role,
            span: Span {
                start_line: scip_occ.range[0] as u32,
                start_col: scip_occ.range[1] as u32,
                end_line: scip_occ.range[2] as u32,
                end_col: scip_occ.range[3] as u32,
            },
            token: String::new(), // Would need to extract from source
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_mapper() {
        let mapper = ScipMapper::new("scip-typescript", "1.0.0");
        assert!(mapper.provenance.contains_key("source"));
    }
}