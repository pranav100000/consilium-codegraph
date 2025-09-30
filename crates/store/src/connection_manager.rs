use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, LazyLock};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Global connection manager that ensures only one connection per database file
/// This solves SQLite's single-writer limitation by sharing connections across GraphStore instances
static CONNECTION_MANAGER: LazyLock<Arc<Mutex<ConnectionRegistry>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(ConnectionRegistry::new()))
});

/// Registry that maps database file paths to shared connections
struct ConnectionRegistry {
    connections: HashMap<PathBuf, Arc<Mutex<Connection>>>,
}

impl ConnectionRegistry {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Get or create a shared connection for the given database path
    fn get_or_create_connection(&mut self, db_path: &Path) -> Result<Arc<Mutex<Connection>>> {
        // Check if we already have a connection for this database
        if let Some(conn) = self.connections.get(db_path) {
            debug!("Reusing existing connection for {:?}", db_path);
            return Ok(Arc::clone(conn));
        }

        // Create new connection with optimized settings
        info!("Creating new database connection for {:?}", db_path);
        let conn = Connection::open(db_path)?;

        // Configure SQLite for optimal concurrency and performance
        configure_connection(&conn)?;

        let shared_conn = Arc::new(Mutex::new(conn));
        self.connections.insert(db_path.to_path_buf(), Arc::clone(&shared_conn));

        Ok(shared_conn)
    }

    /// Clean up closed connections (for future use)
    #[allow(dead_code)]
    fn cleanup_dead_connections(&mut self) {
        // Remove connections that are no longer referenced
        self.connections.retain(|path, conn| {
            if Arc::strong_count(conn) == 1 {
                debug!("Cleaning up unused connection for {:?}", path);
                false
            } else {
                true
            }
        });
    }
}

/// Configure a SQLite connection for optimal performance and concurrency
fn configure_connection(conn: &Connection) -> Result<()> {
    // Enable WAL mode for better concurrency (allows multiple readers + single writer)
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Enable foreign key constraints
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Set busy timeout to handle lock contention gracefully
    conn.pragma_update(None, "busy_timeout", "5000")?; // 5 seconds

    // Optimize for faster writes
    conn.pragma_update(None, "synchronous", "NORMAL")?; // Faster than FULL, safer than OFF

    // Increase cache size for better performance
    conn.pragma_update(None, "cache_size", "-64000")?; // 64MB cache

    // Enable memory-mapped I/O for better performance on large databases
    conn.pragma_update(None, "mmap_size", "268435456")?; // 256MB mmap

    debug!("Database connection configured with optimized settings");
    Ok(())
}

/// Public interface for getting shared database connections
pub fn get_shared_connection(db_path: &Path) -> Result<Arc<Mutex<Connection>>> {
    let mut manager = CONNECTION_MANAGER.lock().map_err(|e| {
        anyhow::anyhow!("Failed to acquire connection manager lock: {}", e)
    })?;

    manager.get_or_create_connection(db_path)
}

/// Execute a database operation with automatic retry logic for lock contention
pub fn execute_with_retry<T, F>(
    conn: &Arc<Mutex<Connection>>,
    operation: F,
    max_retries: u32,
) -> Result<T>
where
    F: Fn(&Connection) -> Result<T>,
{
    let mut attempt = 0;
    let mut last_error = None;

    while attempt <= max_retries {
        // Acquire connection lock
        let connection = conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire connection lock: {}", e)
        })?;

        // Try the operation
        match operation(&*connection) {
            Ok(result) => return Ok(result),
            Err(e) => {
                let error_msg = e.to_string();

                // Check if this is a retryable error
                if is_retryable_error(&error_msg) && attempt < max_retries {
                    warn!("Database operation failed (attempt {}), retrying: {}", attempt + 1, error_msg);

                    // Drop the connection lock before sleeping
                    drop(connection);

                    // Exponential backoff: 10ms, 20ms, 40ms, 80ms, 160ms
                    let delay_ms = 10 * (1 << attempt);
                    std::thread::sleep(Duration::from_millis(delay_ms));

                    attempt += 1;
                    last_error = Some(e);
                } else {
                    // Not retryable or max retries reached
                    return Err(e);
                }
            }
        }
    }

    // All retries exhausted
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Operation failed after {} retries", max_retries)))
}

/// Check if an error is retryable (lock contention related)
fn is_retryable_error(error_msg: &str) -> bool {
    error_msg.contains("database is locked") ||
    error_msg.contains("SQLITE_BUSY") ||
    error_msg.contains("database table is locked") ||
    error_msg.contains("cannot start a transaction within a transaction")
}

/// Batch multiple operations into a single transaction for better performance
pub fn execute_batch_transaction<T, F>(
    conn: &Arc<Mutex<Connection>>,
    operations: Vec<F>,
) -> Result<Vec<T>>
where
    F: Fn(&Connection) -> Result<T>,
{
    let connection = conn.lock().map_err(|e| {
        anyhow::anyhow!("Failed to acquire connection lock: {}", e)
    })?;

    // Start transaction
    let tx = connection.unchecked_transaction()?;
    let mut results = Vec::with_capacity(operations.len());

    // Execute all operations in the transaction
    for operation in operations {
        let result = operation(&tx)?;
        results.push(result);
    }

    // Commit transaction
    tx.commit()?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::thread;

    #[test]
    fn test_connection_sharing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        // Get two connections to the same database
        let conn1 = get_shared_connection(&db_path)?;
        let conn2 = get_shared_connection(&db_path)?;

        // They should be the same Arc (shared)
        assert!(Arc::ptr_eq(&conn1, &conn2));

        Ok(())
    }

    #[test]
    fn test_concurrent_connection_access() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("concurrent.db");

        let handles: Vec<_> = (0..5).map(|thread_id| {
            let db_path = db_path.clone();
            thread::spawn(move || {
                let conn = get_shared_connection(&db_path).unwrap();

                // Execute a simple operation
                execute_with_retry(&conn, |connection| {
                    connection.execute(
                        "CREATE TABLE IF NOT EXISTS test (id INTEGER, value TEXT)",
                        [],
                    )?;

                    connection.execute(
                        "INSERT INTO test (id, value) VALUES (?1, ?2)",
                        rusqlite::params![thread_id, format!("value_{}", thread_id)],
                    )?;

                    Ok(())
                }, 3)
            })
        }).collect();

        // All threads should complete successfully
        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        // Verify all data was inserted
        let conn = get_shared_connection(&db_path)?;
        let count: i64 = execute_with_retry(&conn, |connection| {
            let mut stmt = connection.prepare("SELECT COUNT(*) FROM test")?;
            let count = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }, 3)?;

        assert_eq!(count, 5);
        Ok(())
    }

    #[test]
    fn test_retry_logic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("retry.db");
        let conn = get_shared_connection(&db_path)?;

        // This should succeed even if there are temporary lock issues
        let result = execute_with_retry(&conn, |connection| {
            connection.execute("CREATE TABLE test (id INTEGER)", [])?;
            Ok(42)
        }, 3)?;

        assert_eq!(result, 42);
        Ok(())
    }

    #[test]
    fn test_batch_transaction() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("batch.db");
        let conn = get_shared_connection(&db_path)?;

        // Create table first
        execute_with_retry(&conn, |connection| {
            connection.execute("CREATE TABLE test (id INTEGER, value TEXT)", [])?;
            Ok(())
        }, 3)?;

        // Batch insert operations
        let operations = (0..10).map(|i| {
            move |connection: &Connection| -> Result<()> {
                connection.execute(
                    "INSERT INTO test (id, value) VALUES (?1, ?2)",
                    rusqlite::params![i, format!("batch_value_{}", i)],
                )?;
                Ok(())
            }
        }).collect();

        let results = execute_batch_transaction(&conn, operations)?;
        assert_eq!(results.len(), 10);

        // Verify all data was inserted in single transaction
        let count: i64 = execute_with_retry(&conn, |connection| {
            let mut stmt = connection.prepare("SELECT COUNT(*) FROM test")?;
            let count = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }, 3)?;

        assert_eq!(count, 10);
        Ok(())
    }
}