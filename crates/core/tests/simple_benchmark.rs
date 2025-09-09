use anyhow::Result;
use std::fs;
use std::time::Instant;
use std::path::Path;
use tempfile::TempDir;
use store::GraphStore;
use scip_mapper::ScipMapper;

/// Simple performance benchmarks for SCIP pipeline

fn create_simple_typescript_project(temp_dir: &Path) -> Result<()> {
    // Create package.json
    let package_json = r#"{"name": "benchmark-project", "version": "1.0.0"}"#;
    fs::write(temp_dir.join("package.json"), package_json)?;
    
    // Create TypeScript files
    for i in 0..3 {
        let content = format!(r#"
export interface UserConfig{} {{
    id: number;
    name: string;
    email: string;
}}

export class UserService{} {{
    private users: UserConfig{}[] = [];
    
    addUser(user: UserConfig{}): void {{
        this.users.push(user);
    }}
    
    getUser(id: number): UserConfig{} | null {{
        return this.users.find(u => u.id === id) || null;
    }}
    
    getAllUsers(): UserConfig{}[] {{
        return [...this.users];
    }}
}}
"#, i, i, i, i, i, i);
        
        fs::write(temp_dir.join(format!("service_{}.ts", i)), content)?;
    }
    
    Ok(())
}

fn create_simple_python_project(temp_dir: &Path) -> Result<()> {
    // Create setup.py
    let setup_py = r#"from setuptools import setup
setup(name="benchmark-project", version="1.0.0")
"#;
    fs::write(temp_dir.join("setup.py"), setup_py)?;
    
    // Create Python files
    for i in 0..3 {
        let content = format!(r#"
from typing import List, Optional
from dataclasses import dataclass

@dataclass
class User{}:
    id: int
    name: str
    email: str

class UserService{}:
    def __init__(self):
        self.users: List[User{}] = []
    
    def add_user(self, user: User{}) -> None:
        self.users.append(user)
    
    def get_user(self, user_id: int) -> Optional[User{}]:
        for user in self.users:
            if user.id == user_id:
                return user
        return None
    
    def get_all_users(self) -> List[User{}]:
        return self.users.copy()
"#, i, i, i, i, i, i);
        
        fs::write(temp_dir.join(format!("service_{}.py", i)), content)?;
    }
    
    Ok(())
}

#[test]
fn bench_typescript_scip_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🚀 TypeScript SCIP Performance Benchmark");
    
    // Setup project
    let setup_start = Instant::now();
    create_simple_typescript_project(project_path)?;
    let setup_time = setup_start.elapsed();
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    // Test SCIP indexing
    let indexing_start = Instant::now();
    let scip_result = scip_mapper.run_scip_typescript(&project_path.to_string_lossy());
    let indexing_time = indexing_start.elapsed();
    
    if scip_result.is_err() {
        println!("⏩ Skipping - scip-typescript not available");
        return Ok(());
    }
    
    let scip_file = scip_result?;
    let file_size = fs::metadata(&scip_file)?.len();
    
    // Test JSON parsing
    let parsing_start = Instant::now();
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let parsing_time = parsing_start.elapsed();
    
    // Test IR conversion
    let conversion_start = Instant::now();
    let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "bench")?;
    let conversion_time = conversion_start.elapsed();
    
    // Test storage
    let store = GraphStore::new(project_path)?;
    let commit_id = store.get_or_create_commit("bench")?;
    
    let storage_start = Instant::now();
    for symbol in &symbols {
        let _ = store.insert_symbol(commit_id, symbol);
    }
    let storage_time = storage_start.elapsed();
    
    let total_time = setup_time + indexing_time + parsing_time + conversion_time + storage_time;
    
    println!("📊 Results:");
    println!("  Setup:       {:>6.0}ms", setup_time.as_millis());
    println!("  Indexing:    {:>6.0}ms", indexing_time.as_millis());
    println!("  Parsing:     {:>6.0}ms", parsing_time.as_millis()); 
    println!("  Conversion:  {:>6.0}ms", conversion_time.as_millis());
    println!("  Storage:     {:>6.0}ms", storage_time.as_millis());
    println!("  Total:       {:>6.0}ms", total_time.as_millis());
    println!("  SCIP size:   {:>6} bytes", file_size);
    println!("  Symbols:     {:>6}", symbols.len());
    println!("  Edges:       {:>6}", edges.len());
    println!("  Occurrences: {:>6}", occurrences.len());
    
    // Assertions
    assert!(indexing_time.as_secs() < 30);
    assert!(parsing_time.as_millis() < 1000);
    assert!(!symbols.is_empty());
    
    println!("✅ TypeScript benchmark completed!");
    
    Ok(())
}

#[test] 
fn bench_python_scip_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🐍 Python SCIP Performance Benchmark");
    
    // Setup project
    let setup_start = Instant::now();
    create_simple_python_project(project_path)?;
    let setup_time = setup_start.elapsed();
    
    let scip_mapper = ScipMapper::new("scip-python", "0.6.6");
    
    // Test SCIP indexing
    let indexing_start = Instant::now();
    let scip_result = scip_mapper.run_scip_python(&project_path.to_string_lossy());
    let indexing_time = indexing_start.elapsed();
    
    if scip_result.is_err() {
        println!("⏩ Skipping - scip-python not available");
        return Ok(());
    }
    
    let scip_file = scip_result?;
    let file_size = fs::metadata(&scip_file)?.len();
    
    // Test JSON parsing
    let parsing_start = Instant::now();
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let parsing_time = parsing_start.elapsed();
    
    // Test IR conversion
    let conversion_start = Instant::now();
    let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "bench")?;
    let conversion_time = conversion_start.elapsed();
    
    // Test storage
    let store = GraphStore::new(project_path)?;
    let commit_id = store.get_or_create_commit("bench")?;
    
    let storage_start = Instant::now();
    for symbol in &symbols {
        let _ = store.insert_symbol(commit_id, symbol);
    }
    let storage_time = storage_start.elapsed();
    
    let total_time = setup_time + indexing_time + parsing_time + conversion_time + storage_time;
    
    println!("📊 Results:");
    println!("  Setup:       {:>6.0}ms", setup_time.as_millis());
    println!("  Indexing:    {:>6.0}ms", indexing_time.as_millis());
    println!("  Parsing:     {:>6.0}ms", parsing_time.as_millis());
    println!("  Conversion:  {:>6.0}ms", conversion_time.as_millis());
    println!("  Storage:     {:>6.0}ms", storage_time.as_millis());
    println!("  Total:       {:>6.0}ms", total_time.as_millis());
    println!("  SCIP size:   {:>6} bytes", file_size);
    println!("  Symbols:     {:>6}", symbols.len());
    println!("  Edges:       {:>6}", edges.len());
    println!("  Occurrences: {:>6}", occurrences.len());
    
    // Assertions
    assert!(indexing_time.as_secs() < 45);
    assert!(parsing_time.as_millis() < 1000);
    assert!(!symbols.is_empty());
    
    println!("✅ Python benchmark completed!");
    
    Ok(())
}