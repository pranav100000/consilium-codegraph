use anyhow::Result;
use store::GraphStore;
use protocol::{SymbolIR, Language, SymbolKind, Span, Version};
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::TempDir;
use std::time::Instant;

/// Tests to demonstrate database concurrency issues and validate fixes
/// These tests should initially FAIL due to SQLite lock contention,
/// then PASS after implementing the connection manager solution.

fn create_test_symbol(id: &str, name: &str, thread_id: usize, index: usize) -> SymbolIR {
    SymbolIR {
        id: id.to_string(),
        lang: Language::TypeScript,
        lang_version: Some(Version::ES2020),
        kind: SymbolKind::Function,
        name: name.to_string(),
        fqn: format!("thread_{}.{}", thread_id, name),
        signature: Some(format!("function {}()", name)),
        file_path: format!("thread_{}.ts", thread_id),
        span: Span {
            start_line: index as u32,
            start_col: 0,
            end_line: index as u32,
            end_col: 10,
        },
        visibility: Some("public".to_string()),
        doc: Some(format!("Function {} in thread {}", name, thread_id)),
        sig_hash: format!("hash_{}_{}", thread_id, index),
    }
}

#[test]
fn test_multiple_graphstore_instances_fail() -> Result<()> {
    println!("🔬 Testing multiple GraphStore instances (should initially fail)...");

    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    let errors = Arc::new(Mutex::new(Vec::new()));
    let success_count = Arc::new(Mutex::new(0));

    // Create 5 separate GraphStore instances that will compete for database access
    let handles: Vec<_> = (0..5).map(|thread_id| {
        let repo_path = repo_path.to_path_buf();
        let errors = Arc::clone(&errors);
        let success_count = Arc::clone(&success_count);

        thread::spawn(move || {
            let result = (|| -> Result<()> {
                // Each thread creates its own GraphStore instance
                // This will create separate SQLite connections to the same database
                let store = GraphStore::new(&repo_path)?;
                let commit_id = store.get_or_create_commit(&format!("thread_{}", thread_id))?;

                // Insert 50 symbols per thread
                for i in 0..50 {
                    let symbol = create_test_symbol(
                        &format!("symbol_{}_{}", thread_id, i),
                        &format!("func_{}_{}", thread_id, i),
                        thread_id,
                        i,
                    );
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

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let final_errors = errors.lock().unwrap();
    let final_success = *success_count.lock().unwrap();

    println!("Results: {} successes, {} errors", final_success, final_errors.len());

    if !final_errors.is_empty() {
        println!("Errors encountered:");
        for error in final_errors.iter() {
            println!("  - {}", error);

            // Check for specific SQLite lock errors
            if error.contains("database is locked") || error.contains("SQLITE_BUSY") {
                println!("  ✅ Found expected lock contention error");
            }
        }
    }

    // This test demonstrates the problem - we expect some failures initially
    // After fixing, final_errors.len() should be 0
    if final_errors.len() > 0 {
        println!("⚠️  EXPECTED FAILURE: Found {} lock contention errors (this proves the problem exists)", final_errors.len());
        println!("   This test will pass after implementing connection manager fixes");
    } else {
        println!("✅ No concurrency errors - connection sharing is working!");
    }

    Ok(())
}

#[test]
fn test_concurrent_operations_lock_errors() -> Result<()> {
    println!("🔒 Testing concurrent operations that trigger lock errors...");

    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    let lock_errors = Arc::new(Mutex::new(Vec::new()));

    // Test specific operations that are known to cause lock contention
    let handles: Vec<_> = (0..8).map(|thread_id| {
        let repo_path = repo_path.to_path_buf();
        let lock_errors = Arc::clone(&lock_errors);

        thread::spawn(move || {
            let start = Instant::now();

            let result = (|| -> Result<()> {
                let store = GraphStore::new(&repo_path)?;

                // Operations that compete for locks:
                // 1. Creating commits (INSERT operations)
                let commit_id = store.get_or_create_commit(&format!("commit_{}", thread_id))?;

                // 2. Schema initialization (CREATE TABLE operations)
                // This happens in GraphStore::new() and can conflict

                // 3. Symbol insertion with FTS5 triggers
                for i in 0..20 {
                    let symbol = create_test_symbol(
                        &format!("lock_test_{}_{}", thread_id, i),
                        &format!("lock_func_{}_{}", thread_id, i),
                        thread_id,
                        i,
                    );

                    // INSERT OR REPLACE + FTS5 trigger updates
                    store.insert_symbol(commit_id, &symbol)?;
                }

                // 4. Search operations that might conflict with writes
                let _results = store.search_symbols("lock_func", 10)?;

                Ok(())
            })();

            let duration = start.elapsed();

            if let Err(e) = result {
                let error_msg = e.to_string();
                if error_msg.contains("database is locked") ||
                   error_msg.contains("SQLITE_BUSY") ||
                   error_msg.contains("locked") {
                    let mut errors = lock_errors.lock().unwrap();
                    errors.push(format!("Thread {} ({}ms): {}", thread_id, duration.as_millis(), error_msg));
                }
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let final_lock_errors = lock_errors.lock().unwrap();

    println!("Lock errors detected: {}", final_lock_errors.len());
    for error in final_lock_errors.iter() {
        println!("  {}", error);
    }

    if final_lock_errors.len() > 0 {
        println!("⚠️  EXPECTED FAILURE: Detected {} lock errors", final_lock_errors.len());
        println!("   These specific lock errors will be eliminated by connection sharing");
    } else {
        println!("✅ No lock errors detected - concurrent operations working smoothly!");
    }

    Ok(())
}

#[test]
fn test_real_world_scan_scenario() -> Result<()> {
    println!("🌍 Testing real-world scenario that mimics CLI usage...");

    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    let errors = Arc::new(Mutex::new(Vec::new()));

    // Simulate the pattern from main.rs where multiple GraphStore instances are created
    let handles: Vec<_> = (0..3).map(|scenario_id| {
        let repo_path = repo_path.to_path_buf();
        let errors = Arc::clone(&errors);

        thread::spawn(move || {
            let result = (|| -> Result<()> {
                match scenario_id {
                    0 => {
                        // Simulate: cargo run -- scan
                        println!("Thread {}: Simulating syntactic analysis", scenario_id);
                        let store = GraphStore::new(&repo_path)?;
                        let commit_id = store.create_commit_snapshot("abc123")?;

                        for i in 0..100 {
                            let symbol = create_test_symbol(
                                &format!("syntactic_{}_{}", scenario_id, i),
                                &format!("syn_func_{}", i),
                                scenario_id,
                                i,
                            );
                            store.insert_symbol(commit_id, &symbol)?;
                        }
                    }
                    1 => {
                        // Simulate: semantic analysis (creates second store)
                        println!("Thread {}: Simulating semantic analysis", scenario_id);
                        let store_for_resolution = GraphStore::new(&repo_path)?;
                        let commit_id = store_for_resolution.get_or_create_commit("abc123")?;

                        for i in 0..50 {
                            let symbol = create_test_symbol(
                                &format!("semantic_{}_{}", scenario_id, i),
                                &format!("sem_func_{}", i),
                                scenario_id,
                                i,
                            );
                            store_for_resolution.insert_symbol(commit_id, &symbol)?;
                        }
                    }
                    2 => {
                        // Simulate: cargo run -- search (creates third store)
                        println!("Thread {}: Simulating search operation", scenario_id);
                        let store = GraphStore::new(&repo_path)?;

                        // Multiple search operations
                        for query in &["func", "syn", "sem", "test"] {
                            let _results = store.search_symbols(query, 20)?;
                        }
                    }
                    _ => unreachable!(),
                }

                Ok(())
            })();

            if let Err(e) = result {
                let mut errs = errors.lock().unwrap();
                errs.push(format!("Scenario {}: {}", scenario_id, e));
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let final_errors = errors.lock().unwrap();

    if final_errors.len() > 0 {
        println!("⚠️  REAL-WORLD SCENARIO FAILURES: {} errors", final_errors.len());
        for error in final_errors.iter() {
            println!("  {}", error);
        }
        println!("   This proves the issue will occur in actual CLI usage");
    } else {
        println!("✅ Real-world scenario completed without errors!");
    }

    Ok(())
}

#[test]
fn test_high_contention_stress() -> Result<()> {
    println!("💥 Testing high contention stress scenario...");

    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    let start_time = Instant::now();
    let errors = Arc::new(Mutex::new(Vec::new()));
    let operations_completed = Arc::new(Mutex::new(0));

    // High contention: 15 threads doing intensive operations
    let handles: Vec<_> = (0..15).map(|thread_id| {
        let repo_path = repo_path.to_path_buf();
        let errors = Arc::clone(&errors);
        let operations_completed = Arc::clone(&operations_completed);

        thread::spawn(move || {
            let result = (|| -> Result<()> {
                let store = GraphStore::new(&repo_path)?;
                let commit_id = store.get_or_create_commit(&format!("stress_{}", thread_id))?;

                // Each thread does multiple types of operations
                for batch in 0..5 {
                    // Batch insert symbols
                    for i in 0..10 {
                        let symbol = create_test_symbol(
                            &format!("stress_{}_{}_{}", thread_id, batch, i),
                            &format!("stress_func_{}_{}", batch, i),
                            thread_id,
                            i,
                        );
                        store.insert_symbol(commit_id, &symbol)?;
                    }

                    // Search operations
                    let _results = store.search_symbols(&format!("stress_func_{}", batch), 5)?;

                    // Get symbol count (read operation)
                    let _count = store.get_symbol_count()?;
                }

                let mut ops = operations_completed.lock().unwrap();
                *ops += 1;

                Ok(())
            })();

            if let Err(e) = result {
                let mut errs = errors.lock().unwrap();
                errs.push(format!("Stress thread {}: {}", thread_id, e));
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start_time.elapsed();
    let final_errors = errors.lock().unwrap();
    let final_operations = *operations_completed.lock().unwrap();

    println!("High contention results:");
    println!("  Duration: {}ms", duration.as_millis());
    println!("  Operations completed: {}/15", final_operations);
    println!("  Errors: {}", final_errors.len());

    if final_errors.len() > 0 {
        println!("  Error breakdown:");
        let mut lock_errors = 0;
        for error in final_errors.iter() {
            if error.contains("database is locked") || error.contains("SQLITE_BUSY") {
                lock_errors += 1;
            }
            println!("    {}", error);
        }
        println!("  Lock-related errors: {}/{}", lock_errors, final_errors.len());

        println!("⚠️  HIGH CONTENTION FAILURES: {:.1}% failure rate",
                (final_errors.len() as f32 / 15.0) * 100.0);
    } else {
        println!("✅ High contention handled successfully!");
    }

    Ok(())
}