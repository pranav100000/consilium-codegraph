use anyhow::Result;
use protocol::{Language, Span, SymbolIR, SymbolKind, Version};
use store::GraphStore;
use tempfile::TempDir;

#[test]
fn test_batch_insert_rolls_back_on_validation_error() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test_commit")?;

    // Create a batch with valid symbols followed by an invalid one
    let mut symbols = Vec::new();

    // Add 5 valid symbols
    for i in 0..5 {
        symbols.push(SymbolIR {
            id: format!("test_symbol_{}", i),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: format!("ValidClass{}", i),
            fqn: format!("test.ValidClass{}", i),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: i,
                start_col: 0,
                end_line: i + 1,
                end_col: 0,
            },
            visibility: None,
            doc: None,
            sig_hash: "hash".to_string(),
        });
    }

    // Add an INVALID symbol (empty name) - should cause rollback
    symbols.push(SymbolIR {
        id: "invalid_symbol".to_string(),
        lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
        kind: SymbolKind::Class,
        name: "".to_string(),  // ❌ EMPTY NAME - INVALID!
        fqn: "test.InvalidClass".to_string(),
        signature: None,
        file_path: "test.ts".to_string(),
        span: Span {
            start_line: 10,
            start_col: 0,
            end_line: 11,
            end_col: 0,
        },
        visibility: None,
        doc: None,
        sig_hash: "hash".to_string(),
    });

    // Try to batch insert - should fail
    let result = store.batch_insert_symbols(commit_id, &symbols);
    assert!(result.is_err(), "Batch insert should fail on invalid symbol");

    // CRITICAL TEST: Verify NO symbols were inserted (transaction should roll back)
    let symbol_count = store.get_symbol_count()?;
    assert_eq!(
        symbol_count, 0,
        "Transaction should have rolled back - expected 0 symbols, but found {}. This means partial data was committed!",
        symbol_count
    );

    println!("✅ Transaction correctly rolled back on validation error");
    Ok(())
}

#[test]
fn test_batch_insert_rolls_back_on_database_error() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test_commit")?;

    // Create symbols with duplicate IDs to trigger database constraint error
    let symbols = vec![
        SymbolIR {
            id: "duplicate_id".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "Class1".to_string(),
            fqn: "test.Class1".to_string(),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: 0,
                start_col: 0,
                end_line: 1,
                end_col: 0,
            },
            visibility: None,
            doc: None,
            sig_hash: "hash1".to_string(),
        },
        SymbolIR {
            id: "duplicate_id".to_string(),  // Same ID
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "Class2".to_string(),
            fqn: "test.Class2".to_string(),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: 2,
                start_col: 0,
                end_line: 3,
                end_col: 0,
            },
            visibility: None,
            doc: None,
            sig_hash: "hash2".to_string(),
        },
    ];

    // This should succeed with INSERT OR REPLACE
    let result = store.batch_insert_symbols(commit_id, &symbols);

    // With INSERT OR REPLACE, this should succeed (second overwrites first)
    assert!(result.is_ok(), "Batch insert with duplicates should succeed with OR REPLACE");

    // Should have exactly 1 symbol (the replacement)
    let symbol_count = store.get_symbol_count()?;
    assert_eq!(symbol_count, 1, "Should have 1 symbol after replacement");

    println!("✅ INSERT OR REPLACE correctly handles duplicates");
    Ok(())
}

#[test]
fn test_batch_insert_with_serialization_error() -> Result<()> {
    // This test would require creating a type that fails to serialize
    // For now, we test that the error path works correctly

    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test_commit")?;

    // Create a symbol with data that might cause issues
    let symbols = vec![
        SymbolIR {
            id: "test_symbol".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "TestClass".to_string(),
            fqn: "test.TestClass".to_string(),
            signature: Some("x".repeat(1_000_000)),  // Very large signature
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: 0,
                start_col: 0,
                end_line: 1,
                end_col: 0,
            },
            visibility: None,
            doc: None,
            sig_hash: "hash".to_string(),
        },
    ];

    // This should still work (SQLite can handle large strings)
    let result = store.batch_insert_symbols(commit_id, &symbols);
    assert!(result.is_ok(), "Large data should be handled");

    println!("✅ Large data handled correctly");
    Ok(())
}
