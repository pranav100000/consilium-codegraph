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

    // Verify by searching - no symbols should be findable
    let search_results = store.search_symbols("ValidClass", 10)?;
    assert_eq!(search_results.len(), 0,
        "Found {} symbols in search, but all should have been rolled back",
        search_results.len()
    );

    println!("✅ Transaction correctly rolled back on validation error");
    Ok(())
}

#[test]
fn test_batch_insert_handles_duplicates_with_replace() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test_commit")?;

    // Create symbols with duplicate IDs to verify INSERT OR REPLACE behavior
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

    // Verify it's the second symbol that was kept
    let symbols_in_db = store.search_symbols("Class2", 10)?;
    assert_eq!(symbols_in_db.len(), 1, "Should find the replaced symbol");

    println!("✅ INSERT OR REPLACE correctly handles duplicates");
    Ok(())
}

#[test]
fn test_batch_insert_with_large_data() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test_commit")?;

    // Create a symbol with very large signature
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

    // Verify symbol was inserted
    let symbol_count = store.get_symbol_count()?;
    assert_eq!(symbol_count, 1, "Large symbol should be inserted");

    println!("✅ Large data handled correctly");
    Ok(())
}

/// CRITICAL TEST: Verify rollback occurs when transaction fails mid-batch
/// This test ensures that if ANY error occurs during batch insert,
/// ALL changes are rolled back (not just subsequent ones)
#[test]
fn test_batch_insert_rollback_on_midpoint_failure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = GraphStore::new(temp_dir.path())?;
    let commit_id = store.create_commit_snapshot("test_commit")?;

    // First, insert some valid symbols to establish baseline
    let baseline_symbols = vec![
        SymbolIR {
            id: "baseline_1".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "Baseline1".to_string(),
            fqn: "test.Baseline1".to_string(),
            signature: None,
            file_path: "baseline.ts".to_string(),
            span: Span { start_line: 0, start_col: 0, end_line: 1, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "hash_baseline".to_string(),
        },
    ];

    store.batch_insert_symbols(commit_id, &baseline_symbols)?;

    let baseline_count = store.get_symbol_count()?;
    assert_eq!(baseline_count, 1, "Baseline should have 1 symbol");

    // Now create a batch where the MIDDLE symbol is invalid
    let mut symbols = Vec::new();

    // First 10 valid symbols
    for i in 0..10 {
        symbols.push(SymbolIR {
            id: format!("valid_before_{}", i),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: format!("ValidFunc{}", i),
            fqn: format!("test.ValidFunc{}", i),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span { start_line: i, start_col: 0, end_line: i + 1, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: format!("hash_{}", i),
        });
    }

    // INVALID symbol in the middle (empty FQN)
    symbols.push(SymbolIR {
        id: "invalid_middle".to_string(),
        lang: Language::TypeScript,
        lang_version: Some(Version::ES2020),
        kind: SymbolKind::Function,
        name: "InvalidFunc".to_string(),
        fqn: "".to_string(),  // ❌ EMPTY FQN - INVALID!
        signature: None,
        file_path: "test.ts".to_string(),
        span: Span { start_line: 10, start_col: 0, end_line: 11, end_col: 0 },
        visibility: None,
        doc: None,
        sig_hash: "hash_invalid".to_string(),
    });

    // More valid symbols after the invalid one
    for i in 11..20 {
        symbols.push(SymbolIR {
            id: format!("valid_after_{}", i),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: format!("ValidFunc{}", i),
            fqn: format!("test.ValidFunc{}", i),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span { start_line: i, start_col: 0, end_line: i + 1, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: format!("hash_{}", i),
        });
    }

    // Batch insert should FAIL on the invalid symbol
    let result = store.batch_insert_symbols(commit_id, &symbols);
    assert!(result.is_err(),
        "Batch insert should fail when encountering invalid symbol in middle");

    // CRITICAL VERIFICATION: NO symbols from failed batch should be in database
    let count_after_failure = store.get_symbol_count()?;
    assert_eq!(count_after_failure, baseline_count,
        "Transaction rollback failed! Expected {} symbols (baseline only), but found {}. \
         This means the first 10 valid symbols were committed before the error, \
         which indicates partial transaction commit - a critical bug!",
        baseline_count,
        count_after_failure
    );

    // Double-check: Search for symbols that should NOT exist
    let search_results = store.search_symbols("ValidFunc", 50)?;
    assert_eq!(search_results.len(), 0,
        "Found {} symbols from failed batch. All should have been rolled back!",
        search_results.len()
    );

    println!("✅ Transaction correctly rolled back ALL changes when failure occurred at midpoint");
    Ok(())
}
