use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_test_project() -> Result<TempDir> {
    let dir = TempDir::new()?;
    
    // Create a simple TypeScript file
    let ts_file = r#"
import { helper } from './helper';

export class Calculator {
    private value: number = 0;
    
    add(n: number): void {
        this.value += n;
    }
    
    subtract(n: number): void {
        this.value -= n;
    }
}

export function createCalculator(): Calculator {
    return new Calculator();
}

export const PI = 3.14159;
"#;
    
    let helper_file = r#"
export function helper(x: number): number {
    return x * 2;
}
"#;
    
    fs::write(dir.path().join("calculator.ts"), ts_file)?;
    fs::write(dir.path().join("helper.ts"), helper_file)?;
    
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
fn test_end_to_end_scan() -> Result<()> {
    let test_dir = create_test_project()?;
    
    // Run scan
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Scan should succeed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Indexed 2 files"), "Should index 2 files");
    assert!(stdout.contains("symbols"), "Should find symbols");
    assert!(stdout.contains("edges"), "Should find edges");
    
    // Verify database exists
    let db_path = test_dir.path().join(".reviewbot/graph.db");
    assert!(db_path.exists(), "Database should be created");
    
    Ok(())
}

#[test]
fn test_idempotent_scan() -> Result<()> {
    let test_dir = create_test_project()?;
    
    // First scan
    let output1 = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    
    // Extract just the numbers from the indexed line
    let extract_numbers = |s: &str| -> String {
        // Find the part after "Indexed"
        if let Some(idx) = s.find("Indexed ") {
            let rest = &s[idx + 8..];
            rest.chars()
                .filter(|c| c.is_numeric() || c.is_whitespace() || *c == ',' || c.is_alphabetic())
                .collect()
        } else {
            s.to_string()
        }
    };
    
    let result1 = extract_numbers(&stdout1);
    
    // Second scan
    let output2 = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    
    // Second scan should detect no changes
    assert!(stdout2.contains("Repository unchanged since last scan") || 
            stdout2.contains("0 files") || 
            stdout2.contains("No changes"),
            "Second scan should detect no changes, got: {}", stdout2);
    
    Ok(())
}