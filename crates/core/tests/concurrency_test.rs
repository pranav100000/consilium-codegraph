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

    // CRITICAL ASSERTIONS: Test must fail if there are concurrency errors
    assert_eq!(final_errors.len(), 0,
        "Concurrency test failed: {} threads encountered errors. \
         Connection sharing should eliminate all lock contention. \
         Errors: {:?}",
        final_errors.len(),
        final_errors
    );

    assert_eq!(final_success, 5,
        "Expected all 5 threads to succeed, but only {} succeeded",
        final_success
    );

    // CRITICAL: Verify data integrity - all symbols should be inserted
    let store = GraphStore::new(&repo_path)?;
    let total_symbols = store.get_symbol_count()?;

    assert_eq!(total_symbols, 250,
        "Expected 250 symbols (5 threads × 50 symbols), but found {}. \
         This indicates data loss or duplicate overwrites.",
        total_symbols
    );

    // Verify each thread's data is present
    for thread_id in 0..5 {
        let thread_symbols = store.search_symbols(&format!("func_{}", thread_id), 100)?;
        assert_eq!(thread_symbols.len(), 50,
            "Thread {} should have inserted 50 symbols, but found {}",
            thread_id,
            thread_symbols.len()
        );
    }

    println!("✅ All concurrency assertions passed: 0 errors, 250 symbols inserted correctly");

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

    // CRITICAL ASSERTION: No lock errors should occur with connection sharing
    assert_eq!(final_lock_errors.len(), 0,
        "Lock errors detected during concurrent operations: {} errors. \
         Connection sharing should eliminate all lock contention. \
         Errors: {:?}",
        final_lock_errors.len(),
        final_lock_errors
    );

    // Verify data integrity - all operations completed successfully
    let store = GraphStore::new(&repo_path)?;
    let symbol_count = store.get_symbol_count()?;

    // 8 threads × 20 symbols = 160 expected
    assert!(symbol_count >= 160,
        "Expected at least 160 symbols from 8 threads, but found {}",
        symbol_count
    );

    println!("✅ Concurrent operations test passed: 0 lock errors, {} symbols inserted", symbol_count);

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

    // CRITICAL ASSERTION: Real-world usage should not produce errors
    assert_eq!(final_errors.len(), 0,
        "Real-world scenario test failed: {} errors occurred during simulated CLI usage. \
         Errors: {:?}",
        final_errors.len(),
        final_errors
    );

    // Verify all operations completed successfully
    let store = GraphStore::new(&repo_path)?;
    let symbol_count = store.get_symbol_count()?;

    // Scenario 0: 100 symbols, Scenario 1: 50 symbols
    assert!(symbol_count >= 150,
        "Expected at least 150 symbols from real-world scenario, but found {}",
        symbol_count
    );

    println!("✅ Real-world scenario test passed: {} symbols indexed without errors", symbol_count);

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

    // CRITICAL ASSERTION: High contention should not cause failures
    assert_eq!(final_errors.len(), 0,
        "High contention stress test failed: {} threads encountered errors ({:.1}% failure rate). \
         Connection sharing should handle high contention. \
         Errors: {:?}",
        final_errors.len(),
        (final_errors.len() as f32 / 15.0) * 100.0,
        final_errors
    );

    assert_eq!(final_operations, 15,
        "Expected all 15 threads to complete, but only {} completed",
        final_operations
    );

    // Verify data integrity under high contention
    let store = GraphStore::new(&repo_path)?;
    let symbol_count = store.get_symbol_count()?;

    // 15 threads × 5 batches × 10 symbols = 750 expected
    assert_eq!(symbol_count, 750,
        "Expected 750 symbols (15 threads × 5 batches × 10 symbols), but found {}. \
         High contention caused data loss or corruption.",
        symbol_count
    );

    println!("✅ High contention stress test passed: 15 threads, 750 symbols, 0 errors in {}ms",
        duration.as_millis());

    Ok(())
}