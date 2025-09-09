use anyhow::Result;
use tempfile::TempDir;
use std::fs;
use std::process::Command;

#[test]
fn test_typescript_resolution() -> Result<()> {
    // Create a test TypeScript project
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create package.json
    fs::write(project_path.join("package.json"), r#"{
        "name": "test-project",
        "version": "1.0.0"
    }"#)?;
    
    // Create a TypeScript file with cross-file references
    fs::write(project_path.join("utils.ts"), r#"
export function greet(name: string): string {
    return `Hello, ${name}!`;
}

export const VERSION = "1.0.0";
"#)?;
    
    fs::write(project_path.join("main.ts"), r#"
import { greet, VERSION } from './utils';

function main() {
    const message = greet("World");
    console.log(message);
    console.log(`Version: ${VERSION}`);
}

main();
"#)?;
    
    // Run the scanner with semantic analysis
    let output = Command::new("cargo")
        .args(&["run", "--bin", "reviewbot", "--", "--repo", &project_path.to_string_lossy(), "scan", "--semantic"])
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);
    
    // Check that semantic analysis ran
    assert!(stdout.contains("semantic") || stderr.contains("Starting semantic analysis"));
    
    Ok(())
}

#[test]
fn test_multi_language_resolution() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create mixed language project files
    fs::write(project_path.join("package.json"), "{}")?;
    fs::write(project_path.join("go.mod"), "module test")?;
    fs::write(project_path.join("requirements.txt"), "pytest")?;
    
    // TypeScript file
    fs::write(project_path.join("app.ts"), r#"
export class App {
    run(): void {
        console.log("Running");
    }
}
"#)?;
    
    // Python file
    fs::write(project_path.join("test.py"), r#"
def test_app():
    assert True
"#)?;
    
    // Go file
    fs::write(project_path.join("main.go"), r#"
package main

func main() {
    println("Hello")
}
"#)?;
    
    // Run the scanner with semantic analysis
    let output = Command::new("cargo")
        .args(&["run", "--bin", "reviewbot", "--", "--repo", &project_path.to_string_lossy(), "scan", "--semantic"])
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Check that files were processed
    assert!(stdout.contains("3 files") || stdout.contains("Indexed"));
    
    Ok(())
}