use anyhow::Result;
use protocol::Language;
use scip_mapper::ScipMapper;
use std::path::{Path, PathBuf};

/// Strategy pattern for language-specific SCIP indexing
pub trait LanguageStrategy {
    /// Detect files for this language in the given path
    fn detect_files(&self, path: &Path) -> Vec<PathBuf>;
    
    /// Run the SCIP indexer for this language
    fn run_indexer(&self, scip_mapper: &ScipMapper, project_path: &Path) -> Result<PathBuf>;
    
    /// Get the language this strategy handles
    fn language(&self) -> Language;
    
    /// Get the human-readable name for this language
    fn name(&self) -> &'static str;
    
    /// Check if this strategy can handle the given project
    fn can_handle(&self, path: &Path) -> bool {
        !self.detect_files(path).is_empty()
    }
}

/// TypeScript/JavaScript strategy
pub struct TypeScriptStrategy;

impl LanguageStrategy for TypeScriptStrategy {
    fn detect_files(&self, path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
                        files.push(file_path);
                    }
                }
            }
        }
        
        // Also check for package.json
        let package_json = path.join("package.json");
        if package_json.exists() {
            files.push(package_json);
        }
        
        files
    }
    
    fn run_indexer(&self, scip_mapper: &ScipMapper, project_path: &Path) -> Result<PathBuf> {
        let scip_file = scip_mapper.run_scip_typescript(&project_path.to_string_lossy())?;
        Ok(PathBuf::from(scip_file))
    }
    
    fn language(&self) -> Language {
        Language::TypeScript
    }
    
    fn name(&self) -> &'static str {
        "TypeScript/JavaScript"
    }
}

/// Python strategy
pub struct PythonStrategy;

impl LanguageStrategy for PythonStrategy {
    fn detect_files(&self, path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "py" || ext == "pyi" {
                        files.push(file_path);
                    }
                }
            }
        }
        
        // Also check for Python project markers
        for marker in &["setup.py", "pyproject.toml", "requirements.txt", "__init__.py"] {
            let marker_file = path.join(marker);
            if marker_file.exists() {
                files.push(marker_file);
            }
        }
        
        files
    }
    
    fn run_indexer(&self, scip_mapper: &ScipMapper, project_path: &Path) -> Result<PathBuf> {
        let scip_file = scip_mapper.run_scip_python(&project_path.to_string_lossy())?;
        Ok(PathBuf::from(scip_file))
    }
    
    fn language(&self) -> Language {
        Language::Python
    }
    
    fn name(&self) -> &'static str {
        "Python"
    }
}

/// Go strategy (placeholder for future implementation)
pub struct GoStrategy;

impl LanguageStrategy for GoStrategy {
    fn detect_files(&self, path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "go" {
                        files.push(file_path);
                    }
                }
            }
        }
        
        // Check for go.mod
        let go_mod = path.join("go.mod");
        if go_mod.exists() {
            files.push(go_mod);
        }
        
        files
    }
    
    fn run_indexer(&self, _scip_mapper: &ScipMapper, _project_path: &Path) -> Result<PathBuf> {
        anyhow::bail!("Go SCIP indexing not implemented yet")
    }
    
    fn language(&self) -> Language {
        Language::Go
    }
    
    fn name(&self) -> &'static str {
        "Go"
    }
}

/// Rust strategy for detecting and processing Rust projects
pub struct RustStrategy;

impl LanguageStrategy for RustStrategy {
    fn detect_files(&self, path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        // Recursively find .rs files
        self.find_rust_files(path, &mut files);
        
        // Check for Cargo.toml
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            files.push(cargo_toml);
        }
        
        files
    }
    
    fn run_indexer(&self, scip_mapper: &ScipMapper, project_path: &Path) -> Result<PathBuf> {
        let scip_file = scip_mapper.run_scip_rust(&project_path.to_string_lossy())?;
        Ok(PathBuf::from(scip_file))
    }
    
    fn language(&self) -> Language {
        Language::Rust
    }
    
    fn name(&self) -> &'static str {
        "Rust"
    }
    
    fn can_handle(&self, path: &Path) -> bool {
        path.join("Cargo.toml").exists() || 
        self.has_rust_files(path)
    }
}

impl RustStrategy {
    fn find_rust_files(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                    self.find_rust_files(&path, files);
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    fn has_rust_files(&self, path: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "rs" {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Java strategy for detecting and processing Java projects
pub struct JavaStrategy;

impl LanguageStrategy for JavaStrategy {
    fn detect_files(&self, path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        // Recursively find .java files
        self.find_java_files(path, &mut files);
        
        // Check for build files
        for build_file in &["pom.xml", "build.gradle", "build.gradle.kts"] {
            let build_path = path.join(build_file);
            if build_path.exists() {
                files.push(build_path);
            }
        }
        
        files
    }
    
    fn run_indexer(&self, scip_mapper: &ScipMapper, project_path: &Path) -> Result<PathBuf> {
        let scip_file = scip_mapper.run_scip_java(&project_path.to_string_lossy())?;
        Ok(PathBuf::from(scip_file))
    }
    
    fn language(&self) -> Language {
        Language::Java
    }
    
    fn name(&self) -> &'static str {
        "Java"
    }
    
    fn can_handle(&self, path: &Path) -> bool {
        path.join("pom.xml").exists() || 
        path.join("build.gradle").exists() ||
        path.join("build.gradle.kts").exists() ||
        self.has_java_files(path)
    }
}

impl JavaStrategy {
    fn find_java_files(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                    self.find_java_files(&path, files);
                } else if let Some(ext) = path.extension() {
                    if ext == "java" {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    fn has_java_files(&self, path: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "java" {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// C/C++ strategy for detecting and processing C/C++ projects
pub struct CppStrategy;

impl LanguageStrategy for CppStrategy {
    fn detect_files(&self, path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        // Recursively find C/C++ files
        self.find_cpp_files(path, &mut files);
        
        // Check for build files
        for build_file in &["CMakeLists.txt", "Makefile", "makefile", "configure.ac", "meson.build"] {
            let build_path = path.join(build_file);
            if build_path.exists() {
                files.push(build_path);
            }
        }
        
        files
    }
    
    fn run_indexer(&self, scip_mapper: &ScipMapper, project_path: &Path) -> Result<PathBuf> {
        let scip_file = scip_mapper.run_scip_cpp(&project_path.to_string_lossy())?;
        Ok(PathBuf::from(scip_file))
    }
    
    fn language(&self) -> Language {
        Language::Cpp
    }
    
    fn name(&self) -> &'static str {
        "C++"
    }
    
    fn can_handle(&self, path: &Path) -> bool {
        path.join("CMakeLists.txt").exists() || 
        path.join("Makefile").exists() ||
        path.join("makefile").exists() ||
        self.has_cpp_files(path)
    }
}

impl CppStrategy {
    fn find_cpp_files(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                    self.find_cpp_files(&path, files);
                } else if let Some(ext) = path.extension() {
                    if matches!(ext.to_string_lossy().as_ref(), "cpp" | "cxx" | "cc" | "c" | "hpp" | "hxx" | "h") {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    fn has_cpp_files(&self, path: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if matches!(ext.to_string_lossy().as_ref(), "cpp" | "cxx" | "cc" | "c" | "hpp" | "hxx" | "h") {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Registry of all available language strategies
pub struct LanguageStrategyRegistry {
    strategies: Vec<Box<dyn LanguageStrategy>>,
}

impl LanguageStrategyRegistry {
    pub fn new() -> Self {
        let strategies: Vec<Box<dyn LanguageStrategy>> = vec![
            Box::new(TypeScriptStrategy),
            Box::new(PythonStrategy),
            Box::new(GoStrategy),
            Box::new(RustStrategy),
            Box::new(JavaStrategy),
            Box::new(CppStrategy),
        ];
        
        Self { strategies }
    }
    
    /// Detect all languages present in the given project path
    pub fn detect_languages(&self, path: &Path) -> Vec<&dyn LanguageStrategy> {
        self.strategies
            .iter()
            .filter_map(|strategy| {
                if strategy.can_handle(path) {
                    Some(strategy.as_ref())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Get strategy for a specific language
    pub fn get_strategy(&self, language: Language) -> Option<&dyn LanguageStrategy> {
        self.strategies
            .iter()
            .find(|strategy| strategy.language() == language)
            .map(|s| s.as_ref())
    }
    
    /// List all available strategies
    pub fn list_strategies(&self) -> Vec<&dyn LanguageStrategy> {
        self.strategies.iter().map(|s| s.as_ref()).collect()
    }
}

impl Default for LanguageStrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}