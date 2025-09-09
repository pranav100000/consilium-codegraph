use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use store::GraphStore;

fn create_test_typescript_project(temp_dir: &Path) -> Result<()> {
    let package_json = r#"{"name": "incremental-test", "version": "1.0.0"}"#;
    fs::write(temp_dir.join("package.json"), package_json)?;
    
    // Create initial TypeScript file
    let initial_content = r#"
export interface User {
    id: number;
    name: string;
}

export class UserService {
    private users: User[] = [];
    
    addUser(user: User): void {
        this.users.push(user);
    }
    
    getUser(id: number): User | null {
        return this.users.find(u => u.id === id) || null;
    }
}
"#;
    fs::write(temp_dir.join("user.ts"), initial_content)?;
    
    Ok(())
}

#[test]
fn test_incremental_cli_flag() -> Result<()> {
    // This test verifies that the CLI accepts the --incremental flag
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create test project
    create_test_typescript_project(project_path)?;
    
    // Test that the CLI accepts the incremental flag (dry run)
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write",
            "--semantic", 
            "--incremental"
        ])
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should not have argument parsing errors
    assert!(!stderr.contains("error: unexpected argument"), 
        "CLI should accept --incremental flag. stderr: {}", stderr);
    
    println!("✅ CLI accepts --incremental flag");
    println!("stdout: {}", stdout);
    
    Ok(())
}

#[test]
fn test_incremental_semantic_dry_run() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create test project with multiple files
    create_test_typescript_project(project_path)?;
    
    // Add another TypeScript file
    let service_content = r#"
import { User } from './user';

export class DatabaseService {
    private connection: string = "localhost:5432";
    
    saveUser(user: User): Promise<void> {
        return Promise.resolve();
    }
    
    loadUsers(): Promise<User[]> {
        return Promise.resolve([]);
    }
}
"#;
    fs::write(project_path.join("database.ts"), service_content)?;
    
    // Run full semantic scan first (dry run)
    let output1 = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan",
            "--no-write",
            "--semantic"
        ])
        .output()?;
    
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    println!("📊 Full semantic scan output: {}", stdout1);
    
    // Run incremental semantic scan (dry run)
    let output2 = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan",
            "--no-write",
            "--semantic",
            "--incremental"
        ])
        .output()?;
    
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    println!("📊 Incremental semantic scan output: {}", stdout2);
    
    // Both should complete without errors
    assert!(output1.status.success(), "Full scan should succeed");
    assert!(output2.status.success(), "Incremental scan should succeed");
    
    println!("✅ Both full and incremental scans completed successfully");
    
    Ok(())
}

#[test]
fn test_incremental_hash_calculation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create store
    let store = GraphStore::new(project_path)?;
    
    // Create test file
    let test_content = "console.log('Hello, World!');";
    let test_file = project_path.join("test.js");
    fs::write(&test_file, test_content)?;
    
    // Store initial file hash
    let commit_id = store.get_or_create_commit("test_commit")?;
    let initial_hash = "abcd1234"; // Mock hash
    store.insert_file(commit_id, "test.js", initial_hash, test_content.len())?;
    
    // Verify we can retrieve the hash
    let retrieved_hash = store.get_file_hash("test_commit", "test.js")?;
    assert_eq!(retrieved_hash, Some(initial_hash.to_string()));
    
    // Test file that doesn't exist
    let missing_hash = store.get_file_hash("test_commit", "missing.js")?;
    assert_eq!(missing_hash, None);
    
    println!("✅ File hash storage and retrieval works correctly");
    
    Ok(())
}