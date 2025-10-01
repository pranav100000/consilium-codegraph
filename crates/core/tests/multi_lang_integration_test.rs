use anyhow::Result;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_all_supported_languages() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create test files for each supported language
    fs::write(dir.path().join("main.ts"), r#"
export function greet(name: string): string {
    return `Hello, ${name}!`;
}
"#)?;
    
    fs::write(dir.path().join("utils.py"), r#"
def calculate(x: int, y: int) -> int:
    return x + y

class Helper:
    def process(self, data):
        return data
"#)?;
    
    fs::write(dir.path().join("server.go"), r#"
package main

import "fmt"

func main() {
    fmt.Println("Server starting...")
}
"#)?;
    
    fs::write(dir.path().join("lib.rs"), r#"
pub fn process(input: &str) -> String {
    input.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process() {
        assert_eq!(process("hello"), "HELLO");
    }
}
"#)?;
    
    fs::write(dir.path().join("App.java"), r#"
public class App {
    public static void main(String[] args) {
        System.out.println("Java App");
    }
}
"#)?;
    
    fs::write(dir.path().join("calculator.cpp"), r#"
#include <iostream>

class Calculator {
public:
    int add(int a, int b) {
        return a + b;
    }
};

int main() {
    Calculator calc;
    std::cout << calc.add(5, 3) << std::endl;
    return 0;
}
"#)?;
    
    fs::write(dir.path().join("utils.c"), r#"
#include <stdio.h>

int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

int main() {
    printf("5! = %d\n", factorial(5));
    return 0;
}
"#)?;
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Scan should succeed");

    // Verify actual database contents instead of stdout parsing
    use store::GraphStore;
    let store = GraphStore::new(dir.path())?;
    let file_count = store.get_file_count()?;

    assert_eq!(file_count, 7,
        "Should index all 7 files (TypeScript, Python, Go, Rust, Java, C++, C)");

    // Check database was created
    let db_path = dir.path().join(".reviewbot/graph.db");
    assert!(db_path.exists(), "Database should exist");

    Ok(())
}

#[test]
fn test_edge_detection_across_languages() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create files with imports/includes
    fs::write(dir.path().join("main.ts"), r#"
import { helper } from './helper';
export const result = helper(5);
"#)?;
    
    fs::write(dir.path().join("helper.ts"), r#"
export function helper(x: number): number {
    return x * 2;
}
"#)?;
    
    fs::write(dir.path().join("app.cpp"), r#"
#include "utils.h"
#include <iostream>

int main() {
    process();
    return 0;
}
"#)?;
    
    fs::write(dir.path().join("utils.h"), r#"
void process();
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
        .args(&["run", "-p", "reviewbot", "--", "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success());

    // Verify actual database contents instead of stdout parsing
    use store::GraphStore;
    let store = GraphStore::new(dir.path())?;
    let edge_count = store.get_edge_count()?;

    assert!(edge_count > 0,
        "Should find import edges (found {} edges)", edge_count);

    Ok(())
}

#[test]
fn test_error_recovery() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create files with syntax errors
    fs::write(dir.path().join("broken.ts"), r#"
export function broken() {
    // Missing closing brace
    if (true) {
        return "unclosed"
"#)?;
    
    fs::write(dir.path().join("broken.java"), r#"
public class Broken {
    public void method() {
        // Missing closing braces
"#)?;
    
    fs::write(dir.path().join("valid.py"), r#"
def valid_function():
    return "This is valid"
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
        .args(&["commit", "-m", "Test error recovery"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan - should not crash
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    // Should complete without crashing
    assert!(output.status.success(), "Should handle malformed files gracefully");

    // Verify actual database contents instead of stdout parsing
    use store::GraphStore;
    let store = GraphStore::new(dir.path())?;
    let file_count = store.get_file_count()?;

    assert!(file_count > 0,
        "Should still index files (found {} files)", file_count);

    Ok(())
}

#[test]
fn test_large_file_handling() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create a large file with many symbols
    let mut large_content = String::from("// Large file test\n");
    for i in 0..1000 {
        large_content.push_str(&format!(
            "export function func{}(x: number): number {{ return x + {}; }}\n",
            i, i
        ));
    }
    
    fs::write(dir.path().join("large.ts"), large_content)?;
    
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
        .args(&["commit", "-m", "Large file"])
        .current_dir(&dir)
        .output()?;
    
    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle large files");

    // Verify actual database contents instead of stdout parsing
    use store::GraphStore;
    let store = GraphStore::new(dir.path())?;
    let symbol_count = store.get_symbol_count()?;

    // Generated 1000 classes, should have at least 1000 symbols
    assert!(symbol_count >= 1000,
        "Should index at least 1000 symbols (1000 classes), got {}", symbol_count);

    Ok(())
}