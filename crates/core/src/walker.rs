use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, info};

pub struct FileWalker {
    root: PathBuf,
    extensions: HashSet<String>,
}

impl FileWalker {
    pub fn new(root: PathBuf) -> Self {
        let mut extensions = HashSet::new();
        // TypeScript/JavaScript
        extensions.insert("ts".to_string());
        extensions.insert("tsx".to_string());
        extensions.insert("js".to_string());
        extensions.insert("jsx".to_string());
        extensions.insert("mjs".to_string());
        // Python
        extensions.insert("py".to_string());
        extensions.insert("pyi".to_string());
        // Go
        extensions.insert("go".to_string());
        
        Self { root, extensions }
    }
    
    pub fn walk(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        let walker = WalkBuilder::new(&self.root)
            .hidden(false)  // Include hidden files except .git
            .git_ignore(true)  // Respect .gitignore
            .git_global(true)  // Respect global gitignore
            .git_exclude(true)  // Respect .git/info/exclude
            .require_git(false)  // Work even if not a git repo
            .build();
        
        for entry in walker {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if self.extensions.contains(ext.to_str().unwrap_or("")) {
                        // Skip node_modules and other vendor directories
                        let path_str = path.to_string_lossy();
                        if path_str.contains("node_modules") || 
                           path_str.contains("vendor") ||
                           path_str.contains(".next") ||
                           path_str.contains("dist") ||
                           path_str.contains("build") {
                            continue;
                        }
                        
                        debug!("Found file: {:?}", path);
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
        
        info!("Found {} files to index", files.len());
        Ok(files)
    }
    
    pub fn compute_file_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_walker_finds_source_files() -> Result<()> {
        let dir = TempDir::new()?;
        
        // Create test files
        fs::write(dir.path().join("main.ts"), "console.log('test')")?;
        fs::write(dir.path().join("utils.js"), "export const x = 1")?;
        fs::write(dir.path().join("test.py"), "def main(): pass")?;
        fs::write(dir.path().join("app.go"), "package main")?;
        fs::write(dir.path().join("readme.md"), "# README")?; // Should be ignored
        
        let walker = FileWalker::new(dir.path().to_path_buf());
        let files = walker.walk()?;
        
        assert_eq!(files.len(), 4, "Should find 4 source files");
        
        let file_names: Vec<String> = files.iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();
        
        assert!(file_names.contains(&"main.ts".to_string()));
        assert!(file_names.contains(&"utils.js".to_string()));
        assert!(file_names.contains(&"test.py".to_string()));
        assert!(file_names.contains(&"app.go".to_string()));
        assert!(!file_names.contains(&"readme.md".to_string()));
        
        Ok(())
    }
    
    #[test]
    fn test_walker_respects_gitignore() -> Result<()> {
        let dir = TempDir::new()?;
        
        // Create gitignore
        fs::write(dir.path().join(".gitignore"), "ignored.ts\n*.log")?;
        
        // Create files
        fs::write(dir.path().join("main.ts"), "console.log('test')")?;
        fs::write(dir.path().join("ignored.ts"), "// ignored")?;
        fs::write(dir.path().join("debug.log"), "log content")?;
        
        let walker = FileWalker::new(dir.path().to_path_buf());
        let files = walker.walk()?;
        
        assert_eq!(files.len(), 1, "Should only find main.ts");
        assert!(files[0].ends_with("main.ts"));
        
        Ok(())
    }
    
    #[test]
    fn test_walker_skips_vendor_directories() -> Result<()> {
        let dir = TempDir::new()?;
        
        // Create directories with files
        fs::create_dir(dir.path().join("src"))?;
        fs::create_dir_all(dir.path().join("node_modules/lib"))?;
        fs::create_dir(dir.path().join("vendor"))?;
        fs::create_dir(dir.path().join("dist"))?;
        
        fs::write(dir.path().join("src/main.ts"), "console.log('test')")?;
        fs::write(dir.path().join("node_modules/lib/index.js"), "// node module")?;
        fs::write(dir.path().join("vendor/lib.js"), "// vendor")?;
        fs::write(dir.path().join("dist/bundle.js"), "// dist")?;
        
        let walker = FileWalker::new(dir.path().to_path_buf());
        let files = walker.walk()?;
        
        assert_eq!(files.len(), 1, "Should only find src/main.ts");
        assert!(files[0].ends_with("main.ts"));
        
        Ok(())
    }
    
    #[test]
    fn test_file_hash_computation() {
        let content1 = "hello world";
        let content2 = "hello world";
        let content3 = "different content";
        
        let hash1 = FileWalker::compute_file_hash(content1);
        let hash2 = FileWalker::compute_file_hash(content2);
        let hash3 = FileWalker::compute_file_hash(content3);
        
        assert_eq!(hash1, hash2, "Same content should produce same hash");
        assert_ne!(hash1, hash3, "Different content should produce different hash");
    }
}