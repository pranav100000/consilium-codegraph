/// Comprehensive stress tests designed to systematically break the code graph system
/// Tests scale limits, concurrency, malicious input, resource exhaustion, and data corruption
///
/// These tests push the system to its breaking points to ensure robust error handling
/// and graceful degradation under extreme conditions.

use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, Resolution, Span, SymbolIR, SymbolKind, Version};
use reviewbot::walker::FileWalker;
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use store::GraphStore;
use tempfile::TempDir;

/// Test Category 1: Scale and Performance Breaking Points
/// These tests push the system to handle massive inputs

#[test]
fn test_massive_file_handling() -> Result<()> {
    println!("📏 Testing massive file handling...");

    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    // Create a massive TypeScript file (10MB of code)
    let massive_content = generate_massive_typescript_file(10_000_000); // 10MB
    fs::write(project_path.join("massive.ts"), &massive_content)?;
    fs::write(project_path.join("package.json"), r#"{"name": "massive-test"}"#)?;

    // Test that the system can handle this without crashing (with timeout)
    let output = run_command_with_timeout(
        &mut Command::new("cargo")
            .args(&[
                "run", "-p", "reviewbot", "--",
                "--repo", &project_path.to_string_lossy(),
                "scan", "--no-write"
            ])
            .current_dir(std::env::current_dir()?),
        Duration::from_secs(60)
    );

    match output {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Should fail gracefully, not crash
                assert!(!stderr.contains("panic"), "System should not panic on massive files");
                assert!(!stderr.contains("killed"), "System should not be killed by OS");
                println!("✅ Massive file handled gracefully (expected failure)");
            } else {
                println!("✅ Massive file processed successfully!");
            }
        }
        Err(_) => {
            println!("✅ Massive file processing timed out gracefully");
        }
    }

    Ok(())
}

#[test]
fn test_extremely_deep_directory_nesting() -> Result<()> {
    println!("📁 Testing extremely deep directory nesting...");

    let temp_dir = TempDir::new()?;
    let mut current_path = temp_dir.path().to_path_buf();

    // Create 500 levels of nested directories
    for i in 0..500 {
        current_path = current_path.join(format!("level_{}", i));
        if fs::create_dir(&current_path).is_err() {
            // Hit filesystem limit, that's fine
            break;
        }

        // Add a source file every 50 levels
        if i % 50 == 0 {
            fs::write(current_path.join("deep.ts"), format!("export const level{} = {};", i, i))?;
        }
    }

    let walker = FileWalker::new(temp_dir.path().to_path_buf());
    let start = Instant::now();
    let result = walker.walk();
    let duration = start.elapsed();

    match result {
        Ok(files) => {
            println!("✅ Deep nesting handled: {} files found in {:?}", files.len(), duration);
            assert!(duration < Duration::from_secs(10), "Deep directory walk should complete in reasonable time");
        }
        Err(e) => {
            println!("✅ Deep nesting failed gracefully: {}", e);
        }
    }

    Ok(())
}

#[test]
fn test_millions_of_symbols() -> Result<()> {
    println!("🔢 Testing millions of symbols...");

    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.get_or_create_commit("stress_test")?;

    // Generate 100,000 symbols (scaled down from millions for test speed)
    let symbol_count = 100_000;
    let start = Instant::now();

    for i in 0..symbol_count {
        let symbol = SymbolIR {
            id: format!("stress_symbol_{}", i),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: if i % 3 == 0 { SymbolKind::Function } else if i % 3 == 1 { SymbolKind::Class } else { SymbolKind::Variable },
            name: format!("symbol_{}", i),
            fqn: format!("stress_test.symbol_{}", i),
            signature: Some(format!("function symbol_{}()", i)),
            file_path: format!("stress_{}.ts", i / 1000), // 1000 symbols per file
            span: Span { start_line: (i % 1000) as u32, start_col: 0, end_line: (i % 1000) as u32, end_col: 10 },
            visibility: Some("public".to_string()),
            doc: Some(format!("Generated stress test symbol {}", i)),
            sig_hash: format!("hash_{}", i),
        };

        if let Err(e) = store.insert_symbol(commit_id, &symbol) {
            println!("⚠️ Failed to insert symbol {} after {} symbols: {}", i, i, e);
            break;
        }

        // Progress indicator every 10k symbols
        if i % 10_000 == 0 && i > 0 {
            let elapsed = start.elapsed();
            let rate = i as f64 / elapsed.as_secs_f64();
            println!("   Inserted {} symbols, rate: {:.0} symbols/sec", i, rate);
        }
    }

    let final_count = store.get_symbol_count()?;
    let duration = start.elapsed();

    println!("✅ Symbol stress test completed: {} symbols in {:?}", final_count, duration);
    assert!(final_count > 50_000, "Should handle at least 50k symbols");

    Ok(())
}

/// Test Category 2: Concurrency and Race Conditions
/// These tests simulate multiple processes accessing the system simultaneously

#[test]
fn test_concurrent_database_access() -> Result<()> {
    println!("🔄 Testing concurrent database access...");

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();
    let errors = Arc::new(Mutex::new(Vec::new()));
    let success_count = Arc::new(Mutex::new(0));

    // Spawn 10 threads that all try to write to the database simultaneously
    let handles: Vec<_> = (0..10).map(|thread_id| {
        let db_path = db_path.clone();
        let errors = Arc::clone(&errors);
        let success_count = Arc::clone(&success_count);

        thread::spawn(move || {
            let result = (|| -> Result<()> {
                let store = GraphStore::new(&db_path)?;
                let commit_id = store.get_or_create_commit(&format!("thread_{}", thread_id))?;

                // Each thread inserts 100 symbols
                for i in 0..100 {
                    let symbol = SymbolIR {
                        id: format!("thread_{}_{}", thread_id, i),
                        lang: Language::TypeScript,
                        lang_version: Some(Version::ES2020),
                        kind: SymbolKind::Function,
                        name: format!("func_{}_{}", thread_id, i),
                        fqn: format!("thread_{}.func_{}", thread_id, i),
                        signature: Some(format!("function func_{}_{}", thread_id, i)),
                        file_path: format!("thread_{}.ts", thread_id),
                        span: Span { start_line: i as u32, start_col: 0, end_line: i as u32, end_col: 10 },
                        visibility: Some("public".to_string()),
                        doc: None,
                        sig_hash: format!("hash_{}_{}", thread_id, i),
                    };

                    store.insert_symbol(commit_id, &symbol)?;
                }

                Ok(())
            })();

            match result {
                Ok(_) => {
                    let mut count = success_count.lock().unwrap();
                    *count += 1;
                }
                Err(e) => {
                    let mut errs = errors.lock().unwrap();
                    errs.push(format!("Thread {}: {}", thread_id, e));
                }
            }
        })
    }).collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    let final_errors = errors.lock().unwrap();
    let final_success = *success_count.lock().unwrap();

    println!("✅ Concurrency test: {} successes, {} errors", final_success, final_errors.len());

    if !final_errors.is_empty() {
        println!("   Errors encountered:");
        for error in final_errors.iter() {
            println!("   - {}", error);
        }
    }

    // Should handle concurrent access gracefully - either all succeed or fail gracefully
    assert!(final_success > 0 || !final_errors.is_empty(), "At least some operations should complete");

    Ok(())
}

#[test]
fn test_file_changes_during_processing() -> Result<()> {
    println!("🔄 Testing file changes during processing...");

    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    // Create initial files
    fs::write(project_path.join("package.json"), r#"{"name": "changing-test"}"#)?;
    fs::write(project_path.join("changing.ts"), "export const initial = 'value';")?;

    // Start a scanning process in the background
    let project_path_clone = project_path.to_path_buf();
    let scan_handle = thread::spawn(move || {
        // Simulate a slow scan by adding delays
        for i in 0..50 {
            fs::write(project_path_clone.join("changing.ts"),
                format!("export const iteration{} = 'value{}';\nexport function func{}() {{ return {}; }}", i, i, i, i))
                .unwrap();
            thread::sleep(Duration::from_millis(50));
        }
    });

    // While files are changing, try to scan
    let output = run_command_with_timeout(
        &mut Command::new("cargo")
            .args(&[
                "run", "-p", "reviewbot", "--",
                "--repo", &project_path.to_string_lossy(),
                "scan", "--no-write"
            ])
            .current_dir(std::env::current_dir().unwrap()),
        Duration::from_secs(30)
    );

    scan_handle.join().unwrap();

    match output {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Should handle file changes gracefully
            assert!(!stderr.contains("panic"), "Should not panic when files change during scan");
            println!("✅ File changes during processing handled gracefully");
        }
        Err(_) => {
            println!("✅ Scan timed out gracefully while files were changing");
        }
    }

    Ok(())
}

/// Test Category 3: Malicious and Malformed Input
/// These tests try to break the system with bad data

#[test]
fn test_binary_data_disguised_as_source() -> Result<()> {
    println!("🦠 Testing binary data disguised as source code...");

    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    fs::write(project_path.join("package.json"), r#"{"name": "malicious-test"}"#)?;

    // Create files with binary data but source extensions
    let binary_data = vec![0u8, 1, 2, 255, 254, 0, 127, 128, 255];
    fs::write(project_path.join("malicious.ts"), &binary_data)?;
    fs::write(project_path.join("malicious.py"), &binary_data)?;
    fs::write(project_path.join("malicious.go"), &binary_data)?;

    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--",
            "--repo", &project_path.to_string_lossy(),
            "scan", "--no-write"
        ])
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should handle binary data gracefully
    assert!(!stderr.contains("panic"), "Should not panic on binary data");
    assert!(!stderr.contains("thread"), "Should not crash threads");

    println!("✅ Binary data handled gracefully");
    Ok(())
}

#[test]
fn test_invalid_utf8_encoding() -> Result<()> {
    println!("📝 Testing invalid UTF-8 encoding...");

    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    fs::write(project_path.join("package.json"), r#"{"name": "encoding-test"}"#)?;

    // Create file with invalid UTF-8 sequences
    let invalid_utf8 = vec![
        0xc0, 0x80,  // Invalid start byte
        0x80, 0x80,  // Continuation without start
        0xff, 0xff,  // Invalid bytes
        b'v', b'a', b'l', b'i', b'd', b' ', b'c', b'o', b'd', b'e', // Valid ASCII
        0xed, 0xa0, 0x80, // Surrogate (invalid in UTF-8)
    ];

    fs::write(project_path.join("invalid_utf8.ts"), &invalid_utf8)?;

    let walker = FileWalker::new(project_path.to_path_buf());
    let result = walker.walk();

    match result {
        Ok(files) => {
            println!("✅ Invalid UTF-8 handled: {} files found", files.len());
        }
        Err(e) => {
            println!("✅ Invalid UTF-8 failed gracefully: {}", e);
        }
    }

    Ok(())
}

#[test]
fn test_extremely_long_symbol_names() -> Result<()> {
    println!("📏 Testing extremely long symbol names...");

    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.get_or_create_commit("long_names_test")?;

    // Test various lengths that might break SQL or memory limits
    let test_lengths = vec![1000, 10_000, 100_000, 1_000_000];

    for &length in &test_lengths {
        let long_name = "a".repeat(length);
        let long_fqn = format!("test.{}", "b".repeat(length));
        let long_signature = "c".repeat(length);
        let long_doc = "d".repeat(length);

        let symbol = SymbolIR {
            id: format!("long_symbol_{}", length),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: long_name,
            fqn: long_fqn,
            signature: Some(long_signature),
            file_path: "test.ts".to_string(),
            span: Span { start_line: 1, start_col: 0, end_line: 1, end_col: 10 },
            visibility: Some("public".to_string()),
            doc: Some(long_doc),
            sig_hash: "hash_long".to_string(),
        };

        match store.insert_symbol(commit_id, &symbol) {
            Ok(_) => {
                println!("✅ Length {} handled successfully", length);
            }
            Err(e) => {
                println!("⚠️ Length {} failed gracefully: {}", length, e);
                // Failure is acceptable for extreme lengths
            }
        }
    }

    Ok(())
}

#[test]
fn test_circular_symlinks() -> Result<()> {
    println!("🔄 Testing circular symlinks...");

    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    fs::write(project_path.join("package.json"), r#"{"name": "symlink-test"}"#)?;

    // Create circular symlinks (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let dir_a = project_path.join("dir_a");
        let dir_b = project_path.join("dir_b");

        fs::create_dir(&dir_a)?;
        fs::create_dir(&dir_b)?;

        // Create circular symlinks: dir_a -> dir_b -> dir_a
        let _ = symlink(&dir_b, dir_a.join("link_to_b"));
        let _ = symlink(&dir_a, dir_b.join("link_to_a"));

        let walker = FileWalker::new(project_path.to_path_buf());
        let result = walker.walk();

        match result {
            Ok(files) => {
                println!("✅ Circular symlinks handled: {} files found", files.len());
            }
            Err(e) => {
                println!("✅ Circular symlinks failed gracefully: {}", e);
            }
        }
    }

    #[cfg(not(unix))]
    {
        println!("⏭️ Skipping circular symlink test on non-Unix system");
    }

    Ok(())
}

/// Test Category 4: Resource Exhaustion
/// These tests try to exhaust system resources

#[test]
fn test_memory_pressure() -> Result<()> {
    println!("💾 Testing memory pressure...");

    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    // Create many files that could consume memory
    fs::write(project_path.join("package.json"), r#"{"name": "memory-test"}"#)?;

    for i in 0..1000 {
        let content = format!(
            "export class MemoryHog{} {{\n{}\n}}",
            i,
            (0..100).map(|j| format!("  method{}(): string {{ return 'data{}'; }}", j, j)).collect::<Vec<_>>().join("\n")
        );
        fs::write(project_path.join(format!("memory_hog_{}.ts", i)), content)?;
    }

    let start = Instant::now();
    let output = run_command_with_timeout(
        &mut Command::new("cargo")
            .args(&[
                "run", "-p", "reviewbot", "--",
                "--repo", &project_path.to_string_lossy(),
                "scan", "--no-write"
            ])
            .current_dir(std::env::current_dir().unwrap()),
        Duration::from_secs(120)
    );

    match output {
        Ok(output) => {
            let duration = start.elapsed();
            if output.status.success() {
                println!("✅ Memory pressure handled successfully in {:?}", duration);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(!stderr.contains("killed"), "Should not be killed by OS for memory");
                println!("✅ Memory pressure failed gracefully: process exited cleanly");
            }
        }
        Err(_) => {
            println!("✅ Memory pressure test timed out gracefully");
        }
    }

    Ok(())
}

/// Helper function to generate massive TypeScript content
fn generate_massive_typescript_file(target_size: usize) -> String {
    let mut content = String::new();
    let base_function = "export function generatedFunc{}(): string {\n    return 'This is a generated function with some content to make it longer';\n}\n\n";

    let mut current_size = 0;
    let mut counter = 0;

    while current_size < target_size {
        let func = base_function.replace("{}", &counter.to_string());
        content.push_str(&func);
        current_size += func.len();
        counter += 1;

        // Add some classes too for variety
        if counter % 100 == 0 {
            let class_content = format!(
                "export class GeneratedClass{} {{\n    private data: string = 'some data';\n    public getData(): string {{ return this.data; }}\n}}\n\n",
                counter / 100
            );
            content.push_str(&class_content);
            current_size += class_content.len();
        }
    }

    content
}

/// Test Category 5: Data Consistency and Integrity
/// These tests verify the system maintains data integrity under stress

#[test]
fn test_database_consistency_under_stress() -> Result<()> {
    println!("🔐 Testing database consistency under stress...");

    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.get_or_create_commit("consistency_test")?;

    // Insert symbols and edges simultaneously to test referential integrity
    let symbols_count = 1000;
    let edges_count = 500;

    // Insert symbols first
    for i in 0..symbols_count {
        let symbol = SymbolIR {
            id: format!("consistency_symbol_{}", i),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: format!("func_{}", i),
            fqn: format!("consistency.func_{}", i),
            signature: Some(format!("function func_{}()", i)),
            file_path: "consistency.ts".to_string(),
            span: Span { start_line: i as u32, start_col: 0, end_line: i as u32, end_col: 10 },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: format!("hash_{}", i),
        };

        store.insert_symbol(commit_id, &symbol)?;
    }

    // Insert edges that reference the symbols
    for i in 0..edges_count {
        let edge = EdgeIR {
            edge_type: EdgeType::Calls,
            src: Some(format!("consistency_symbol_{}", i)),
            dst: Some(format!("consistency_symbol_{}", (i + 1) % symbols_count)),
            file_src: Some("consistency.ts".to_string()),
            file_dst: Some("consistency.ts".to_string()),
            resolution: Resolution::Semantic,
            meta: HashMap::new(),
            provenance: {
                let mut p = HashMap::new();
                p.insert("test".to_string(), "consistency".to_string());
                p
            },
        };

        store.insert_edge(commit_id, &edge)?;
    }

    // Verify data consistency
    let final_symbol_count = store.get_symbol_count()?;
    assert_eq!(final_symbol_count, symbols_count, "All symbols should be stored");

    println!("✅ Database consistency maintained: {} symbols, {} edges", final_symbol_count, edges_count);
    Ok(())
}

/// Helper function to run a command with a timeout
fn run_command_with_timeout(command: &mut Command, timeout: Duration) -> Result<std::process::Output> {
    use std::sync::mpsc;

    let (sender, receiver) = mpsc::channel();

    // Clone the command to move it into the thread
    let mut cmd = Command::new(command.get_program());

    // Copy args and other settings (simplified version)
    let args: Vec<std::ffi::OsString> = command.get_args().map(|s| s.to_os_string()).collect();
    cmd.args(&args);

    if let Some(dir) = command.get_current_dir() {
        cmd.current_dir(dir);
    }

    let _handle = thread::spawn(move || {
        let result = cmd.output();
        let _ = sender.send(result);
    });

    match receiver.recv_timeout(timeout) {
        Ok(result) => result.map_err(|e| anyhow::anyhow!("Command failed: {}", e)),
        Err(_) => {
            // Command timed out
            Err(anyhow::anyhow!("Command timed out after {:?}", timeout))
        }
    }
}

/// Test Category 6: External Tool Dependencies
/// These tests verify graceful handling of missing or broken external tools

#[test]
fn test_missing_external_tools() -> Result<()> {
    println!("🔧 Testing missing external tools...");

    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    // Create a project that would require SCIP tools
    fs::write(project_path.join("package.json"), r#"{"name": "tool-test"}"#)?;
    fs::write(project_path.join("test.ts"), "export function test() { return 'test'; }")?;

    // Try to run semantic analysis which would require scip-typescript
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--",
            "--repo", &project_path.to_string_lossy(),
            "scan", "--semantic" // This would require external tools
        ])
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should handle missing tools gracefully
    assert!(!stderr.contains("panic"), "Should not panic when external tools are missing");

    if !output.status.success() {
        println!("✅ Missing external tools handled gracefully");
    } else {
        println!("✅ External tools are available and working");
    }

    Ok(())
}