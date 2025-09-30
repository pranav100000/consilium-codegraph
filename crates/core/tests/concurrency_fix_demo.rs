/// Demonstrates the concurrency fix working
/// This test should PASS by using shared connections

use anyhow::Result;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, LazyLock};
use std::thread;
use tempfile::TempDir;

/// Simple shared connection manager for demonstration
static DEMO_CONNECTIONS: LazyLock<Arc<Mutex<HashMap<PathBuf, Arc<Mutex<Connection>>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

fn get_demo_connection(db_path: &Path) -> Result<Arc<Mutex<Connection>>> {
    let mut connections = DEMO_CONNECTIONS.lock().unwrap();

    if let Some(conn) = connections.get(db_path) {
        return Ok(Arc::clone(conn));
    }

    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "busy_timeout", "5000")?;

    let shared_conn = Arc::new(Mutex::new(conn));
    connections.insert(db_path.to_path_buf(), Arc::clone(&shared_conn));

    Ok(shared_conn)
}

#[test]
fn test_shared_connections_eliminate_locks() -> Result<()> {
    println!("🔧 Testing shared connections (should eliminate lock errors)...");

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("shared.db");

    let errors = Arc::new(Mutex::new(Vec::new()));
    let success_count = Arc::new(Mutex::new(0));

    // Create table once
    {
        let conn = get_demo_connection(&db_path)?;
        let connection = conn.lock().unwrap();
        connection.execute(
            "CREATE TABLE test_symbols (id TEXT PRIMARY KEY, name TEXT, data TEXT)",
            [],
        )?;
    }

    // Same test as before, but using shared connections
    let handles: Vec<_> = (0..5).map(|thread_id| {
        let db_path = db_path.clone();
        let errors = Arc::clone(&errors);
        let success_count = Arc::clone(&success_count);

        thread::spawn(move || {
            let result = (|| -> Result<()> {
                // Get shared connection instead of creating new one
                let conn = get_demo_connection(&db_path)?;

                // Insert 50 symbols per thread (same as failing test)
                for i in 0..50 {
                    let connection = conn.lock().unwrap();
                    connection.execute(
                        "INSERT OR REPLACE INTO test_symbols (id, name, data) VALUES (?1, ?2, ?3)",
                        params![
                            format!("symbol_{}_{}", thread_id, i),
                            format!("func_{}_{}", thread_id, i),
                            format!("data_{}_{}", thread_id, i)
                        ],
                    )?;
                    // Drop the lock quickly
                    drop(connection);
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

    println!("Shared connection results: {} successes, {} errors", final_success, final_errors.len());

    if !final_errors.is_empty() {
        println!("Unexpected errors with shared connections:");
        for error in final_errors.iter() {
            println!("  - {}", error);
        }
    }

    // Verify all data was inserted
    let conn = get_demo_connection(&db_path)?;
    let connection = conn.lock().unwrap();
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM test_symbols",
        [],
        |row| row.get(0),
    )?;

    println!("Total symbols inserted: {}", count);
    println!("Expected: 250 (5 threads × 50 symbols each)");

    // With shared connections, we should have:
    // 1. Zero lock errors (all operations succeed)
    // 2. All 250 symbols inserted successfully
    assert_eq!(final_success, 5, "All threads should succeed with shared connections");
    assert_eq!(final_errors.len(), 0, "No lock errors should occur with shared connections");
    assert_eq!(count, 250, "All symbols should be inserted");

    println!("✅ Shared connections eliminated all lock errors!");
    Ok(())
}

#[test]
fn test_connection_reuse_proof() -> Result<()> {
    println!("🔗 Testing connection reuse...");

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("reuse.db");

    // Get connection twice
    let conn1 = get_demo_connection(&db_path)?;
    let conn2 = get_demo_connection(&db_path)?;

    // They should be the same Arc (shared)
    assert!(Arc::ptr_eq(&conn1, &conn2), "Connections should be shared");

    println!("✅ Connection reuse verified!");
    Ok(())
}