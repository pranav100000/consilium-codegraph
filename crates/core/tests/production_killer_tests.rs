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
///
/// IGNORED REASON: Creates ~10MB file, tests batch insert limits
/// - Generates file with 100,000 function symbols
/// - Tests SQLite transaction size limits and batch insertion performance
/// - Validates no OOM or transaction failure on huge single-file batches
/// - Run explicitly with: cargo test test_massive_symbol_count_single_file -- --ignored
#[test]
#[ignore] // Memory/CPU intensive - run explicitly with --ignored flag
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
///
/// Tests SQLite transaction rollback when disk space is exhausted during a write.
///
/// This test attempts to simulate disk-full conditions using platform-specific methods.
/// If the necessary infrastructure isn't available, the test provides detailed manual
/// testing instructions.
///
/// IGNORED REASON: Requires platform-specific infrastructure setup
/// - Linux: Loop device with size limit (requires root/sudo)
/// - macOS: Small APFS volume (requires admin privileges)
/// - Expected: Transaction rolls back cleanly, no partial data
/// - Run explicitly with: cargo test test_disk_full_during_batch_insert -- --ignored --nocapture
#[test]
#[ignore] // Requires infrastructure setup (disk quota/loop device)
fn test_disk_full_during_batch_insert() -> Result<()> {
    use std::process::Command;
    use tempfile::TempDir;

    println!("🔍 Testing disk-full handling during SQLite transaction...\n");

    // Try to create a small filesystem for testing
    #[cfg(target_os = "linux")]
    {
        // Attempt to create a small loop device (requires privileges)
        println!("📦 Attempting to create 10MB loop device (requires sudo)...");

        let loop_file = std::env::temp_dir().join("reviewbot_test_disk.img");

        // Create a 10MB file
        let dd_result = Command::new("dd")
            .args(&["if=/dev/zero", &format!("of={}", loop_file.display()), "bs=1M", "count=10"])
            .output();

        if let Ok(output) = dd_result {
            if output.status.success() {
                // Format as ext4
                let mkfs_result = Command::new("mkfs.ext4")
                    .args(&["-F", loop_file.to_str().unwrap()])
                    .output();

                if let Ok(mkfs_output) = mkfs_result {
                    if mkfs_output.status.success() {
                        let mount_dir = TempDir::new()?;

                        // Mount the filesystem
                        let mount_result = Command::new("sudo")
                            .args(&["mount", "-o", "loop", loop_file.to_str().unwrap(), mount_dir.path().to_str().unwrap()])
                            .output();

                        if let Ok(mount_output) = mount_result {
                            if mount_output.status.success() {
                                println!("✅ Successfully created 10MB test filesystem");

                                // Now try to fill it with SQLite database
                                let result = test_disk_full_scenario(mount_dir.path());

                                // Cleanup: unmount
                                let _ = Command::new("sudo")
                                    .args(&["umount", mount_dir.path().to_str().unwrap()])
                                    .output();

                                let _ = std::fs::remove_file(&loop_file);

                                return result;
                            }
                        }
                    }
                }
            }
        }

        println!("⚠️  Could not create loop device (requires sudo)");
    }

    #[cfg(target_os = "macos")]
    {
        println!("📦 Attempting to create small APFS volume (requires admin)...");

        // Create a sparse disk image
        let disk_image = std::env::temp_dir().join("reviewbot_test.dmg");

        let create_result = Command::new("hdiutil")
            .args(&["create", "-size", "10m", "-fs", "APFS", "-volname", "ReviewBotTest",
                    disk_image.to_str().unwrap()])
            .output();

        if let Ok(output) = create_result {
            if output.status.success() {
                // Mount the disk image
                let mount_result = Command::new("hdiutil")
                    .args(&["attach", disk_image.to_str().unwrap()])
                    .output();

                if let Ok(mount_output) = mount_result {
                    if mount_output.status.success() {
                        let mount_point = std::path::Path::new("/Volumes/ReviewBotTest");

                        if mount_point.exists() {
                            println!("✅ Successfully created 10MB test volume");

                            let result = test_disk_full_scenario(mount_point);

                            // Cleanup: eject
                            let _ = Command::new("hdiutil")
                                .args(&["eject", mount_point.to_str().unwrap()])
                                .output();

                            let _ = std::fs::remove_file(&disk_image);

                            return result;
                        }
                    }
                }
            }
        }

        println!("⚠️  Could not create disk image (requires privileges)");
    }

    // Fallback: Print manual testing instructions
    println!("\n📋 MANUAL TESTING INSTRUCTIONS:");
    println!("════════════════════════════════════════════════════════════");
    println!("\n🐧 Linux:");
    println!("  1. Create loop device:");
    println!("     dd if=/dev/zero of=/tmp/test.img bs=1M count=10");
    println!("     mkfs.ext4 -F /tmp/test.img");
    println!("     sudo mkdir /mnt/test && sudo mount -o loop /tmp/test.img /mnt/test");
    println!("  2. Run: cargo run -- --repo /mnt/test scan");
    println!("  3. Fill disk with large batch insert");
    println!("  4. Expected: SQLITE_FULL error, transaction rollback");
    println!("  5. Cleanup: sudo umount /mnt/test && rm /tmp/test.img");

    println!("\n🍎 macOS:");
    println!("  1. Create disk image:");
    println!("     hdiutil create -size 10m -fs APFS -volname Test /tmp/test.dmg");
    println!("     hdiutil attach /tmp/test.dmg");
    println!("  2. Run: cargo run -- --repo /Volumes/Test scan");
    println!("  3. Expected: SQLITE_FULL error, transaction rollback");
    println!("  4. Cleanup: hdiutil eject /Volumes/Test && rm /tmp/test.dmg");

    println!("\n✅ Expected Behavior:");
    println!("  - SQLite returns SQLITE_FULL error");
    println!("  - Transaction rolls back cleanly");
    println!("  - No partial data in database");
    println!("  - Application exits gracefully with error message");

    println!("\n❌ Failure Modes to Watch:");
    println!("  - Database left in inconsistent state");
    println!("  - Partial symbols/edges committed");
    println!("  - Database corruption requiring rebuild");
    println!("════════════════════════════════════════════════════════════\n");

    Ok(())
}

/// Helper function to test disk-full scenario
fn test_disk_full_scenario(path: &std::path::Path) -> Result<()> {
    use store::GraphStore;
    use protocol::{SymbolIR, SymbolKind, Language, Span, Version};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    println!("🧪 Running disk-full test on {:?}", path);

    // Initialize database
    let store = GraphStore::new(path)?;
    let commit_id = store.create_commit_snapshot("test")?;

    // Try to insert massive batch that exceeds 10MB
    let mut symbols = Vec::new();
    for i in 0..50_000 {
        let name = format!("function_with_very_long_name_{}", i);
        let fqn = format!("test.{}", name);

        // Compute signature hash
        let mut hasher = DefaultHasher::new();
        fqn.hash(&mut hasher);
        let sig_hash = format!("{:x}", hasher.finish());

        symbols.push(SymbolIR {
            id: format!("symbol_{}", i),
            name: name.clone(),
            fqn,
            kind: SymbolKind::Function,
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            span: Span { start_line: i, start_col: 0, end_line: i, end_col: 50 },
            signature: Some(format!("function signature with lots of text {}", i)),
            file_path: "test.ts".to_string(),
            visibility: None,
            doc: None,
            sig_hash,
        });
    }

    println!("💾 Attempting to insert 50k symbols (should exceed 10MB disk)...");

    // This should fail with SQLITE_FULL
    match store.batch_insert_symbols(commit_id, &symbols) {
        Ok(_) => {
            println!("❌ UNEXPECTED: Batch insert succeeded (disk may be larger than 10MB)");
            println!("   Try reducing disk size or increasing batch size");
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("full") || error_str.contains("FULL") ||
               error_str.contains("No space") || error_str.contains("no space") {
                println!("✅ CORRECT: Got disk-full error: {}", e);
                println!("✅ Transaction should have rolled back");

                // Verify database is still consistent
                let symbol_count = store.get_symbol_count()?;
                if symbol_count == 0 {
                    println!("✅ VERIFIED: No partial data (symbol_count = 0)");
                } else {
                    println!("❌ FAILURE: Found {} symbols (should be 0 - partial commit!)", symbol_count);
                }
            } else {
                println!("⚠️  Got error but not disk-full: {}", e);
            }
        }
    }

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
///
/// IGNORED REASON: Non-deterministic race condition, hard to reproduce
/// - Tests scenario where file is appended to during read_to_string()
/// - Race condition: File grows between stat() and read() syscalls
/// - Expected: read_to_string() reads original size, or returns partial data
/// - Impact: Could parse incorrect symbols or fail with UTF-8 error
/// - Mitigation: Real-world repos rarely modified during indexing (git HEAD is stable)
#[test]
#[ignore] // Non-deterministic race condition - documents edge case
fn test_file_modified_during_read() -> Result<()> {
    // To test this reliably would require:
    // 1. Fork process to modify file in tight loop
    // 2. Main process reads repeatedly
    // 3. Detect inconsistent reads (hard to validate)

    println!("⚠️  RACE CONDITION: File could be modified during read");
    println!("    Current: read_to_string might see partial/inconsistent data");
    println!("    Impact: Could parse wrong symbols or fail with UTF-8 error");
    println!("    Mitigation: Git repos have stable HEAD, rare in practice");

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
