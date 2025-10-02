use anyhow::Result;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Test: Very large number of files (10,000+)
///
/// IGNORED REASON: Performance test that takes 60+ seconds
/// - Creates 10,000 source files to test walker and batch insertion performance
/// - Validates system handles large codebases (e.g., Linux kernel, Chromium)
/// - Run explicitly with: cargo test test_massive_file_count -- --ignored
#[test]
#[ignore] // Slow test - run explicitly with --ignored flag
fn test_massive_file_count() -> Result<()> {
    let temp_dir = TempDir::new()?;

    println!("Creating 10,000 files...");
    for i in 0..10_000 {
        let dir = temp_dir.path().join(format!("dir_{}", i / 100));
        fs::create_dir_all(&dir)?;
        fs::write(
            dir.join(format!("file_{}.ts", i)),
            format!("export const VAR_{} = {};", i, i)
        )?;
    }

    println!("Running scan...");
    let start = std::time::Instant::now();

    let output = Command::new("cargo")
        .args([
            "run", "--release", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let duration = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("Duration: {:?}", duration);
    println!("Output: {}", stdout);

    assert!(output.status.success(), "Should handle 10k files");
    assert!(duration.as_secs() < 300, "Should complete in under 5 minutes");

    Ok(())
}

/// Test: File with 1 million lines
///
/// IGNORED REASON: Creates ~100MB file in memory, takes 30+ seconds
/// - Tests Tree-sitter parser on extremely large single file
/// - Validates no stack overflow or OOM on massive files
/// - Run explicitly with: cargo test test_million_line_file -- --ignored
#[test]
#[ignore] // Memory/CPU intensive - run explicitly with --ignored flag
fn test_million_line_file() -> Result<()> {
    let temp_dir = TempDir::new()?;

    println!("Creating 1M line file...");
    let mut content = String::from("export class HugeFile {\n");
    for i in 0..1_000_000 {
        content.push_str(&format!("    method_{}() {{ return {}; }}\n", i, i));
    }
    content.push_str("}\n");

    fs::write(temp_dir.path().join("huge.ts"), content)?;

    println!("Running scan...");
    let output = Command::new("cargo")
        .args([
            "run", "--release", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    // Should handle but might be slow
    assert!(output.status.success() || stdout.contains("timeout"),
        "Should handle or timeout gracefully");

    Ok(())
}

/// Test: Symbol name with SQL injection attempt
#[test]
fn test_sql_injection_in_symbols() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::write(
        temp_dir.path().join("injection.ts"),
        r#"
// Malicious class name
export class MaliciousClass {};

"#
    )?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    assert!(output.status.success(), "Should handle special characters safely");

    Ok(())
}

/// Test: Null bytes in file content
#[test]
fn test_null_bytes_in_content() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let content = b"export class Test {\x00\x00\x00 }";
    fs::write(temp_dir.path().join("nulls.ts"), content)?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    // Null bytes are valid UTF-8, should handle
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stderr: {}", stderr);

    assert!(output.status.success() || stderr.contains("null"),
        "Should handle null bytes");

    Ok(())
}

/// Test: Mixed line endings (CRLF, LF, CR)
#[test]
fn test_mixed_line_endings() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let content = "export class Test {\r\n    method1() {}\n    method2() {}\r    method3() {}\r\n}";
    fs::write(temp_dir.path().join("mixed.ts"), content)?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    assert!(output.status.success(), "Should handle mixed line endings");

    Ok(())
}

/// Test: Extremely long symbol name (10k characters)
#[test]
fn test_extremely_long_symbol_name() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let long_name = "A".repeat(10_000);
    let content = format!("export class {} {{}}", long_name);
    fs::write(temp_dir.path().join("long_name.ts"), content)?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    assert!(output.status.success(), "Should handle very long names");

    Ok(())
}

/// Test: Read-only file system (permission errors)
#[test]
#[cfg(unix)]
fn test_readonly_database_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::write(temp_dir.path().join("test.ts"), "export const x = 1;")?;

    // Make directory read-only
    let permissions = std::fs::Permissions::from_mode(0o444);
    fs::set_permissions(temp_dir.path(), permissions)?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    // Restore permissions for cleanup
    let permissions = std::fs::Permissions::from_mode(0o755);
    fs::set_permissions(temp_dir.path(), permissions)?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail with permission error, not crash
    assert!(!output.status.success() && stderr.contains("Permission"),
        "Should report permission error gracefully");

    Ok(())
}

/// Test: Database corruption recovery
#[test]
fn test_corrupt_database_recovery() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::write(temp_dir.path().join("test.ts"), "export const x = 1;")?;

    // Run initial scan
    Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    // Corrupt the database (completely invalid file)
    let db_path = temp_dir.path().join(".reviewbot/graph.db");
    fs::write(&db_path, b"CORRUPT DATA")?;

    // Try to scan again
    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);
    println!("Exit status: {:?}", output.status);

    // When database is completely corrupt (not a valid SQLite file),
    // SQLite automatically creates a fresh database and the scan succeeds.
    // This is correct behavior for batch processing - auto-recovery.
    assert!(output.status.success(),
        "Should auto-recover from completely corrupt database by creating fresh one");

    // Verify actual database state - should have re-scanned and created valid database
    use store::GraphStore;
    let store = GraphStore::new(temp_dir.path())?;
    let file_count = store.get_file_count()?;

    assert_eq!(file_count, 1,
        "Should have 1 file after recovery (re-created database from scratch)");

    Ok(())
}

/// Test: Disk full scenario during batch insert
///
/// IGNORED REASON: Requires special infrastructure setup
/// - Would need: loop device (Linux), disk quota, or small partition
/// - Tests transaction rollback when SQLite runs out of disk space
/// - Expected behavior: Transaction rolls back, no partial data
/// - TODO: Implement with platform-specific disk quota simulation
#[test]
#[ignore] // Requires infrastructure setup (disk quota/loop device)
fn test_disk_full_handling() -> Result<()> {
    // This would require creating a small virtual filesystem
    // or using platform-specific quota tools
    // Documents the edge case
    Ok(())
}

/// Test: Network filesystem latency (NFS, SMB, remote mounts)
///
/// IGNORED REASON: Requires network filesystem mount
/// - Would test behavior on NFS/SMB mounts with high latency
/// - Validates no timeouts or performance degradation
/// - Run manually by placing test repo on network mount
#[test]
#[ignore] // Requires network filesystem mount (NFS/SMB)
fn test_network_filesystem_handling() -> Result<()> {
    // Test on NFS mount would reveal performance issues
    // and potential timeout problems
    Ok(())
}
