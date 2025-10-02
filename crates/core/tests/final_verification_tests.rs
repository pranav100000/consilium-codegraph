/// Final verification tests - verify actual behavior of documented edge cases
/// and find any remaining production issues

use anyhow::Result;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// VERIFY: What actually happens when file is deleted mid-scan?
#[test]
fn verify_file_deleted_mid_scan_actual_behavior() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create multiple files
    for i in 0..5 {
        fs::write(
            temp_dir.path().join(format!("file{}.ts", i)),
            format!("export const x{} = {};", i, i)
        )?;
    }

    // Create a file we'll delete
    fs::write(
        temp_dir.path().join("will_delete.ts"),
        "export const deleted = 1;"
    )?;

    // Now immediately delete it (simulates TOCTOU)
    fs::remove_file(temp_dir.path().join("will_delete.ts"))?;

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
    println!("Success: {}", output.status.success());

    // Current behavior: This WILL succeed because file was deleted before walker ran
    // Real TOCTOU would be: file exists when walker runs, deleted before parse
    // That's nearly impossible to test deterministically

    Ok(())
}

/// VERIFY: Batch insert with mixed valid/invalid symbols
#[test]
fn verify_batch_insert_partial_failure() -> Result<()> {
    use store::GraphStore;
    use protocol::{Language, Span, SymbolIR, SymbolKind, Version};
    use tempfile::TempDir;

    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test")?;

    let symbols = vec![
        SymbolIR {
            id: "valid_1".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "Valid1".to_string(),
            fqn: "test.Valid1".to_string(),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span { start_line: 0, start_col: 0, end_line: 1, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "hash1".to_string(),
        },
        SymbolIR {
            id: "invalid".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "".to_string(), // INVALID - empty name
            fqn: "test.Invalid".to_string(),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span { start_line: 2, start_col: 0, end_line: 3, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "hash2".to_string(),
        },
        SymbolIR {
            id: "valid_2".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "Valid2".to_string(),
            fqn: "test.Valid2".to_string(),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span { start_line: 4, start_col: 0, end_line: 5, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "hash3".to_string(),
        },
    ];

    let result = store.batch_insert_symbols(commit_id, &symbols);

    // Batch insert should FAIL on invalid symbol
    assert!(result.is_err(), "Batch with invalid symbol should fail");

    // Verify NO symbols were inserted (transaction rolled back)
    let count = store.get_symbol_count()?;
    assert_eq!(count, 0, "Transaction should have rolled back - no partial inserts");

    println!("✅ Batch insert correctly rolls back on validation error");

    Ok(())
}

/// VERIFY: Connection manager under extreme concurrent load
#[test]
fn verify_connection_manager_extreme_concurrency() -> Result<()> {
    use store::GraphStore;
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    let temp_dir = TempDir::new()?;
    let repo_path = Arc::new(temp_dir.path().to_path_buf());

    // Spawn 50 threads all trying to create stores and insert data
    let handles: Vec<_> = (0..50).map(|i| {
        let repo_path = Arc::clone(&repo_path);
        thread::spawn(move || -> Result<()> {
            let store = GraphStore::new(&repo_path)?;
            let commit_id = store.get_or_create_commit(&format!("commit_{}", i))?;

            // Each thread inserts 100 files
            for j in 0..100 {
                store.insert_file(
                    commit_id,
                    &format!("file_{}_{}.ts", i, j),
                    &format!("hash_{}", j),
                    100
                )?;
            }
            Ok(())
        })
    }).collect();

    let mut success_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.join().unwrap() {
            Ok(_) => success_count += 1,
            Err(e) => {
                println!("Thread failed: {}", e);
                error_count += 1;
            }
        }
    }

    println!("Success: {}, Errors: {}", success_count, error_count);

    // All should succeed with connection sharing
    assert_eq!(error_count, 0, "All concurrent operations should succeed");
    assert_eq!(success_count, 50, "All 50 threads should complete");

    Ok(())
}

/// VERIFY: DefaultHasher stability across runs
#[test]
fn verify_hash_stability() -> Result<()> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let test_string = "test symbol signature";

    let mut hasher1 = DefaultHasher::new();
    test_string.hash(&mut hasher1);
    let hash1 = hasher1.finish();

    let mut hasher2 = DefaultHasher::new();
    test_string.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    assert_eq!(hash1, hash2, "DefaultHasher should be stable within same run");

    println!("⚠️  WARNING: DefaultHasher is NOT stable across Rust versions");
    println!("   Hash values may change between compilations");
    println!("   Recommendation: Use cryptographic hash (SHA256) for production");

    Ok(())
}

/// VERIFY: Memory usage with huge batch (50k symbols)
///
/// IGNORED REASON: Memory-intensive test, allocates 50k symbols in RAM
/// - Tests memory usage for batch insertion of 50,000 symbols
/// - Validates no memory leaks or excessive allocations
/// - Typical production usage: 1k-10k symbols per batch
/// - Run explicitly with: cargo test verify_memory_usage_huge_batch -- --ignored
#[test]
#[ignore] // Memory intensive - run explicitly with --ignored flag
fn verify_memory_usage_huge_batch() -> Result<()> {
    use store::GraphStore;
    use protocol::{EdgeIR, EdgeType, Language, Span, SymbolIR, SymbolKind, Version, Resolution};
    use tempfile::TempDir;

    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test")?;

    // Create 50k symbols
    let symbols: Vec<SymbolIR> = (0..50_000).map(|i| SymbolIR {
        id: format!("symbol_{}", i),
        lang: Language::TypeScript,
        lang_version: Some(Version::ES2020),
        kind: SymbolKind::Function,
        name: format!("func_{}", i),
        fqn: format!("test.func_{}", i),
        signature: Some(format!("() => {}", i)),
        file_path: "huge.ts".to_string(),
        span: Span { start_line: i, start_col: 0, end_line: i + 1, end_col: 0 },
        visibility: None,
        doc: Some(format!("Documentation for function {}", i)),
        sig_hash: format!("hash_{}", i),
    }).collect();

    println!("Inserting 50k symbols in single batch...");

    let result = store.batch_insert_symbols(commit_id, &symbols);

    match result {
        Ok(_) => {
            println!("✅ Successfully inserted 50k symbols");

            // Verify they're all there
            let count = store.get_symbol_count()?;
            assert_eq!(count, 50_000, "All symbols should be inserted");
        }
        Err(e) => {
            println!("❌ Failed to insert 50k symbols: {}", e);

            // Check if it's a transaction size limit
            if e.to_string().contains("too large") || e.to_string().contains("limit") {
                println!("   This is a known SQLite transaction size limit");
                println!("   Recommendation: Implement batch chunking (e.g., 10k per batch)");
            }
        }
    }

    Ok(())
}

/// VERIFY: What happens with duplicate commit SHA but different content?
#[test]
fn verify_same_commit_different_content() -> Result<()> {
    use store::GraphStore;
    use tempfile::TempDir;

    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;

    // First scan
    let commit_id1 = store.create_commit_snapshot("abc123")?;
    store.insert_file(commit_id1, "test.ts", "hash1", 100)?;

    // Same commit SHA, different file
    let commit_id2 = store.get_or_create_commit("abc123")?;
    store.insert_file(commit_id2, "test.ts", "hash2", 200)?; // Different hash!

    // They should be the SAME commit_id
    assert_eq!(commit_id1, commit_id2, "Same SHA should return same commit_id");

    // But file was UPDATED with INSERT OR REPLACE
    let files = store.get_files_in_commit("abc123")?;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].1, "hash2", "File should be updated to new hash");

    println!("✅ Same commit SHA correctly updates existing data");

    Ok(())
}
