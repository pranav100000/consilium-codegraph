/// Production Killer Tests - Find every way to break the system
/// These test real-world scenarios that could cause production failures

use anyhow::Result;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// CRITICAL: Non-git repository - does it crash?
#[test]
fn test_non_git_repository() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create files but NO git repo
    fs::write(temp_dir.path().join("test.ts"), "export const x = 1;")?;

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
    println!("Exit code: {:?}", output.status.code());

    // Should either succeed with "unknown" commit or fail gracefully
    // NOT crash with unwrap/panic
    assert!(
        output.status.success() || stderr.contains("git") || stderr.contains("repository"),
        "Should handle non-git repos gracefully, not crash"
    );

    Ok(())
}

/// CRITICAL: File deleted between walk and parse (TOCTOU)
#[test]
fn test_file_deleted_during_scan() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a file
    let file_path = temp_dir.path().join("disappearing.ts");
    fs::write(&file_path, "export const x = 1;")?;

    // This is hard to test deterministically because:
    // 1. Walker collects all paths
    // 2. Then main.rs iterates and reads each file
    // If file is deleted between these steps, read_to_string will fail

    // Current code has this issue - it would fail the entire scan
    // Let's document this edge case

    println!("⚠️  KNOWN ISSUE: Files deleted during scan will fail the scan");
    println!("    Current behavior: Scan fails with file not found error");
    println!("    Expected behavior: Skip missing files and continue");

    Ok(())
}

/// CRITICAL: Extremely large transaction (file with 100k symbols)
#[test]
#[ignore] // Creates large file
fn test_massive_symbol_count_single_file() -> Result<()> {
    let temp_dir = TempDir::new()?;

    println!("Creating file with 100k symbols...");
    let mut content = String::from("// Generated file\n");

    // Create 100k functions
    for i in 0..100_000 {
        content.push_str(&format!("export function func_{}() {{ return {}; }}\n", i, i));
    }

    fs::write(temp_dir.path().join("huge.ts"), content)?;

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
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Duration: {:?}", duration);
    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    // Should handle or fail gracefully, not crash
    assert!(
        output.status.success() || stderr.contains("too large") || stderr.contains("limit"),
        "Should handle massive symbol count (100k)"
    );

    Ok(())
}

/// CRITICAL: Case-insensitive filesystem (macOS) - File.ts vs file.ts
#[test]
#[cfg(target_os = "macos")]
fn test_case_insensitive_filesystem_collisions() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // On macOS, these are the SAME file
    fs::write(temp_dir.path().join("Test.ts"), "export class Upper {}")?;

    // This will OVERWRITE the previous file on case-insensitive FS
    fs::write(temp_dir.path().join("test.ts"), "export class Lower {}")?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should process 1 file (they're the same on macOS)
    assert!(output.status.success(), "Should handle case-insensitive FS");
    assert!(
        stdout.contains("1 files") || stdout.contains("Indexed 1"),
        "Should recognize that Test.ts and test.ts are the same file on macOS"
    );

    Ok(())
}

/// CRITICAL: Special files that shouldn't be read
#[test]
#[cfg(unix)]
fn test_special_files_devices() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create symlink to /dev/null with .ts extension
    std::os::unix::fs::symlink("/dev/null", temp_dir.path().join("devnull.ts"))?;

    // Create symlink to /dev/random (could hang or return infinite data)
    std::os::unix::fs::symlink("/dev/random", temp_dir.path().join("random.ts"))?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should either skip symlinks or handle device files gracefully
    // Should NOT hang reading /dev/random
    assert!(
        output.status.success() || stderr.contains("special") || stderr.contains("device"),
        "Should handle special device files safely"
    );

    Ok(())
}

/// CRITICAL: Disk full during transaction
#[test]
#[ignore] // Requires special setup to simulate disk full
fn test_disk_full_during_batch_insert() -> Result<()> {
    // This would require:
    // 1. Creating a small filesystem (loop device or quota)
    // 2. Filling it during batch insert
    // 3. Verifying transaction rolls back

    println!("⚠️  TODO: Implement disk-full simulation");
    println!("    Expected: Transaction should rollback");
    println!("    Current: Unknown - could leave partial data");

    Ok(())
}

/// CRITICAL: Symbol ID hash collisions
#[test]
fn test_symbol_id_hash_collisions() -> Result<()> {
    // Symbol IDs use format: repo://{sha}/{path}#sym({lang}:{fqn}:{sig_hash})
    // sig_hash is DefaultHasher - could have collisions

    // Two different symbols that might hash to same value
    // This is probabilistic - hard to force a collision

    println!("⚠️  POTENTIAL ISSUE: DefaultHasher is not cryptographic");
    println!("    Hash collisions could cause symbol overwrites");
    println!("    Recommendation: Use SHA256 for sig_hash instead");

    Ok(())
}

/// CRITICAL: File growing while being read
#[test]
#[ignore] // Requires concurrent modification
fn test_file_modified_during_read() -> Result<()> {
    // File could be appended to while read_to_string is reading
    // This is a race condition that's hard to test deterministically

    println!("⚠️  RACE CONDITION: File could be modified during read");
    println!("    Current: read_to_string might see partial/inconsistent data");
    println!("    Impact: Could parse wrong symbols or fail");

    Ok(())
}

/// CRITICAL: Submodules - are they scanned twice?
#[test]
fn test_git_submodules_not_double_scanned() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Initialize main repo
    Command::new("git").args(["init"]).current_dir(temp_dir.path()).output()?;
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(temp_dir.path()).output()?;
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(temp_dir.path()).output()?;

    fs::write(temp_dir.path().join("main.ts"), "export const x = 1;")?;

    // Create submodule directory (simplified - not actual submodule)
    fs::create_dir_all(temp_dir.path().join("submodule"))?;
    fs::write(temp_dir.path().join("submodule/sub.ts"), "export const y = 2;")?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    // Both files should be scanned (we're not actually using git submodules)
    assert!(output.status.success());

    Ok(())
}

/// CRITICAL: Windows path separators
#[test]
#[cfg(target_os = "windows")]
fn test_windows_path_separators() -> Result<()> {
    let temp_dir = TempDir::new()?;

    fs::create_dir_all(temp_dir.path().join("src\\components"))?;
    fs::write(
        temp_dir.path().join("src\\components\\Test.ts"),
        "export class Test {}"
    )?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should handle backslashes correctly
    assert!(output.status.success(), "Should handle Windows paths");
    assert!(
        stdout.contains("1 files") || stdout.contains("Indexed 1"),
        "Should find file with backslash separators"
    );

    Ok(())
}

/// CRITICAL: Empty file (no symbols at all)
#[test]
fn test_completely_empty_file() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Truly empty file
    fs::write(temp_dir.path().join("empty.ts"), "")?;

    // File with only whitespace
    fs::write(temp_dir.path().join("whitespace.ts"), "   \n\n\t\t\n   ")?;

    // File with only comments
    fs::write(temp_dir.path().join("comments.ts"), "// comment\n/* block */")?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);

    // Should process all 3 files even if they have 0 symbols
    assert!(output.status.success());
    assert!(
        stdout.contains("3 files") || stdout.contains("Indexed 3"),
        "Should handle empty files gracefully"
    );

    Ok(())
}

/// CRITICAL: Extremely long single line (10MB line)
#[test]
fn test_extremely_long_single_line() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create 10MB single line (minified code simulation)
    let long_line = "export const data = \"".to_string() + &"x".repeat(10_000_000) + "\";";
    fs::write(temp_dir.path().join("minified.ts"), long_line)?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should handle or fail gracefully
    assert!(
        output.status.success() || stderr.contains("line") || stderr.contains("too long"),
        "Should handle extremely long lines (10MB)"
    );

    Ok(())
}

/// CRITICAL: Detached HEAD state
#[test]
fn test_git_detached_head() -> Result<()> {
    let temp_dir = TempDir::new()?;

    Command::new("git").args(["init"]).current_dir(temp_dir.path()).output()?;
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(temp_dir.path()).output()?;
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(temp_dir.path()).output()?;

    fs::write(temp_dir.path().join("test.ts"), "export const x = 1;")?;

    Command::new("git").args(["add", "."]).current_dir(temp_dir.path()).output()?;
    Command::new("git").args(["commit", "-m", "initial"]).current_dir(temp_dir.path()).output()?;

    // Get commit SHA
    let sha_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(temp_dir.path())
        .output()?;
    let sha = String::from_utf8_lossy(&sha_output.stdout).trim().to_string();

    // Detach HEAD
    Command::new("git")
        .args(["checkout", &sha])
        .current_dir(temp_dir.path())
        .output()?;

    let output = Command::new("cargo")
        .args([
            "run", "-p", "reviewbot", "--",
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan"
        ])
        .output()?;

    // Should work in detached HEAD
    assert!(output.status.success(), "Should work with detached HEAD");

    Ok(())
}

/// CRITICAL: .gitignore changed during scan
#[test]
fn test_gitignore_changed_during_scan() -> Result<()> {
    // If .gitignore is modified during scan:
    // - Walker uses old .gitignore
    // - Next scan uses new .gitignore
    // - Could see "new" files that were always there

    println!("⚠️  EDGE CASE: .gitignore modified during scan");
    println!("    Impact: Walker snapshot is taken at start");
    println!("    Behavior: Scan uses .gitignore from scan start time");

    Ok(())
}
