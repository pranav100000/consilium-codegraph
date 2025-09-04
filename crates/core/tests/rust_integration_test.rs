use anyhow::Result;
use std::fs;
use tempfile::TempDir;

fn create_rust_test_project() -> Result<TempDir> {
    let dir = TempDir::new()?;
    
    // Create a simple Rust file with various constructs
    let main_rs = r#"
use std::collections::HashMap;

pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Calculator { value: 0 }
    }
    
    pub fn add(&mut self, n: i32) {
        self.value += n;
    }
}

pub trait Compute {
    fn compute(&self) -> i32;
}

impl Compute for Calculator {
    fn compute(&self) -> i32 {
        self.value
    }
}

pub enum Operation {
    Add(i32),
    Subtract(i32),
    Multiply(i32),
}

pub fn process_operation(op: Operation, value: i32) -> i32 {
    match op {
        Operation::Add(n) => value + n,
        Operation::Subtract(n) => value - n,
        Operation::Multiply(n) => value * n,
    }
}

const PI: f64 = 3.14159;
type Result<T> = std::result::Result<T, String>;

mod utils {
    pub fn double(x: i32) -> i32 {
        x * 2
    }
}
"#;
    
    let lib_rs = r#"
pub mod calculator;

pub use calculator::Calculator;
"#;
    
    fs::write(dir.path().join("main.rs"), main_rs)?;
    fs::write(dir.path().join("lib.rs"), lib_rs)?;
    
    // Initialize git repo
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(dir.path())
        .output()?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(dir.path())
        .output()?;
        
    std::process::Command::new("git")
        .args(&["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()?;
    
    Ok(dir)
}

#[test]
fn test_rust_parsing() -> Result<()> {
    let test_dir = create_rust_test_project()?;
    
    // Run scan
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Scan should succeed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Indexed 2 files"), "Should index 2 Rust files");
    assert!(stdout.contains("symbols"), "Should find symbols");
    assert!(stdout.contains("edges"), "Should find edges");
    
    // Verify database exists
    let db_path = test_dir.path().join(".reviewbot/graph.db");
    assert!(db_path.exists(), "Database should be created");
    
    // Test search for Rust symbols
    let search_output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "search", "Calculator"])
        .output()?;
    
    assert!(search_output.status.success(), "Search should succeed");
    
    let search_stdout = String::from_utf8_lossy(&search_output.stdout);
    assert!(search_stdout.contains("Calculator"), "Should find Calculator struct");
    assert!(search_stdout.contains("Struct"), "Should identify as struct");
    
    Ok(())
}

#[test]
fn test_rust_symbols_extraction() -> Result<()> {
    let test_dir = create_rust_test_project()?;
    
    // Run scan
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Scan should succeed");
    
    // Search for various symbol types
    
    // Search for trait
    let trait_search = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "search", "Compute"])
        .output()?;
    
    let trait_stdout = String::from_utf8_lossy(&trait_search.stdout);
    assert!(trait_stdout.contains("Compute"), "Should find Compute trait");
    assert!(trait_stdout.contains("Trait"), "Should identify as trait");
    
    // Search for enum
    let enum_search = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "search", "Operation"])
        .output()?;
    
    let enum_stdout = String::from_utf8_lossy(&enum_search.stdout);
    assert!(enum_stdout.contains("Operation"), "Should find Operation enum");
    assert!(enum_stdout.contains("Enum"), "Should identify as enum");
    
    // Search for function
    let func_search = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "search", "process_operation"])
        .output()?;
    
    let func_stdout = String::from_utf8_lossy(&func_search.stdout);
    assert!(func_stdout.contains("process_operation"), "Should find process_operation function");
    assert!(func_stdout.contains("Function"), "Should identify as function");
    
    // Search for module
    let mod_search = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "search", "utils"])
        .output()?;
    
    let mod_stdout = String::from_utf8_lossy(&mod_search.stdout);
    assert!(mod_stdout.contains("utils"), "Should find utils module");
    assert!(mod_stdout.contains("Module"), "Should identify as module");
    
    Ok(())
}

#[test]
fn test_rust_mixed_languages() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create files in different languages
    let rust_file = r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    x + y
}
"#;
    
    let python_file = r#"
def calculate(x, y):
    return x + y
"#;
    
    let go_file = r#"
package main

func calculate(x, y int) int {
    return x + y
}
"#;
    
    let ts_file = r#"
export function calculate(x: number, y: number): number {
    return x + y;
}
"#;
    
    fs::write(dir.path().join("calc.rs"), rust_file)?;
    fs::write(dir.path().join("calc.py"), python_file)?;
    fs::write(dir.path().join("calc.go"), go_file)?;
    fs::write(dir.path().join("calc.ts"), ts_file)?;
    
    // Initialize git repo
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(dir.path())
        .output()?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(dir.path())
        .output()?;
        
    std::process::Command::new("git")
        .args(&["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()?;
    
    // Run scan
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Scan should succeed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Indexed 4 files"), "Should index 4 files from different languages");
    
    // Search for calculate function across all languages
    let search_output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", dir.path().to_str().unwrap(), "search", "calculate"])
        .output()?;
    
    let search_stdout = String::from_utf8_lossy(&search_output.stdout);
    
    // Should find calculate in all 4 files
    assert!(search_stdout.contains("calc.rs"), "Should find Rust calculate");
    assert!(search_stdout.contains("calc.py"), "Should find Python calculate");
    assert!(search_stdout.contains("calc.go"), "Should find Go calculate");
    assert!(search_stdout.contains("calc.ts"), "Should find TypeScript calculate");
    
    Ok(())
}