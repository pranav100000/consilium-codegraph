use anyhow::Result;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;
use reviewbot::language_strategy::*;

/// Comprehensive edge case testing for robustness and error handling
/// These tests ensure the system gracefully handles unusual or problematic scenarios

#[test]
fn test_malformed_files_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🚫 Testing malformed files handling...");
    
    // Create package.json to make it a valid project
    fs::write(project_path.join("package.json"), r#"{"name": "test"}"#)?;
    
    // Create malformed TypeScript file with syntax errors
    let malformed_ts = r#"
// This file has intentional syntax errors
export class BrokenClass {
    constructor(
        // Missing closing parenthesis and implementation
    
    method1() {
        return "unclosed string
        // Missing quote and brace
    
    // Unmatched braces and brackets
    if (true { 
        console.log("broken");
    ]]
    
    // Invalid syntax combinations
    const = 123;
    function () {}
    class extends {}
"#;
    fs::write(project_path.join("malformed.ts"), malformed_ts)?;
    
    // Create malformed Python file
    let malformed_py = r#"
# Malformed Python with syntax errors
def broken_function(
    # Missing closing parenthesis
    
    if True
        print("missing colon")
        
    # Invalid indentation
wrong_indent = "bad"

    # Unclosed strings and brackets
    data = {"key": "unclosed
    list_data = [1, 2, 3
    
    # Invalid syntax
    def 123invalid():
        pass
        
    # Mixing tabs and spaces intentionally (if possible)
    	mixed_indent = "problem"
"#;
    fs::write(project_path.join("malformed.py"), malformed_py)?;
    
    // Create malformed Go file
    let malformed_go = r#"
package main

import (
    "fmt"
    // Missing closing parenthesis for import
    
func main() {
    // Missing opening brace
    fmt.Println("broken go")
    
    // Invalid syntax
    var = "missing type"
    const = 123
    
    // Unclosed function
    func incomplete(
    
    // Invalid struct
    type BrokenStruct struct {
        Field string
        // Missing closing brace
"#;
    fs::write(project_path.join("go.mod"), "module broken-test\ngo 1.20")?;
    fs::write(project_path.join("malformed.go"), malformed_go)?;
    
    // The system should handle malformed files gracefully
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully despite malformed files
    assert!(output.status.success(), 
        "System should handle malformed files gracefully. stderr: {}", stderr);
    
    // Should process the valid files (package.json, go.mod) and skip/warn about malformed ones
    // The exact behavior depends on implementation, but it shouldn't crash
    
    println!("✅ Malformed files handled gracefully");
    Ok(())
}

#[test]
fn test_missing_dependencies_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("📦 Testing missing dependencies handling...");
    
    // Create TypeScript project with missing dependencies
    let package_json = r#"{
  "name": "missing-deps-test",
  "version": "1.0.0",
  "dependencies": {
    "nonexistent-package": "1.0.0",
    "another-missing-dep": "^2.1.0"
  },
  "devDependencies": {
    "missing-dev-dep": "latest"
  }
}"#;
    fs::write(project_path.join("package.json"), package_json)?;
    
    let ts_content = r#"
import { SomeType } from 'nonexistent-package';
import { AnotherType } from 'another-missing-dep';

export class TestClass {
    private field: SomeType;
    
    constructor(value: AnotherType) {
        this.field = value.transform();
    }
}
"#;
    fs::write(project_path.join("test.ts"), ts_content)?;
    
    // Create Python project with missing requirements
    let requirements = r#"
nonexistent-python-package==1.2.3
another-missing-package>=2.0.0
definitely-not-real-package
"#;
    fs::write(project_path.join("requirements.txt"), requirements)?;
    
    let py_content = r#"
import nonexistent_python_package
from another_missing_package import SomeClass
from definitely_not_real_package.module import function

def test_function():
    obj = SomeClass()
    return function(obj)
"#;
    fs::write(project_path.join("test.py"), py_content)?;
    
    // System should handle missing dependencies gracefully
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully even with missing dependencies
    assert!(output.status.success(), 
        "System should handle missing dependencies gracefully. stderr: {}", stderr);
    
    println!("✅ Missing dependencies handled gracefully");
    Ok(())
}

#[test]
fn test_permission_denied_scenarios() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🔒 Testing permission denied scenarios...");
    
    // Create a basic project structure
    fs::write(project_path.join("package.json"), r#"{"name": "permission-test"}"#)?;
    fs::write(project_path.join("test.ts"), "export const value = 42;")?;
    
    // Create a directory and then make it inaccessible (on Unix systems)
    let restricted_dir = project_path.join("restricted");
    fs::create_dir(&restricted_dir)?;
    fs::write(restricted_dir.join("secret.ts"), "export const secret = 'hidden';")?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Remove all permissions from the directory
        let mut perms = fs::metadata(&restricted_dir)?.permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&restricted_dir, perms)?;
    }
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Restore permissions for cleanup
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&restricted_dir).unwrap_or_else(|_| {
            // If we can't read metadata, try to restore anyway
            fs::create_dir_all(&restricted_dir).ok();
            fs::metadata(&restricted_dir).unwrap()
        }).permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&restricted_dir, perms);
    }
    
    // Should complete successfully, handling permission errors gracefully
    assert!(output.status.success(), 
        "System should handle permission denied gracefully. stderr: {}", stderr);
    
    // Should process accessible files and warn about inaccessible ones
    println!("✅ Permission denied scenarios handled gracefully");
    Ok(())
}

#[test]
fn test_very_large_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("📏 Testing very large files handling...");
    
    fs::write(project_path.join("package.json"), r#"{"name": "large-file-test"}"#)?;
    
    // Create a very large TypeScript file (but not so large it causes issues in CI)
    let large_file_path = project_path.join("large.ts");
    let mut large_file = File::create(&large_file_path)?;
    
    writeln!(large_file, "// Very large TypeScript file for testing")?;
    writeln!(large_file, "export class LargeClass {{")?;
    
    // Generate many methods and properties (creates ~1MB file)
    for i in 0..10000 {
        writeln!(large_file, "  method{}(): string {{ return 'method{}'; }}", i, i)?;
        if i % 100 == 0 {
            writeln!(large_file, "  // Progress: {}/10000 methods", i)?;
        }
    }
    
    writeln!(large_file, "  // End of large class")?;
    writeln!(large_file, "}}")?;
    writeln!(large_file, "")?;
    writeln!(large_file, "export const LARGE_ARRAY = [")?;
    for i in 0..1000 {
        writeln!(large_file, "  'item{}',", i)?;
    }
    writeln!(large_file, "];")?;
    
    // Create a large Python file as well
    let large_py_path = project_path.join("large.py");
    let mut large_py_file = File::create(&large_py_path)?;
    
    writeln!(large_py_file, "# Very large Python file for testing")?;
    writeln!(large_py_file, "class LargePythonClass:")?;
    
    for i in 0..5000 {
        writeln!(large_py_file, "    def method_{}(self):", i)?;
        writeln!(large_py_file, "        return f'method_{}'", i)?;
        writeln!(large_py_file, "")?;
    }
    
    writeln!(large_py_file, "LARGE_DICT = {{")?;
    for i in 0..1000 {
        writeln!(large_py_file, "    '{}': 'value{}',", i, i)?;
    }
    writeln!(large_py_file, "}}")?;
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should handle large files without running out of memory or timing out
    assert!(output.status.success(), 
        "System should handle large files efficiently. stderr: {}", stderr);
    
    println!("✅ Large files handled efficiently");
    Ok(())
}

#[test]
fn test_binary_files_mixed_with_source() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🔢 Testing binary files mixed with source code...");
    
    fs::write(project_path.join("package.json"), r#"{"name": "binary-test"}"#)?;
    fs::write(project_path.join("valid.ts"), "export const value = 42;")?;
    
    // Create various binary files that might be in a project
    let binary_data = vec![0u8, 1, 2, 3, 255, 254, 128, 127]; // Some binary data
    
    fs::write(project_path.join("binary.exe"), &binary_data)?;
    fs::write(project_path.join("image.png"), &binary_data)?;
    fs::write(project_path.join("archive.zip"), &binary_data)?;
    fs::write(project_path.join("library.so"), &binary_data)?;
    fs::write(project_path.join("data.bin"), &binary_data)?;
    
    // Create files with misleading extensions
    fs::write(project_path.join("fake.ts"), &binary_data)?; // Binary data with .ts extension
    fs::write(project_path.join("fake.py"), &binary_data)?; // Binary data with .py extension
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully, processing valid source files and skipping binary files
    assert!(output.status.success(), 
        "System should handle binary files gracefully. stderr: {}", stderr);
    
    println!("✅ Binary files handled gracefully");
    Ok(())
}

#[test]
fn test_symlinks_and_circular_references() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🔗 Testing symlinks and circular references...");
    
    fs::write(project_path.join("package.json"), r#"{"name": "symlink-test"}"#)?;
    fs::write(project_path.join("real.ts"), "export const real = 'value';")?;
    
    // Create subdirectories
    let subdir = project_path.join("subdir");
    fs::create_dir(&subdir)?;
    fs::write(subdir.join("file.ts"), "export const sub = 'file';")?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs;
        
        // Create symbolic links (only on Unix systems)
        let _ = fs::symlink("../real.ts", project_path.join("link_to_real.ts"));
        let _ = fs::symlink("../subdir", project_path.join("link_to_subdir"));
        
        // Try to create a circular reference (may fail, but that's expected)
        let circular_dir = project_path.join("circular");
        std::fs::create_dir(&circular_dir).ok();
        let _ = fs::symlink("../circular", circular_dir.join("self"));
        
        // Link to non-existent file
        let _ = fs::symlink("../nonexistent.ts", project_path.join("broken_link.ts"));
    }
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully, handling symlinks appropriately
    assert!(output.status.success(), 
        "System should handle symlinks gracefully. stderr: {}", stderr);
    
    println!("✅ Symlinks and circular references handled gracefully");
    Ok(())
}

#[test]
fn test_unusual_file_names() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("📝 Testing unusual file names...");
    
    fs::write(project_path.join("package.json"), r#"{"name": "unusual-names-test"}"#)?;
    
    // Create files with unusual names that are still valid
    fs::write(project_path.join("normal.ts"), "export const normal = 1;")?;
    
    // Files with spaces
    fs::write(project_path.join("file with spaces.ts"), "export const spaces = 2;")?;
    
    // Files with special characters (that are valid on most filesystems)
    fs::write(project_path.join("file-with-dashes.ts"), "export const dashes = 3;")?;
    fs::write(project_path.join("file_with_underscores.ts"), "export const underscores = 4;")?;
    fs::write(project_path.join("file.with.dots.ts"), "export const dots = 5;")?;
    
    // Files with numbers
    fs::write(project_path.join("123numeric.ts"), "export const numeric = 6;")?;
    fs::write(project_path.join("file123.ts"), "export const mixed = 7;")?;
    
    // Files with unicode characters (if supported by filesystem)
    let unicode_name = "файл.ts"; // Russian characters
    if fs::write(project_path.join(unicode_name), "export const unicode = 8;").is_ok() {
        println!("   Created unicode filename test");
    }
    
    // Very long filename (but within filesystem limits)
    let long_name = format!("{}.ts", "a".repeat(100));
    fs::write(project_path.join(&long_name), "export const long = 9;")?;
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully, handling unusual filenames
    assert!(output.status.success(), 
        "System should handle unusual filenames gracefully. stderr: {}", stderr);
    
    println!("✅ Unusual file names handled gracefully");
    Ok(())
}

#[test]
fn test_empty_and_whitespace_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("⚪ Testing empty and whitespace-only files...");
    
    fs::write(project_path.join("package.json"), r#"{"name": "empty-test"}"#)?;
    
    // Completely empty file
    fs::write(project_path.join("empty.ts"), "")?;
    
    // File with only whitespace
    fs::write(project_path.join("whitespace.ts"), "   \n\t\n   \n")?;
    
    // File with only comments
    fs::write(project_path.join("comments.ts"), r#"
// This file only has comments
/* 
   Multi-line comment
   with no actual code
*/
// Another comment
"#)?;
    
    // File with valid content mixed with empty files
    fs::write(project_path.join("valid.ts"), "export const value = 42;")?;
    
    // Same for Python
    fs::write(project_path.join("empty.py"), "")?;
    fs::write(project_path.join("whitespace.py"), "   \n\t\n   \n")?;
    fs::write(project_path.join("comments.py"), r#"
# This file only has comments
"""
Multi-line comment
with no actual code
"""
# Another comment
"#)?;
    fs::write(project_path.join("requirements.txt"), "")?;
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully, handling empty files gracefully
    assert!(output.status.success(), 
        "System should handle empty files gracefully. stderr: {}", stderr);
    
    println!("✅ Empty and whitespace-only files handled gracefully");
    Ok(())
}

#[test] 
fn test_deeply_nested_directory_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🏗️ Testing deeply nested directory structure...");
    
    fs::write(project_path.join("package.json"), r#"{"name": "nested-test"}"#)?;
    
    // Create deeply nested directory structure
    let mut current_path = project_path.to_path_buf();
    for i in 0..20 {
        current_path = current_path.join(format!("level{}", i));
        fs::create_dir_all(&current_path)?;
        
        // Add a file at each level
        fs::write(
            current_path.join(format!("file{}.ts", i)),
            format!("export const level{} = {};", i, i)
        )?;
    }
    
    // Also create a wide structure (many siblings)
    let wide_dir = project_path.join("wide");
    fs::create_dir(&wide_dir)?;
    for i in 0..50 {
        fs::write(
            wide_dir.join(format!("wide{}.ts", i)),
            format!("export const wide{} = {};", i, i)
        )?;
    }
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should complete successfully, handling deep nesting
    assert!(output.status.success(), 
        "System should handle deeply nested structures gracefully. stderr: {}", stderr);
    
    println!("✅ Deeply nested directory structure handled gracefully");
    Ok(())
}

#[test]
fn test_language_strategy_edge_cases() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    println!("🔧 Testing language strategy edge cases...");
    
    // Test each strategy individually with edge cases
    let registry = LanguageStrategyRegistry::new();
    
    // Test empty directory
    let strategies = registry.detect_languages(project_path);
    assert!(strategies.is_empty(), "Empty directory should detect no languages");
    
    // Test directory with only non-source files
    fs::write(project_path.join("README.md"), "# Test project")?;
    fs::write(project_path.join("LICENSE"), "MIT License")?;
    fs::write(project_path.join(".gitignore"), "node_modules/")?;
    
    let strategies = registry.detect_languages(project_path);
    assert!(strategies.is_empty(), "Non-source files should not trigger language detection");
    
    // Test ambiguous files (multiple possible languages)
    fs::write(project_path.join("Makefile"), "all:\n\techo 'building'")?; // Could be C/C++
    
    let strategies = registry.detect_languages(project_path);
    // Should detect C++ due to Makefile
    let cpp_detected = strategies.iter().any(|s| matches!(s.language(), protocol::Language::Cpp));
    assert!(cpp_detected, "Makefile should trigger C++ detection");
    
    // Test conflicting build files
    fs::write(project_path.join("package.json"), r#"{"name": "test"}"#)?; // TypeScript/JS
    fs::write(project_path.join("Cargo.toml"), "[package]\nname = \"test\"")?; // Rust
    fs::write(project_path.join("go.mod"), "module test")?; // Go
    
    let strategies = registry.detect_languages(project_path);
    // Should detect multiple languages
    assert!(strategies.len() >= 3, "Should detect multiple languages with multiple build files");
    
    println!("✅ Language strategy edge cases handled correctly");
    Ok(())
}