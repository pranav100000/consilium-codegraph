use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, Resolution};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use store::GraphStore;
use tracing::{debug, info};
use scip_mapper::ScipMapper;
use crate::language_strategy::{LanguageStrategyRegistry, LanguageStrategy};

pub struct ResolutionEngine {
    pub store: GraphStore,
    scip_mapper: ScipMapper,
    language_registry: LanguageStrategyRegistry,
}

impl ResolutionEngine {
    pub fn new(store: GraphStore) -> Self {
        let scip_mapper = ScipMapper::new("consilium", "0.1.0")
            .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
        let language_registry = LanguageStrategyRegistry::new();
        
        Self { 
            store, 
            scip_mapper,
            language_registry,
        }
    }
    
    pub async fn resolve_project(&mut self, project_path: &Path, commit_sha: &str) -> Result<()> {
        info!("Starting cross-file symbol resolution for project");
        
        // Detect languages using strategy registry
        let strategies = self.language_registry.detect_languages(project_path);
        
        if strategies.is_empty() {
            info!("No supported languages detected in project");
            return Ok(());
        }
        
        info!("Detected languages: {}", strategies.iter()
            .map(|s| s.name())
            .collect::<Vec<_>>()
            .join(", "));
        
        // Process each detected language
        for strategy in strategies {
            info!("Processing {} files", strategy.name());
            
            match self.process_language_strategy(strategy, project_path, commit_sha).await {
                Ok(()) => {
                    info!("Successfully processed {} semantic analysis", strategy.name());
                }
                Err(e) => {
                    info!("Failed to process {} semantic analysis: {}", strategy.name(), e);
                    // Continue with other languages even if one fails
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_language_strategy(
        &mut self, 
        strategy: &dyn LanguageStrategy, 
        project_path: &Path, 
        commit_sha: &str
    ) -> Result<()> {
        // Run the language-specific indexer
        let scip_file = strategy.run_indexer(&self.scip_mapper, project_path)?;
        
        // Parse SCIP index
        let scip_index = self.scip_mapper.parse_scip_index(&scip_file.to_string_lossy())?;
        
        // Convert to IR
        let (symbols, edges, occurrences) = self.scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
        
        // Store semantic data
        let commit_id = self.store.get_or_create_commit(commit_sha)?;
        
        // Insert symbols
        for symbol in &symbols {
            if let Err(e) = self.store.insert_symbol(commit_id, symbol) {
                debug!("Failed to insert symbol {}: {}", symbol.id, e);
            }
        }
        
        // Insert edges
        for edge in &edges {
            if let Err(e) = self.store.insert_edge(commit_id, edge) {
                debug!("Failed to insert edge: {}", e);
            }
        }
        
        // Insert occurrences
        for occurrence in &occurrences {
            if let Err(e) = self.store.insert_occurrence(commit_id, occurrence) {
                debug!("Failed to insert occurrence: {}", e);
            }
        }
        
        info!("Stored {} symbols, {} edges, {} occurrences for {}", 
              symbols.len(), edges.len(), occurrences.len(), strategy.name());
        
        Ok(())
    }
    
    async fn resolve_typescript_project(&mut self, project_path: &Path, commit_sha: &str) -> Result<()> {
        info!("Resolving TypeScript/JavaScript symbols using SCIP");
        
        // Run scip-typescript indexer
        let scip_file = self.scip_mapper.run_scip_typescript(project_path.to_str().unwrap())?;
        
        // Parse SCIP index
        let scip_index = self.scip_mapper.parse_scip_index(&scip_file)?;
        
        // Convert to IR and store
        let (symbols, edges, occurrences) = self.scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
        
        // Store semantic data
        // Store semantic data with a dummy commit_id (0 for now)
        for symbol in &symbols {
            self.store.insert_symbol(0, symbol)?;
        }
        for edge in &edges {
            self.store.insert_edge(0, edge)?;
        }
        for occurrence in &occurrences {
            self.store.insert_occurrence(0, occurrence)?;
        }
        
        info!("Stored {} symbols, {} edges, {} occurrences from TypeScript", 
              symbols.len(), edges.len(), occurrences.len());
        
        Ok(())
    }
    
    async fn resolve_python_project(&mut self, project_path: &Path, commit_sha: &str) -> Result<()> {
        info!("Resolving Python symbols using SCIP");
        
        // Check if scip-python is available
        if let Ok(output) = std::process::Command::new("scip-python")
            .arg("--version")
            .output() 
        {
            if output.status.success() {
                // Run scip-python indexer
                let output = std::process::Command::new("scip-python")
                    .arg("index")
                    .current_dir(project_path)
                    .output()?;
                
                if output.status.success() {
                    let scip_file = project_path.join("index.scip");
                    if scip_file.exists() {
                        let scip_index = self.scip_mapper.parse_scip_index(scip_file.to_str().unwrap())?;
                        let (symbols, edges, occurrences) = self.scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
                        
                        for symbol in &symbols {
                            self.store.insert_symbol(0, symbol)?;
                        }
                        for edge in &edges {
                            self.store.insert_edge(0, edge)?;
                        }
                        for occurrence in &occurrences {
                            self.store.insert_occurrence(0, occurrence)?;
                        }
                        
                        info!("Stored {} symbols, {} edges, {} occurrences from Python", 
                              symbols.len(), edges.len(), occurrences.len());
                    }
                }
            }
        } else {
            debug!("scip-python not available, skipping Python semantic analysis");
        }
        
        Ok(())
    }
    
    async fn resolve_go_project(&mut self, project_path: &Path, commit_sha: &str) -> Result<()> {
        info!("Resolving Go symbols using SCIP");
        
        // Check if scip-go is available
        if let Ok(output) = std::process::Command::new("scip-go")
            .arg("--version")
            .output() 
        {
            if output.status.success() {
                // Run scip-go indexer
                let output = std::process::Command::new("scip-go")
                    .current_dir(project_path)
                    .output()?;
                
                if output.status.success() {
                    let scip_file = project_path.join("index.scip");
                    if scip_file.exists() {
                        let scip_index = self.scip_mapper.parse_scip_index(scip_file.to_str().unwrap())?;
                        let (symbols, edges, occurrences) = self.scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
                        
                        for symbol in &symbols {
                            self.store.insert_symbol(0, symbol)?;
                        }
                        for edge in &edges {
                            self.store.insert_edge(0, edge)?;
                        }
                        for occurrence in &occurrences {
                            self.store.insert_occurrence(0, occurrence)?;
                        }
                        
                        info!("Stored {} symbols, {} edges, {} occurrences from Go", 
                              symbols.len(), edges.len(), occurrences.len());
                    }
                }
            }
        } else {
            debug!("scip-go not available, skipping Go semantic analysis");
        }
        
        Ok(())
    }
    
    async fn resolve_rust_project(&mut self, project_path: &Path, commit_sha: &str) -> Result<()> {
        info!("Resolving Rust symbols using rust-analyzer SCIP");
        
        // Check if rust-analyzer with SCIP support is available
        if let Ok(output) = std::process::Command::new("rust-analyzer")
            .arg("scip")
            .arg(".")
            .current_dir(project_path)
            .output() 
        {
            if output.status.success() {
                let scip_file = project_path.join("index.scip");
                if scip_file.exists() {
                    let scip_index = self.scip_mapper.parse_scip_index(scip_file.to_str().unwrap())?;
                    let (symbols, edges, occurrences) = self.scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
                    
                    for symbol in &symbols {
                        self.store.insert_symbol(0, symbol)?;
                    }
                    for edge in &edges {
                        self.store.insert_edge(0, edge)?;
                    }
                    for occurrence in &occurrences {
                        self.store.insert_occurrence(0, occurrence)?;
                    }
                    
                    info!("Stored {} symbols, {} edges, {} occurrences from Rust", 
                          symbols.len(), edges.len(), occurrences.len());
                }
            }
        } else {
            debug!("rust-analyzer SCIP not available, skipping Rust semantic analysis");
        }
        
        Ok(())
    }
    
    async fn resolve_java_project(&mut self, project_path: &Path, commit_sha: &str) -> Result<()> {
        info!("Resolving Java symbols using SCIP");
        
        // Check if scip-java is available
        if let Ok(output) = std::process::Command::new("scip-java")
            .arg("index")
            .current_dir(project_path)
            .output() 
        {
            if output.status.success() {
                let scip_file = project_path.join("index.scip");
                if scip_file.exists() {
                    let scip_index = self.scip_mapper.parse_scip_index(scip_file.to_str().unwrap())?;
                    let (symbols, edges, occurrences) = self.scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
                    
                    for symbol in &symbols {
                        self.store.insert_symbol(0, symbol)?;
                    }
                    for edge in &edges {
                        self.store.insert_edge(0, edge)?;
                    }
                    for occurrence in &occurrences {
                        self.store.insert_occurrence(0, occurrence)?;
                    }
                    
                    info!("Stored {} symbols, {} edges, {} occurrences from Java", 
                          symbols.len(), edges.len(), occurrences.len());
                }
            }
        } else {
            debug!("scip-java not available, skipping Java semantic analysis");
        }
        
        Ok(())
    }
    
    pub fn resolve_references(&mut self, unresolved_edges: Vec<EdgeIR>) -> Result<Vec<EdgeIR>> {
        let mut resolved_edges = Vec::new();
        
        for mut edge in unresolved_edges {
            if edge.resolution == Resolution::Syntactic {
                // Try to resolve using stored semantic data
                if let Some(ref dst_name) = edge.dst {
                    // Look up symbol by FQN
                    if let Ok(Some(symbol)) = self.store.find_symbol_by_fqn(dst_name) {
                        // Found a match - upgrade to semantic resolution
                        edge.dst = Some(symbol.id.clone());
                        edge.resolution = Resolution::Semantic;
                        edge.meta.insert("resolved_by".to_string(), json!("scip"));
                    }
                }
            }
            resolved_edges.push(edge);
        }
        
        Ok(resolved_edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_detect_languages() {
        let temp_dir = TempDir::new().unwrap();
        let store = GraphStore::new(temp_dir.path()).unwrap();
        let engine = ResolutionEngine::new(store);
        
        // Create language-specific files
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("go.mod"), "module test").unwrap();
        
        let languages = engine.detect_project_languages(temp_dir.path()).unwrap();
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Go));
    }
}