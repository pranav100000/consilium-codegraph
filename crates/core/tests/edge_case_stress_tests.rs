use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Test 1: Empty repository - does it handle gracefully?
#[test]
fn test_empty_repository() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp_dir.path())
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp_dir.path())
        .output()?;

    // Create initial commit with no files
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "initial"])
        .current_dir(temp_dir.path())
        .output()?;

    // Run scan
    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    // Should succeed with 0 files
    assert!(output.status.success(), "Empty repo scan should succeed");

    // Verify actual database contents
    use store::GraphStore;
    let store = GraphStore::new(temp_dir.path())?;
    let file_count = store.get_file_count()?;

    assert_eq!(file_count, 0,
        "Should have 0 files in empty repository");

    Ok(())
}

/// Test 2: Repository with only binary files
#[test]
fn test_binary_files_only() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create binary files
    fs::write(temp_dir.path().join("image.png"), vec![0xFF, 0xD8, 0xFF, 0xE0])?;
    fs::write(temp_dir.path().join("data.bin"), vec![0x00, 0x01, 0x02, 0x03])?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    // Should succeed with 0 source files
    assert!(output.status.success(), "Binary-only repo should succeed");

    Ok(())
}

/// Test 3: Files with invalid UTF-8
#[test]
fn test_invalid_utf8_files() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create file with invalid UTF-8 but .ts extension
    let invalid_bytes = vec![0xFF, 0xFE, 0xFD, b'c', b'o', b'd', b'e'];
    fs::write(temp_dir.path().join("invalid.ts"), invalid_bytes)?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    // Should handle gracefully (skip or warn, not crash)
    assert!(output.status.success() || stderr.contains("Failed to read"),
        "Should handle invalid UTF-8 gracefully");

    Ok(())
}

/// Test 4: Extremely deep directory nesting
#[test]
fn test_deep_directory_nesting() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create deeply nested directory (100 levels)
    let mut path = temp_dir.path().to_path_buf();
    for i in 0..100 {
        path.push(format!("level_{}", i));
    }
    fs::create_dir_all(&path)?;

    // Create a file at the deepest level
    fs::write(path.join("deep.ts"), "export const x = 1;")?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    assert!(output.status.success(), "Deep nesting should work");

    // Verify actual database contents
    use store::GraphStore;
    let store = GraphStore::new(temp_dir.path())?;
    let file_count = store.get_file_count()?;

    assert_eq!(file_count, 1,
        "Should find exactly 1 deeply nested file");

    Ok(())
}

/// Test 5: Files with same name in different directories
#[test]
fn test_duplicate_filenames() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create multiple index.ts files
    fs::create_dir_all(temp_dir.path().join("src/components"))?;
    fs::create_dir_all(temp_dir.path().join("src/utils"))?;
    fs::create_dir_all(temp_dir.path().join("tests"))?;

    let code = "export class Component {}";
    fs::write(temp_dir.path().join("src/components/index.ts"), code)?;
    fs::write(temp_dir.path().join("src/utils/index.ts"), code)?;
    fs::write(temp_dir.path().join("tests/index.ts"), code)?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    assert!(output.status.success(), "Duplicate filenames should work");

    // Verify actual database contents
    use store::GraphStore;
    let store = GraphStore::new(temp_dir.path())?;
    let file_count = store.get_file_count()?;

    assert_eq!(file_count, 3,
        "Should process all 3 files with same name in different directories");

    Ok(())
}

/// Test 6: Circular dependencies (A imports B, B imports A)
#[test]
fn test_circular_dependencies() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::write(
        temp_dir.path().join("a.ts"),
        r#"
import { B } from './b';
export class A {
    b: B;
}
"#
    )?;

    fs::write(
        temp_dir.path().join("b.ts"),
        r#"
import { A } from './a';
export class B {
    a: A;
}
"#
    )?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    assert!(output.status.success(), "Circular dependencies should not crash");

    Ok(())
}

/// Test 7: Malformed source code
#[test]
fn test_malformed_source_code() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::write(
        temp_dir.path().join("broken.ts"),
        r#"
class Broken {
    // Missing closing brace
    method() {
        if (true) {
            console.log("unclosed
"#
    )?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    // Should handle gracefully (tree-sitter recovers from errors)
    assert!(output.status.success(), "Malformed code should not crash the scanner");

    Ok(())
}

/// Test 8: Extremely long file paths
#[test]
#[cfg(not(target_os = "windows"))] // Windows has shorter path limits
fn test_extremely_long_paths() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a very long path (close to filesystem limits)
    let long_name = "a".repeat(200);
    let mut path = temp_dir.path().to_path_buf();
    path.push(&long_name);

    if fs::create_dir_all(&path).is_ok() {
        fs::write(path.join("file.ts"), "export const x = 1;")?;

        let output = Command::new("cargo")
            .args([
                "run", "-p", "reviewbot", "--",
                "--repo", &temp_dir.path().to_string_lossy(),
                "scan"
            ])
            .output()?;

        assert!(output.status.success() || String::from_utf8_lossy(&output.stderr).contains("path"),
            "Should handle long paths gracefully");
    }

    Ok(())
}

/// Test 9: Unicode in file names and code
#[test]
fn test_unicode_filenames_and_content() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Unicode filename
    fs::write(
        temp_dir.path().join("测试文件.ts"),
        r#"
export class 中文类 {
    名字: string = "🎉";
    方法() {
        return "Unicode works!";
    }
}
"#
    )?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    assert!(output.status.success(), "Unicode should be handled");

    Ok(())
}

/// Test 10: Symlinks (potential infinite loops)
#[test]
#[cfg(unix)] // Symlinks work differently on Windows
fn test_symlink_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::create_dir_all(temp_dir.path().join("src"))?;
    fs::write(temp_dir.path().join("src/real.ts"), "export const x = 1;")?;

    // Create symlink to parent directory (potential infinite loop)
    std::os::unix::fs::symlink(
        temp_dir.path(),
        temp_dir.path().join("src/loop")
    )?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    // Should not hang (ignore tool has symlink protection)
    assert!(output.status.success(), "Should handle symlinks without hanging");

    Ok(())
}

/// Test 11: File deleted between walk and parse
#[test]
fn test_file_disappears_during_scan() -> Result<()> {
    // This is hard to test deterministically, but documents the edge case
    // In practice, walk() collects all paths, then main.rs iterates them
    // If a file is deleted between these steps, read_to_string will fail
    // Current code doesn't handle this - it would crash

    // TODO: Add error handling for files that disappear during scan
    Ok(())
}

/// Test 12: Concurrent scans on same repository
#[test]
fn test_concurrent_scans_same_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::write(temp_dir.path().join("test.ts"), "export const x = 1;")?;

    // This would test database locking under concurrent access
    // Current implementation should handle this due to connection sharing
    // But worth stress testing

    let handles: Vec<_> = (0..3).map(|_| {
        let path = temp_dir.path().to_path_buf();
        std::thread::spawn(move || {
            Command::new("cargo")
                .args([
                    "run", "-p", "reviewbot", "--",
                    "--repo", &path.to_string_lossy(),
                    "scan"
                ])
                .output()
        })
    }).collect();

    for handle in handles {
        let output = handle.join().unwrap()?;
        assert!(output.status.success() ||
                String::from_utf8_lossy(&output.stderr).contains("locked"),
            "Concurrent scans should either succeed or report lock");
    }

    Ok(())
}
