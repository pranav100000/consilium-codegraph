use anyhow::Result;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use store::GraphStore;
use tracing::{debug, info, warn};
use scip_mapper::ScipMapper;
use crate::language_strategy::LanguageStrategyRegistry;

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
        
        // Detect languages using strategy registry and collect them
        let strategy_langs: Vec<_> = self.language_registry.detect_languages(project_path)
            .into_iter()
            .map(|s| s.language())
            .collect();
        
        if strategy_langs.is_empty() {
            info!("No supported languages detected in project");
            return Ok(());
        }
        
        info!("Detected languages: {}", strategy_langs.iter()
            .map(|lang| format!("{:?}", lang))
            .collect::<Vec<_>>()
            .join(", "));
        
        // Process each detected language
        for lang in strategy_langs {
            let lang_name = format!("{:?}", lang);
            info!("Processing {} files", lang_name);
            
            // Process based on language type
            match self.process_language(lang, project_path, commit_sha).await {
                Ok(()) => {
                    info!("Successfully processed {} semantic analysis", lang_name);
                }
                Err(e) => {
                    info!("Failed to process {} semantic analysis: {}", lang_name, e);
                    // Continue with other languages even if one fails
                }
            }
        }
        
        Ok(())
    }

    pub async fn resolve_project_incremental(&mut self, project_path: &Path, commit_sha: &str) -> Result<()> {
        info!("Starting incremental cross-file symbol resolution for project");
        
        // Detect languages and get their file lists
        let strategies = self.language_registry.detect_languages(project_path);
        
        if strategies.is_empty() {
            info!("No supported languages detected in project");
            return Ok(());
        }
        
        // Build file lists for each language
        let mut language_files: HashMap<protocol::Language, Vec<std::path::PathBuf>> = HashMap::new();
        for strategy in &strategies {
            let files = strategy.detect_files(project_path);
            if !files.is_empty() {
                language_files.insert(strategy.language(), files);
            }
        }
        
        info!("Detected {} languages with files to process", language_files.len());
        
        // Check which files have changed since last processing
        let changed_files = self.detect_changed_files(project_path, commit_sha, &language_files).await?;
        
        if changed_files.is_empty() {
            info!("No files have changed - skipping semantic processing");
            return Ok(());
        }
        
        info!("Found {} changed files requiring semantic reprocessing", changed_files.len());
        
        // Process only languages that have changed files
        for (lang, files) in changed_files {
            if !files.is_empty() {
                let lang_name = format!("{:?}", lang);
                info!("Processing {} changed {} files", files.len(), lang_name);
                
                // Clean up old semantic data for changed files
                self.cleanup_old_semantic_data(commit_sha, &files).await?;
                
                // Process the language with only changed files
                match self.process_language_files(lang, project_path, commit_sha, &files).await {
                    Ok(()) => {
                        info!("Successfully processed {} incremental semantic analysis", lang_name);
                    }
                    Err(e) => {
                        warn!("Failed to process {} incremental semantic analysis: {}", lang_name, e);
                        // Continue with other languages even if one fails
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_language(
        &mut self, 
        language: protocol::Language, 
        project_path: &Path, 
        commit_sha: &str
    ) -> Result<()> {
        use protocol::Language;
        
        // Run the language-specific indexer
        let scip_file = match language {
            Language::TypeScript | Language::JavaScript => {
                self.scip_mapper.run_scip_typescript(&project_path.to_string_lossy())?
            }
            Language::Python => {
                self.scip_mapper.run_scip_python(&project_path.to_string_lossy())?
            }
            Language::Go => {
                self.scip_mapper.run_scip_go(&project_path.to_string_lossy())?
            }
            Language::Rust => {
                self.scip_mapper.run_scip_rust(&project_path.to_string_lossy())?
            }
            Language::Java => {
                self.scip_mapper.run_scip_java(&project_path.to_string_lossy())?
            }
            Language::Cpp => {
                self.scip_mapper.run_scip_cpp(&project_path.to_string_lossy())?
            }
            _ => {
                anyhow::bail!("SCIP indexing not implemented for {:?}", language);
            }
        };
        
        // Parse SCIP index
        let scip_index = self.scip_mapper.parse_scip_index(&scip_file)?;
        
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
        
        info!("Stored {} symbols, {} edges, {} occurrences for {:?}", 
              symbols.len(), edges.len(), occurrences.len(), language);
        
        Ok(())
    }
    
    async fn detect_changed_files(
        &self,
        project_path: &Path,
        commit_sha: &str,
        language_files: &HashMap<protocol::Language, Vec<std::path::PathBuf>>
    ) -> Result<HashMap<protocol::Language, Vec<std::path::PathBuf>>> {
        let mut changed_files = HashMap::new();
        
        for (lang, files) in language_files {
            let mut lang_changed_files = Vec::new();
            
            for file_path in files {
                // Calculate current file hash
                let current_hash = if file_path.exists() {
                    self.calculate_file_hash(file_path)?
                } else {
                    // File was deleted
                    String::new()
                };
                
                // Get stored hash from database
                let relative_path = file_path.strip_prefix(project_path)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string();
                    
                let stored_hash = self.store.get_file_hash(commit_sha, &relative_path)?;
                
                // Compare hashes
                let file_changed = match &stored_hash {
                    Some(stored) => stored != &current_hash,
                    None => !current_hash.is_empty(), // New file
                };
                
                if file_changed {
                    debug!("File changed: {} (was: {:?}, now: {})", relative_path, stored_hash, current_hash);
                    lang_changed_files.push(file_path.clone());
                }
            }
            
            if !lang_changed_files.is_empty() {
                changed_files.insert(lang.clone(), lang_changed_files);
            }
        }
        
        Ok(changed_files)
    }
    
    async fn cleanup_old_semantic_data(
        &self,
        _commit_sha: &str,
        changed_files: &[std::path::PathBuf]
    ) -> Result<()> {
        info!("Cleaning up old semantic data for {} changed files", changed_files.len());
        
        // TODO: Add methods to GraphStore to delete symbols/edges/occurrences by file path
        // For now, we'll rely on the INSERT OR REPLACE behavior in the database
        
        Ok(())
    }
    
    async fn process_language_files(
        &mut self,
        language: protocol::Language,
        project_path: &Path,
        commit_sha: &str,
        files: &[std::path::PathBuf]
    ) -> Result<()> {
        info!("Processing {} files for language {:?}", files.len(), language);
        
        // For incremental processing, we still need to run the full SCIP indexer
        // because SCIP indexers work at the project level and understand cross-file dependencies
        // However, we can be smarter about what we store afterwards
        
        // Run the language-specific indexer (same as full processing)
        let scip_file = match language {
            protocol::Language::TypeScript | protocol::Language::JavaScript => {
                self.scip_mapper.run_scip_typescript(&project_path.to_string_lossy())?
            }
            protocol::Language::Python => {
                self.scip_mapper.run_scip_python(&project_path.to_string_lossy())?
            }
            protocol::Language::Go => {
                self.scip_mapper.run_scip_go(&project_path.to_string_lossy())?
            }
            protocol::Language::Rust => {
                self.scip_mapper.run_scip_rust(&project_path.to_string_lossy())?
            }
            protocol::Language::Java => {
                self.scip_mapper.run_scip_java(&project_path.to_string_lossy())?
            }
            protocol::Language::Cpp => {
                self.scip_mapper.run_scip_cpp(&project_path.to_string_lossy())?
            }
            _ => {
                anyhow::bail!("SCIP indexing not implemented for {:?}", language);
            }
        };
        
        // Parse SCIP index
        let scip_index = self.scip_mapper.parse_scip_index(&scip_file)?;
        
        // Convert to IR
        let (symbols, edges, occurrences) = self.scip_mapper.map_scip_to_ir(&scip_index, commit_sha)?;
        
        // Filter to only symbols/edges/occurrences related to changed files
        let changed_file_paths: Vec<String> = files.iter()
            .filter_map(|p| p.strip_prefix(project_path).ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        
        let filtered_symbols: Vec<_> = symbols.into_iter()
            .filter(|s| changed_file_paths.iter().any(|f| s.file_path.contains(f)))
            .collect();
            
        let filtered_occurrences: Vec<_> = occurrences.into_iter()
            .filter(|o| changed_file_paths.iter().any(|f| o.file_path.contains(f)))
            .collect();
        
        // Keep all edges since they may connect changed and unchanged files
        let filtered_edges = edges;
        
        // Store semantic data
        let commit_id = self.store.get_or_create_commit(commit_sha)?;
        
        // Update file hashes for changed files
        for file_path in files {
            if file_path.exists() {
                let content_hash = self.calculate_file_hash(file_path)?;
                let relative_path = file_path.strip_prefix(project_path)
                    .unwrap_or(file_path)
                    .to_string_lossy();
                let file_size = fs::metadata(file_path)?.len() as usize;
                
                self.store.insert_file(commit_id, &relative_path, &content_hash, file_size)?;
            }
        }
        
        // Insert symbols
        for symbol in &filtered_symbols {
            if let Err(e) = self.store.insert_symbol(commit_id, symbol) {
                debug!("Failed to insert symbol {}: {}", symbol.id, e);
            }
        }
        
        // Insert edges
        for edge in &filtered_edges {
            if let Err(e) = self.store.insert_edge(commit_id, edge) {
                debug!("Failed to insert edge: {}", e);
            }
        }
        
        // Insert occurrences
        for occurrence in &filtered_occurrences {
            if let Err(e) = self.store.insert_occurrence(commit_id, occurrence) {
                debug!("Failed to insert occurrence: {}", e);
            }
        }
        
        info!("Stored {} symbols, {} edges, {} occurrences for {:?} (incremental)", 
              filtered_symbols.len(), filtered_edges.len(), filtered_occurrences.len(), language);
        
        Ok(())
    }
    
    fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        let content = fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }
}