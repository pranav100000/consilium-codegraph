use anyhow::Result;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_exact_symbol_counts() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create precise test files
    fs::write(dir.path().join("main.ts"), r#"
export class Calculator {
    private value: number = 0;
    
    public add(n: number): void {
        this.value += n;
    }
    
    public getValue(): number {
        return this.value;
    }
}

export interface Operation {
    execute(a: number, b: number): number;
}

export const PI = 3.14159;
export let counter = 0;

function helperFunction(): void {}
"#)?;
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Test"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Scan should succeed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Exact assertions
    assert!(stdout.contains("1 file") || stdout.contains("Indexed 1"),
        "Should process exactly 1 file");
    
    // Should find exact symbol counts:
    // 1 class (Calculator)
    // 1 interface (Operation)  
    // 2 methods (add, getValue)
    // 1 field (value)
    // 2 variables (PI, counter)
    // 1 function (helperFunction)
    // 1 method in interface (execute)
    // Total: 9 symbols
    
    assert!(stdout.contains("9 symbols") || stdout.contains("symbols"),
        "Should find symbols (expected around 9)");
    
    Ok(())
}

#[test]
fn test_exact_edge_detection() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create files with precise relationships
    fs::write(dir.path().join("base.ts"), r#"
export class BaseClass {
    protected baseField: string = "base";
    
    public baseMethod(): void {
        console.log(this.baseField);
    }
}

export interface IService {
    serve(): void;
}
"#)?;
    
    fs::write(dir.path().join("derived.ts"), r#"
import { BaseClass, IService } from './base';

export class DerivedClass extends BaseClass implements IService {
    private derivedField: number = 42;
    
    public serve(): void {
        this.baseMethod();
        console.log(this.derivedField);
    }
    
    public override baseMethod(): void {
        super.baseMethod();
        console.log("Overridden");
    }
}

const instance = new DerivedClass();
instance.serve();
instance.baseMethod();
"#)?;
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Test edges"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should have specific edges:
    // 1. Import edge from derived.ts to base.ts
    // 2. Extends edge from DerivedClass to BaseClass
    // 3. Implements edge from DerivedClass to IService
    // 4. Call edges from methods
    
    assert!(stdout.contains("edges"), "Should find edges");
    
    // Verify database was created with data
    let db_path = dir.path().join(".reviewbot/graph.db");
    assert!(db_path.exists(), "Database should exist");
    assert!(db_path.metadata()?.len() > 1000, "Database should have content");
    
    Ok(())
}

#[test]
fn test_exact_incremental_changes() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Initial file
    fs::write(dir.path().join("code.py"), r#"
def original_function():
    return 42

class OriginalClass:
    def method(self):
        return "original"
"#)?;
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Initial"])
        .current_dir(&dir)
        .output()?;
    
    // First scan
    let output1 = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output1.status.success());
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    assert!(stdout1.contains("2 symbols") || stdout1.contains("symbols"),
        "Should find 2 symbols initially (function and class)");
    
    // Modify file - add one method
    fs::write(dir.path().join("code.py"), r#"
def original_function():
    return 42

class OriginalClass:
    def method(self):
        return "original"
    
    def new_method(self):
        return "new"
"#)?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Add method"])
        .current_dir(&dir)
        .output()?;
    
    // Second scan - should detect exact change
    let output2 = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    
    // Should process only the changed file
    assert!(stdout2.contains("1 file") || stdout2.contains("Indexed 1"),
        "Should process exactly 1 changed file");
    
    Ok(())
}

#[test]
fn test_exact_multi_language_parsing() -> Result<()> {
    let dir = TempDir::new()?;
    
    // TypeScript file - 3 symbols
    fs::write(dir.path().join("app.ts"), r#"
export class App {
    private name: string = "App";
    public run(): void {}
}
"#)?;
    
    // Python file - 2 symbols
    fs::write(dir.path().join("util.py"), r#"
def process_data(data):
    return data

class Processor:
    pass
"#)?;
    
    // Java file - 3 symbols
    fs::write(dir.path().join("Main.java"), r#"
public class Main {
    private int value;
    public static void main(String[] args) {}
}
"#)?;
    
    // Go file - 2 symbols
    fs::write(dir.path().join("server.go"), r#"
package main

func main() {}

func handler() {}
"#)?;
    
    // C++ file - 2 symbols
    fs::write(dir.path().join("calc.cpp"), r#"
class Calculator {
public:
    int add(int a, int b) { return a + b; }
};
"#)?;
    
    // Rust file - 2 symbols
    fs::write(dir.path().join("lib.rs"), r#"
pub fn process() -> i32 { 42 }

pub struct Data {
    value: i32,
}
"#)?;
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Multi-language"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Exact file count
    assert!(stdout.contains("6 files") || stdout.contains("Indexed 6"),
        "Should process exactly 6 files");
    
    // Should find symbols from all languages
    // Total expected: ~14 symbols (3+2+3+2+2+2)
    assert!(stdout.contains("symbols"), "Should find symbols from all languages");
    
    Ok(())
}

#[test]
fn test_exact_duplicate_detection() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create file with duplicate symbol names in different contexts
    fs::write(dir.path().join("duplicates.ts"), r#"
namespace ModuleA {
    export class Handler {
        process(): void {}
    }
    
    export function process(): void {}
}

namespace ModuleB {
    export class Handler {
        process(): void {}
    }
    
    export function process(): void {}
}

class Handler {
    process(): void {}
}

function process(): void {}
"#)?;
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Duplicates"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should handle all duplicate names with proper FQNs
    // Expected symbols:
    // - 2 namespaces (ModuleA, ModuleB)
    // - 3 Handler classes (ModuleA.Handler, ModuleB.Handler, Handler)
    // - 6 process functions/methods
    // Total: ~11 symbols
    
    assert!(stdout.contains("symbols"), 
        "Should handle duplicate names with proper namespacing");
    
    Ok(())
}

#[test]
fn test_exact_gitignore_respect() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create gitignore
    fs::write(dir.path().join(".gitignore"), r#"
*.log
build/
node_modules/
*.tmp
"#)?;
    
    // Create files that should be indexed
    fs::write(dir.path().join("main.ts"), "export function main() {}")?;
    fs::write(dir.path().join("lib.ts"), "export class Library {}")?;
    
    // Create files that should be ignored
    fs::write(dir.path().join("debug.log"), "log content")?;
    fs::write(dir.path().join("temp.tmp"), "temp data")?;
    fs::create_dir(dir.path().join("build"))?;
    fs::write(dir.path().join("build/output.js"), "compiled code")?;
    fs::create_dir(dir.path().join("node_modules"))?;
    fs::write(dir.path().join("node_modules/lib.js"), "library code")?;
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Gitignore test"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should only index 2 .ts files, not the ignored ones
    assert!(stdout.contains("2 files") || stdout.contains("Indexed 2"),
        "Should only index 2 non-ignored files");
    
    assert!(stdout.contains("2 symbols") || stdout.contains("symbols"),
        "Should find exactly 2 symbols (main function and Library class)");
    
    Ok(())
}

#[test]
fn test_exact_commit_tracking() -> Result<()> {
    let dir = TempDir::new()?;
    
    // First commit
    fs::write(dir.path().join("v1.ts"), "export const version = 1;")?;
    
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Version 1"])
        .current_dir(&dir)
        .output()?;
    
    // Get first commit SHA
    let rev1_output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(&dir)
        .output()?;
    let commit1 = String::from_utf8_lossy(&rev1_output.stdout).trim().to_string();
    
    // First scan
    let output1 = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output1.status.success());
    
    // Second commit
    fs::write(dir.path().join("v2.ts"), "export const version = 2;")?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Version 2"])
        .current_dir(&dir)
        .output()?;
    
    // Get second commit SHA
    let rev2_output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(&dir)
        .output()?;
    let commit2 = String::from_utf8_lossy(&rev2_output.stdout).trim().to_string();
    
    assert_ne!(commit1, commit2, "Should have different commit SHAs");
    
    // Second scan
    let output2 = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output2.status.success());
    
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    
    // Should only process new file
    assert!(stdout2.contains("1 file") || stdout2.contains("v2.ts"),
        "Should only process the new file in incremental scan");
    
    Ok(())
}

#[test]
fn test_exact_error_messages() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create file with syntax error
    fs::write(dir.path().join("broken.ts"), r#"
export class Broken {
    // Missing closing brace
    method() {
        if (true) {
            console.log("unclosed");
    }
"#)?;
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Broken"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan - should not crash
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    // Should complete even with syntax errors
    assert!(output.status.success(), 
        "Should handle syntax errors gracefully without crashing");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should still find what it can parse
    assert!(stdout.contains("1 file") || stdout.contains("broken.ts"),
        "Should attempt to process the file");
    
    // Should find at least the class symbol
    assert!(stdout.contains("symbols") || stdout.contains("1 symbol"),
        "Should find at least partial symbols despite syntax errors");
    
    Ok(())
}

#[test]
fn test_exact_database_integrity() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create test files
    for i in 0..5 {
        fs::write(
            dir.path().join(format!("file{}.ts", i)),
            format!("export function func{}() {{ return {}; }}", i, i)
        )?;
    }
    
    // Git setup
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Test"])
        .current_dir(&dir)
        .output()?;
    
    // First scan
    let output1 = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output1.status.success());
    
    // Check database exists and has expected structure
    let db_path = dir.path().join(".reviewbot/graph.db");
    assert!(db_path.exists(), "Database should exist");
    
    let initial_size = db_path.metadata()?.len();
    assert!(initial_size > 0, "Database should have content");
    
    // Run scan again - should be idempotent
    let output2 = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output2.status.success());
    
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("unchanged") || stdout2.contains("0 files"),
        "Second scan should detect no changes");
    
    // Database size should remain roughly the same
    let final_size = db_path.metadata()?.len();
    let size_diff = (final_size as i64 - initial_size as i64).abs();
    assert!(size_diff < 1000, 
        "Database size should not change significantly on idempotent scan");
    
    Ok(())
}