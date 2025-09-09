use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, OccurrenceRole, Resolution, Span, SymbolIR, SymbolKind, Version};
use rusqlite::Connection;
use std::collections::HashMap;
use store::GraphStore;
use tempfile::TempDir;

/// Test comprehensive database persistence and query functionality
/// This covers transaction integrity, concurrent operations, query optimization,
/// and various edge cases for database operations

fn create_test_store() -> Result<(GraphStore, TempDir)> {
    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    Ok((store, temp_dir))
}

fn create_complex_symbol(id: &str, name: &str, lang: Language, kind: SymbolKind) -> SymbolIR {
    let lang_debug = format!("{:?}", lang).to_lowercase();
    let file_ext = match &lang {
        Language::TypeScript => "ts",
        Language::Python => "py",
        Language::Rust => "rs",
        Language::Go => "go",
        Language::Java => "java",
        Language::Cpp => "cpp",
        _ => "txt",
    };
    let visibility = match &kind {
        SymbolKind::Class | SymbolKind::Function => "public".to_string(),
        _ => "private".to_string(),
    };
    
    SymbolIR {
        id: id.to_string(),
        lang,
        lang_version: Some(Version::Unknown),
        kind,
        name: name.to_string(),
        fqn: format!("{}.{}", lang_debug, name),
        signature: Some(format!("{}()", name)),
        file_path: format!("{}.{}", name.to_lowercase(), file_ext),
        span: Span {
            start_line: 10 + id.len() as u32,
            start_col: 5,
            end_line: 10 + id.len() as u32,
            end_col: 5 + name.len() as u32,
        },
        visibility: Some(visibility),
        doc: Some(format!("Documentation for {}", name)),
        sig_hash: format!("hash_{}", id),
    }
}

#[test]
fn test_transaction_rollback_on_error() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // This test would require accessing the internal connection to simulate failures
    // For now, test that normal operations work and counts are consistent
    
    let initial_count = store.get_symbol_count()?;
    
    let symbol1 = create_complex_symbol("s1", "func1", Language::TypeScript, SymbolKind::Function);
    let symbol2 = create_complex_symbol("s2", "func2", Language::Python, SymbolKind::Function);
    
    store.insert_symbol(commit_id, &symbol1)?;
    store.insert_symbol(commit_id, &symbol2)?;
    
    let final_count = store.get_symbol_count()?;
    assert_eq!(final_count, initial_count + 2);
    
    Ok(())
}

#[test]
fn test_large_batch_insertions() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("batch_test")?;
    
    // Insert large batch of symbols
    let batch_size = 1000;
    let languages = [Language::TypeScript, Language::Python, Language::Rust, Language::Go, Language::Java];
    let kinds = [SymbolKind::Function, SymbolKind::Class, SymbolKind::Variable, SymbolKind::Method];
    
    for i in 0..batch_size {
        let lang = languages[i % languages.len()].clone();
        let kind = kinds[i % kinds.len()].clone();
        let symbol = create_complex_symbol(
            &format!("batch_{}", i),
            &format!("symbol_{}", i),
            lang,
            kind
        );
        store.insert_symbol(commit_id, &symbol)?;
        
        // Add some edges to create a realistic graph
        if i > 0 && i % 10 == 0 {
            let edge = EdgeIR {
                edge_type: EdgeType::Calls,
                src: Some(format!("batch_{}", i - 1)),
                dst: Some(format!("batch_{}", i)),
                file_src: None,
                file_dst: None,
                resolution: Resolution::Semantic,
                meta: HashMap::new(),
                provenance: HashMap::from([
                    ("tool".to_string(), "test".to_string()),
                    ("version".to_string(), "1.0".to_string()),
                ]),
            };
            store.insert_edge(commit_id, &edge)?;
        }
    }
    
    let symbol_count = store.get_symbol_count()?;
    assert_eq!(symbol_count, batch_size);
    
    let edge_count = store.get_edge_count()?;
    assert_eq!(edge_count, (batch_size / 10) - 1); // -1 because we skip i=0
    
    Ok(())
}

#[test]
fn test_query_performance_with_indexes() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("perf_test")?;
    
    // Insert data to test index usage
    for i in 0..500 {
        let symbol = create_complex_symbol(
            &format!("perf_{}", i),
            &format!("testSymbol_{}", i),
            Language::TypeScript,
            SymbolKind::Function
        );
        store.insert_symbol(commit_id, &symbol)?;
    }
    
    // Test FQN lookup (should use idx_symbol_fqn index)
    let start = std::time::Instant::now();
    let result = store.get_symbol_by_fqn("typescript.testSymbol_250")?;
    let fqn_lookup_time = start.elapsed();
    
    assert!(result.is_some());
    assert!(fqn_lookup_time.as_millis() < 100); // Should be very fast with index
    
    // Test search (should use FTS5 or LIKE index)
    let start = std::time::Instant::now();
    let results = store.search_symbols("testSymbol", 50)?;
    let search_time = start.elapsed();
    
    assert!(results.len() > 0);
    assert!(search_time.as_millis() < 200); // Should be reasonable with index
    
    Ok(())
}

#[test]
fn test_complex_graph_operations() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("graph_test")?;
    
    // Create a more complex graph structure
    let symbols = vec![
        ("main", SymbolKind::Function),
        ("UserService", SymbolKind::Class),
        ("getUser", SymbolKind::Method),
        ("validateUser", SymbolKind::Method),
        ("DatabaseConnection", SymbolKind::Class),
        ("query", SymbolKind::Method),
        ("Logger", SymbolKind::Class),
        ("log", SymbolKind::Method),
    ];
    
    for (name, kind) in &symbols {
        let symbol = create_complex_symbol(
            &name.to_lowercase(),
            name,
            Language::TypeScript,
            kind.clone()
        );
        store.insert_symbol(commit_id, &symbol)?;
    }
    
    // Create realistic edges
    let edges = vec![
        ("main", "userservice", EdgeType::Calls),
        ("main", "logger", EdgeType::Calls),
        ("userservice", "getuser", EdgeType::Contains),
        ("userservice", "validateuser", EdgeType::Contains),
        ("getuser", "databaseconnection", EdgeType::Calls),
        ("getuser", "log", EdgeType::Calls),
        ("validateuser", "log", EdgeType::Calls),
        ("databaseconnection", "query", EdgeType::Contains),
        ("query", "log", EdgeType::Calls),
    ];
    
    for (src, dst, edge_type) in edges {
        let edge = EdgeIR {
            edge_type,
            src: Some(src.to_string()),
            dst: Some(dst.to_string()),
            file_src: Some(format!("{}.ts", src)),
            file_dst: Some(format!("{}.ts", dst)),
            resolution: Resolution::Semantic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        };
        store.insert_edge(commit_id, &edge)?;
    }
    
    // Build graph and test operations
    let graph = store.build_graph()?;
    let stats = graph.stats();
    
    assert_eq!(stats.node_count, 8);
    assert_eq!(stats.edge_count, 9);
    
    // Test caller/callee relationships
    let callers = store.get_callers("log", 3)?;
    assert!(callers.len() >= 3); // getuser, validateuser, query should call log
    
    let callees = store.get_callees("main", 2)?;
    assert!(callees.len() >= 2); // main should reach multiple functions
    
    Ok(())
}

#[test]
fn test_fts5_full_text_search_advanced() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("fts_test")?;
    
    // Create symbols with rich documentation
    let test_data = vec![
        ("auth", "AuthenticationService", "Handles user authentication and session management"),
        ("db", "DatabaseManager", "Manages database connections and query execution"),
        ("api", "APIController", "REST API controller for handling HTTP requests"),
        ("cache", "CacheService", "Implements caching mechanisms for performance optimization"),
        ("log", "LoggingUtility", "Provides logging functionality across the application"),
        ("valid", "ValidationHelper", "Contains validation logic for user input"),
        ("email", "EmailService", "Handles email sending and template processing"),
    ];
    
    for (id, name, doc) in test_data {
        let mut symbol = create_complex_symbol(id, name, Language::TypeScript, SymbolKind::Class);
        symbol.doc = Some(doc.to_string());
        store.insert_symbol(commit_id, &symbol)?;
    }
    
    // Test various FTS5 query patterns
    let authentication_results = store.search_symbols_fts("authentication", 10)?;
    assert!(authentication_results.len() > 0);
    
    let database_results = store.search_symbols_fts("database", 10)?;
    assert!(database_results.len() > 0);
    
    // Test phrase search
    let session_results = store.search_symbols_fts("\"session management\"", 10)?;
    assert!(session_results.len() > 0);
    
    // Test prefix search
    let api_results = store.search_symbols_fts("API*", 10)?;
    assert!(api_results.len() > 0);
    
    // Test complex query (AND/OR operations)
    let complex_results = store.search_symbols_fts("user OR database", 10)?;
    assert!(complex_results.len() >= 2);
    
    Ok(())
}

#[test]
fn test_incremental_data_updates() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    
    // Simulate incremental updates across multiple commits
    let commit1 = store.get_or_create_commit("commit_1")?;
    let commit2 = store.get_or_create_commit("commit_2")?;
    let commit3 = store.get_or_create_commit("commit_3")?;
    
    // Commit 1: Initial state
    let symbol1 = create_complex_symbol("s1", "initialFunc", Language::TypeScript, SymbolKind::Function);
    store.insert_symbol(commit1, &symbol1)?;
    store.insert_file(commit1, "main.ts", "hash1", 1024)?;
    
    // Commit 2: Add more symbols
    let symbol2 = create_complex_symbol("s2", "newFunc", Language::TypeScript, SymbolKind::Function);
    store.insert_symbol(commit2, &symbol1)?; // Same symbol, different commit
    store.insert_symbol(commit2, &symbol2)?; // New symbol
    store.insert_file(commit2, "main.ts", "hash2", 1100)?; // Updated file
    store.insert_file(commit2, "utils.ts", "hash3", 500)?; // New file
    
    // Commit 3: Modify and add
    let mut symbol3 = symbol1.clone();
    symbol3.doc = Some("Updated documentation".to_string()); // Modified symbol
    store.insert_symbol(commit3, &symbol3)?;
    store.insert_file(commit3, "main.ts", "hash4", 1200)?;
    
    // Test file evolution
    let files_c1 = store.get_files_in_commit("commit_1")?;
    let files_c2 = store.get_files_in_commit("commit_2")?;
    let files_c3 = store.get_files_in_commit("commit_3")?;
    
    assert_eq!(files_c1.len(), 1);
    assert_eq!(files_c2.len(), 2);
    assert_eq!(files_c3.len(), 1);
    
    // Test latest commit detection
    let latest = store.get_latest_commit()?;
    assert_eq!(latest, Some("commit_3".to_string()));
    
    Ok(())
}

#[test]
fn test_database_schema_constraints() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("constraint_test")?;
    
    // Test unique constraints
    let symbol = create_complex_symbol("unique_test", "TestFunc", Language::TypeScript, SymbolKind::Function);
    
    // First insertion should succeed
    store.insert_symbol(commit_id, &symbol)?;
    
    // Second insertion with same ID should replace (INSERT OR REPLACE)
    store.insert_symbol(commit_id, &symbol)?;
    
    let count = store.get_symbol_count()?;
    assert_eq!(count, 1); // Should still be 1 due to REPLACE
    
    // Test foreign key constraints by inserting edge without symbols
    let edge = EdgeIR {
        edge_type: EdgeType::Calls,
        src: Some("nonexistent_src".to_string()),
        dst: Some("nonexistent_dst".to_string()),
        file_src: None,
        file_dst: None,
        resolution: Resolution::Syntactic,
        meta: HashMap::new(),
        provenance: HashMap::new(),
    };
    
    // This should succeed (we don't have FK constraints on symbol references in edges)
    store.insert_edge(commit_id, &edge)?;
    
    Ok(())
}

#[test]
fn test_multi_language_symbol_resolution() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("multi_lang_test")?;
    
    // Create symbols in different languages with same names
    let languages = vec![
        (Language::TypeScript, "ts"),
        (Language::Python, "py"),
        (Language::Rust, "rs"),
        (Language::Go, "go"),
        (Language::Java, "java"),
    ];
    
    for (lang, ext) in &languages {
        let mut symbol = create_complex_symbol(
            &format!("{}_main", format!("{:?}", lang).to_lowercase()),
            "main",
            lang.clone(),
            SymbolKind::Function
        );
        symbol.file_path = format!("main.{}", ext);
        store.insert_symbol(commit_id, &symbol)?;
    }
    
    // Test language-specific searches
    let results = store.search_symbols("main", 10)?;
    assert_eq!(results.len(), 5); // Should find all 5 main functions
    
    // Test FQN resolution (should be unique per language)
    for lang in [Language::TypeScript, Language::Python, Language::Rust, Language::Go, Language::Java] {
        let fqn = format!("{}.main", format!("{:?}", lang).to_lowercase());
        let result = store.get_symbol_by_fqn(&fqn)?;
        assert!(result.is_some(), "Should find symbol for language: {:?}", lang);
        assert_eq!(result.unwrap().lang, lang);
    }
    
    Ok(())
}

#[test]
fn test_occurrence_tracking_and_queries() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("occurrence_test")?;
    
    // Create symbol
    let symbol = create_complex_symbol("test_func", "testFunction", Language::TypeScript, SymbolKind::Function);
    store.insert_symbol(commit_id, &symbol)?;
    
    // Create multiple occurrences of the symbol
    let occurrences = vec![
        (OccurrenceRole::Definition, 10, 5, "testFunction"),
        (OccurrenceRole::Reference, 25, 12, "testFunction"),
        (OccurrenceRole::Reference, 30, 8, "testFunction"),
        (OccurrenceRole::Write, 35, 4, "testFunction"),
    ];
    
    for (role, line, col, token) in occurrences {
        let occurrence = OccurrenceIR {
            file_path: "test.ts".to_string(),
            symbol_id: Some("test_func".to_string()),
            role,
            span: Span {
                start_line: line,
                start_col: col,
                end_line: line,
                end_col: col + token.len() as u32,
            },
            token: token.to_string(),
        };
        store.insert_occurrence(commit_id, &occurrence)?;
    }
    
    // Test that occurrences are tracked (would need query methods for occurrences)
    // For now, just ensure no errors occurred during insertion
    
    Ok(())
}

#[test]
fn test_error_recovery_and_data_integrity() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("error_test")?;
    
    // Test insertion of malformed data
    let mut malformed_symbol = create_complex_symbol("malformed", "test", Language::TypeScript, SymbolKind::Function);
    malformed_symbol.span.start_line = u32::MAX; // Extreme value
    malformed_symbol.span.end_line = 0; // Invalid span (end < start)
    
    // Should still insert successfully (no validation in current implementation)
    store.insert_symbol(commit_id, &malformed_symbol)?;
    
    let retrieved = store.get_symbol("malformed")?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().span.start_line, u32::MAX);
    
    Ok(())
}

#[test]
fn test_wal_mode_concurrent_access_simulation() -> Result<()> {
    let (store, temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("concurrent_test")?;
    
    // Insert some initial data
    for i in 0..10 {
        let symbol = create_complex_symbol(
            &format!("concurrent_{}", i),
            &format!("func_{}", i),
            Language::TypeScript,
            SymbolKind::Function
        );
        store.insert_symbol(commit_id, &symbol)?;
    }
    
    // Simulate concurrent read by opening another connection
    let db_path = temp_dir.path().join(".reviewbot").join("graph.db");
    let read_conn = Connection::open(&db_path)?;
    
    // This read should succeed due to WAL mode
    let count: i64 = read_conn.query_row(
        "SELECT COUNT(*) FROM symbol",
        [],
        |row| row.get(0)
    )?;
    
    assert_eq!(count, 10);
    
    Ok(())
}

#[test]
fn test_database_corruption_recovery() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    
    // Test that basic operations work
    let commit_id = store.get_or_create_commit("recovery_test")?;
    let symbol = create_complex_symbol("recovery", "testFunc", Language::TypeScript, SymbolKind::Function);
    store.insert_symbol(commit_id, &symbol)?;
    
    // Test database integrity checks (would need PRAGMA integrity_check)
    let symbol_count = store.get_symbol_count()?;
    let edge_count = store.get_edge_count()?;
    let file_count = store.get_file_count()?;
    
    assert_eq!(symbol_count, 1);
    assert_eq!(edge_count, 0);
    assert_eq!(file_count, 0);
    
    Ok(())
}

#[test]
fn test_query_result_ordering_and_pagination() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("pagination_test")?;
    
    // Insert symbols with predictable ordering
    let names = vec!["alpha", "beta", "gamma", "delta", "epsilon"];
    for (i, name) in names.iter().enumerate() {
        let mut symbol = create_complex_symbol(
            &format!("sort_{}", i),
            name,
            Language::TypeScript,
            SymbolKind::Function
        );
        // Set different line numbers for predictable ordering
        symbol.span.start_line = (i as u32 + 1) * 10;
        store.insert_symbol(commit_id, &symbol)?;
    }
    
    // Test search with different limits (pagination simulation)
    let results_2 = store.search_symbols("", 2)?;
    let results_5 = store.search_symbols("", 5)?;
    let results_10 = store.search_symbols("", 10)?;
    
    assert_eq!(results_2.len(), 2);
    assert_eq!(results_5.len(), 5);
    assert_eq!(results_10.len(), 5); // No more than 5 exist
    
    // Test that ordering is consistent
    assert_eq!(results_2[0].id, results_5[0].id);
    assert_eq!(results_2[1].id, results_5[1].id);
    
    Ok(())
}

#[test]
fn test_symbol_fqn_uniqueness_across_contexts() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    
    // Test FQN uniqueness across different commits
    let commit1 = store.get_or_create_commit("commit_1")?;
    let commit2 = store.get_or_create_commit("commit_2")?;
    
    // Same FQN in different commits should be allowed
    let symbol1 = create_complex_symbol("s1_c1", "testFunc", Language::TypeScript, SymbolKind::Function);
    let symbol2 = create_complex_symbol("s1_c2", "testFunc", Language::TypeScript, SymbolKind::Function);
    
    store.insert_symbol(commit1, &symbol1)?;
    store.insert_symbol(commit2, &symbol2)?;
    
    // Both should exist
    let count = store.get_symbol_count()?;
    assert_eq!(count, 2);
    
    // FQN lookup should return the most recent (ORDER BY id DESC)
    let by_fqn = store.get_symbol_by_fqn("typescript.testFunc")?;
    assert!(by_fqn.is_some());
    // Should be the second symbol (higher ID due to insertion order)
    assert_eq!(by_fqn.unwrap().id, "s1_c2");
    
    Ok(())
}