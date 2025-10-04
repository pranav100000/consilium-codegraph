use store::{GraphStore, CodeGraph};
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, OccurrenceRole, Resolution, SymbolIR, SymbolKind, Span};
use tempfile::TempDir;
use std::collections::HashMap;
use anyhow::Result;

fn create_test_store() -> Result<(GraphStore, TempDir)> {
    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    Ok((store, temp_dir))
}

fn create_test_symbol(id: &str, name: &str) -> SymbolIR {
    SymbolIR {
        id: id.to_string(),
        lang: Language::TypeScript,
        lang_version: None,
        kind: SymbolKind::Function,
        name: name.to_string(),
        fqn: format!("test.{}", name),
        signature: Some(format!("function {}()", name)),
        file_path: "test.ts".to_string(),
        span: Span {
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 10,
        },
        visibility: Some("public".to_string()),
        doc: Some("Test function".to_string()),
        sig_hash: format!("hash_{}", id),
    }
}

#[test]
fn test_store_creation() -> Result<()> {
    let (_store, _temp_dir) = create_test_store()?;
    Ok(())
}

#[test]
fn test_commit_operations() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    
    // Test creating a commit
    let commit_id = store.get_or_create_commit("abc123")?;
    assert!(commit_id > 0);
    
    // Test getting the same commit (should not create new)
    let commit_id2 = store.get_or_create_commit("abc123")?;
    assert_eq!(commit_id, commit_id2);
    
    // Sleep to ensure different timestamp (SQLite timestamps are in seconds)
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Test creating different commit
    let commit_id3 = store.get_or_create_commit("def456")?;
    assert_ne!(commit_id, commit_id3);
    
    // Test getting latest commit
    let latest = store.get_latest_commit()?;
    assert_eq!(latest, Some("def456".to_string()));
    
    Ok(())
}

#[test]
fn test_file_operations() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Insert a file
    store.insert_file(commit_id, "src/main.rs", "hash123", 1024)?;
    
    // Get file hash
    let hash = store.get_file_hash("test_commit", "src/main.rs")?;
    assert_eq!(hash, Some("hash123".to_string()));
    
    // Get files in commit
    let files = store.get_files_in_commit("test_commit")?;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "src/main.rs");
    assert_eq!(files[0].1, "hash123");
    
    // Test non-existent file
    let hash = store.get_file_hash("test_commit", "nonexistent.rs")?;
    assert_eq!(hash, None);
    
    Ok(())
}

#[test]
fn test_symbol_operations() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    let symbol = create_test_symbol("sym1", "testFunc");
    store.insert_symbol(commit_id, &symbol)?;
    
    // Get symbol by ID
    let retrieved = store.get_symbol("sym1")?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "testFunc");
    assert_eq!(retrieved.id, "sym1");
    
    // Get symbol by FQN
    let by_fqn = store.get_symbol_by_fqn("test.testFunc")?;
    assert!(by_fqn.is_some());
    assert_eq!(by_fqn.unwrap().id, "sym1");
    
    // Get symbols in file
    let in_file = store.get_symbols_in_file("test.ts")?;
    assert_eq!(in_file.len(), 1);
    assert_eq!(in_file[0].id, "sym1");
    
    // Test symbol count
    let count = store.get_symbol_count()?;
    assert_eq!(count, 1);
    
    // Test non-existent symbol
    let missing = store.get_symbol("nonexistent")?;
    assert!(missing.is_none());
    
    Ok(())
}

#[test]
fn test_edge_operations() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Insert symbols
    let sym1 = create_test_symbol("sym1", "func1");
    let sym2 = create_test_symbol("sym2", "func2");
    store.insert_symbol(commit_id, &sym1)?;
    store.insert_symbol(commit_id, &sym2)?;
    
    // Insert edge
    let edge = EdgeIR {
        edge_type: EdgeType::Calls,
        src: Some("sym1".to_string()),
        dst: Some("sym2".to_string()),
        file_src: Some("test.ts".to_string()),
        file_dst: Some("test.ts".to_string()),
        resolution: Resolution::Syntactic,
        meta: HashMap::new(),
        provenance: HashMap::new(),
    };
    store.insert_edge(commit_id, &edge)?;
    
    // Get edges for symbol
    let edges = store.get_edges("sym1")?;
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].src, Some("sym1".to_string()));
    assert_eq!(edges[0].dst, Some("sym2".to_string()));
    
    // Test edge count
    let count = store.get_edge_count()?;
    assert_eq!(count, 1);
    
    Ok(())
}

#[test]
fn test_occurrence_operations() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    let occurrence = OccurrenceIR {
        file_path: "test.ts".to_string(),
        symbol_id: Some("sym1".to_string()),
        role: OccurrenceRole::Definition,
        span: Span {
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 10,
        },
        token: "testFunc".to_string(),
    };
    
    store.insert_occurrence(commit_id, &occurrence)?;
    
    // Verify insertion (would need to add a getter method)
    Ok(())
}

#[test]
fn test_search_operations() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Insert various symbols
    let symbols = vec![
        create_test_symbol("s1", "getUserById"),
        create_test_symbol("s2", "setUserName"),
        create_test_symbol("s3", "deleteUser"),
        create_test_symbol("s4", "AdminUser"),
        create_test_symbol("s5", "normalFunction"),
    ];
    
    for sym in &symbols {
        store.insert_symbol(commit_id, sym)?;
    }
    
    // Search for "User"
    let results = store.search_symbols("User", 10)?;
    assert_eq!(results.len(), 4); // Should find getUserById, setUserName, deleteUser, AdminUser
    
    // Search with limit
    let results = store.search_symbols("User", 2)?;
    assert_eq!(results.len(), 2);
    
    // Search for exact match
    let results = store.search_symbols("normalFunction", 10)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "normalFunction");
    
    // Search for non-existent
    let results = store.search_symbols("nonexistent", 10)?;
    assert_eq!(results.len(), 0);
    
    Ok(())
}

#[test]
fn test_fts5_search() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Insert symbols with documentation
    let mut sym1 = create_test_symbol("s1", "processUserData");
    sym1.doc = Some("Process user data and validate inputs".to_string());
    
    let mut sym2 = create_test_symbol("s2", "validateEmail");
    sym2.doc = Some("Validate email format according to RFC".to_string());
    
    let mut sym3 = create_test_symbol("s3", "sendNotification");
    sym3.doc = Some("Send notification to user via email".to_string());
    
    store.insert_symbol(commit_id, &sym1)?;
    store.insert_symbol(commit_id, &sym2)?;
    store.insert_symbol(commit_id, &sym3)?;
    
    // FTS5 search should match on documentation too
    let results = store.search_symbols_fts("validate", 10)?;
    assert_eq!(results.len(), 2); // Should find both processUserData and validateEmail
    
    // Test prefix matching (FTS5 does prefix, not fuzzy)
    let results = store.search_symbols_fts("send*", 10)?;
    assert!(results.len() > 0); // Should find sendNotification
    
    Ok(())
}

#[test]
fn test_clear_file_data() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Insert symbol and occurrence
    let symbol = create_test_symbol("sym1", "testFunc");
    store.insert_symbol(commit_id, &symbol)?;
    
    let occurrence = OccurrenceIR {
        file_path: "test.ts".to_string(),
        symbol_id: Some("sym1".to_string()),
        role: OccurrenceRole::Definition,
        span: Span {
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 10,
        },
        token: "testFunc".to_string(),
    };
    store.insert_occurrence(commit_id, &occurrence)?;
    
    // Clear file data
    store.clear_file_data(commit_id, "test.ts")?;
    
    // Symbol should be gone
    let symbols = store.get_symbols_in_file("test.ts")?;
    assert_eq!(symbols.len(), 0);
    
    Ok(())
}

#[test]
fn test_graph_building() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Create a small graph
    let sym1 = create_test_symbol("s1", "main");
    let sym2 = create_test_symbol("s2", "helper");
    let sym3 = create_test_symbol("s3", "util");
    
    store.insert_symbol(commit_id, &sym1)?;
    store.insert_symbol(commit_id, &sym2)?;
    store.insert_symbol(commit_id, &sym3)?;
    
    // main calls helper
    store.insert_edge(commit_id, &EdgeIR {
        edge_type: EdgeType::Calls,
        src: Some("s1".to_string()),
        dst: Some("s2".to_string()),
        file_src: None,
        file_dst: None,
        resolution: Resolution::Syntactic,
        meta: HashMap::new(),
        provenance: HashMap::new(),
    })?;
    
    // helper calls util
    store.insert_edge(commit_id, &EdgeIR {
        edge_type: EdgeType::Calls,
        src: Some("s2".to_string()),
        dst: Some("s3".to_string()),
        file_src: None,
        file_dst: None,
        resolution: Resolution::Syntactic,
        meta: HashMap::new(),
        provenance: HashMap::new(),
    })?;
    
    let graph = store.build_graph()?;
    let stats = graph.stats();
    
    assert_eq!(stats.node_count, 3);
    assert_eq!(stats.edge_count, 2);
    assert!(!stats.is_cyclic);
    
    Ok(())
}

#[test]
fn test_idempotency() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    let symbol = create_test_symbol("sym1", "testFunc");
    
    // Insert same symbol twice
    store.insert_symbol(commit_id, &symbol)?;
    store.insert_symbol(commit_id, &symbol)?;
    
    // Should only have one symbol
    let count = store.get_symbol_count()?;
    assert_eq!(count, 1);
    
    Ok(())
}

#[test]
fn test_unicode_symbols() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Test with unicode characters
    let mut symbol = create_test_symbol("sym_unicode", "测试函数");
    symbol.doc = Some("这是一个测试函数 with émojis 😀".to_string());
    
    store.insert_symbol(commit_id, &symbol)?;
    
    let retrieved = store.get_symbol("sym_unicode")?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "测试函数");
    
    // Search for unicode
    let results = store.search_symbols("测试", 10)?;
    assert_eq!(results.len(), 1);
    
    Ok(())
}

#[test]
fn test_very_long_names() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Create symbol with very long name
    let long_name = "a".repeat(1000);
    let mut symbol = create_test_symbol("sym_long", &long_name);
    symbol.fqn = format!("test.{}", long_name);
    
    store.insert_symbol(commit_id, &symbol)?;
    
    let retrieved = store.get_symbol("sym_long")?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name.len(), 1000);
    
    Ok(())
}

#[test]
fn test_empty_values() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;

    // Symbol with empty name and FQN should be rejected
    let symbol = SymbolIR {
        id: "empty".to_string(),
        lang: Language::Unknown,
        lang_version: None,
        kind: SymbolKind::Variable,
        name: "".to_string(), // Empty name
        fqn: "".to_string(),  // Empty FQN
        signature: None,
        file_path: "".to_string(), // Empty path
        span: Span {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        visibility: None,
        doc: None,
        sig_hash: "".to_string(),
    };

    // Should fail validation
    let result = store.insert_symbol(commit_id, &symbol);
    assert!(result.is_err(), "Empty name/FQN should be rejected");
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));

    // Valid symbol with minimal but non-empty values should work
    let valid_symbol = SymbolIR {
        id: "valid".to_string(),
        lang: Language::Unknown,
        lang_version: None,
        kind: SymbolKind::Variable,
        name: "x".to_string(), // Non-empty name
        fqn: "x".to_string(),  // Non-empty FQN
        signature: None,
        file_path: "".to_string(), // Empty path is OK
        span: Span {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        visibility: None,
        doc: None,
        sig_hash: "".to_string(),
    };

    store.insert_symbol(commit_id, &valid_symbol)?;
    let retrieved = store.get_symbol("valid")?;
    assert!(retrieved.is_some());

    Ok(())
}

#[test]
fn test_special_characters_in_paths() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Test with special characters in file path
    let mut symbol = create_test_symbol("sym1", "test");
    symbol.file_path = "src/with spaces/and-dashes/under_scores/file.ts".to_string();
    
    store.insert_symbol(commit_id, &symbol)?;
    
    let in_file = store.get_symbols_in_file("src/with spaces/and-dashes/under_scores/file.ts")?;
    assert_eq!(in_file.len(), 1);
    
    Ok(())
}

#[test]
fn test_sql_injection_protection() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Try to inject SQL
    let mut symbol = create_test_symbol("sym1", "test');DROP TABLE symbol;--");
    symbol.fqn = "'; DROP TABLE symbol; --".to_string();
    
    store.insert_symbol(commit_id, &symbol)?;
    
    // Table should still exist
    let count = store.get_symbol_count()?;
    assert_eq!(count, 1);
    
    // Search with injection attempt
    let results = store.search_symbols("'; DROP TABLE symbol; --", 10)?;
    assert_eq!(results.len(), 1);
    
    Ok(())
}

// Note: Concurrent test removed because SQLite connections are not thread-safe (not Send)
// In production, you'd use a connection pool or separate connections per thread

#[test]
fn test_cycle_detection_in_graph() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Create symbols
    for i in 1..=3 {
        let sym = create_test_symbol(&format!("s{}", i), &format!("func{}", i));
        store.insert_symbol(commit_id, &sym)?;
    }
    
    // Create a cycle: s1 -> s2 -> s3 -> s1
    let edges = vec![
        ("s1", "s2"),
        ("s2", "s3"),
        ("s3", "s1"), // Creates cycle
    ];
    
    for (src, dst) in edges {
        store.insert_edge(commit_id, &EdgeIR {
            edge_type: EdgeType::Calls,
            src: Some(src.to_string()),
            dst: Some(dst.to_string()),
            file_src: None,
            file_dst: None,
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        })?;
    }
    
    let graph = store.build_graph()?;
    let stats = graph.stats();
    
    assert!(stats.is_cyclic, "Should detect cycle");
    
    // Test finding cycles
    let cycles = graph.find_cycles_containing("s1");
    assert!(!cycles.is_empty(), "Should find at least one cycle");
    
    Ok(())
}

#[test]
fn test_file_count_distinct() -> Result<()> {
    let (store, _temp_dir) = create_test_store()?;
    
    // Insert same file in multiple commits
    let commit1 = store.get_or_create_commit("commit1")?;
    let commit2 = store.get_or_create_commit("commit2")?;
    
    store.insert_file(commit1, "file.rs", "hash1", 100)?;
    store.insert_file(commit2, "file.rs", "hash2", 200)?;
    store.insert_file(commit1, "other.rs", "hash3", 300)?;
    
    // Should count distinct paths
    let count = store.get_file_count()?;
    assert_eq!(count, 2); // file.rs and other.rs
    
    Ok(())
}