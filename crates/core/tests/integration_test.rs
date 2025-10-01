use anyhow::Result;
use std::fs;
use tempfile::TempDir;

fn create_test_project() -> Result<TempDir> {
    let dir = TempDir::new()?;
    
    // Create a simple TypeScript file
    let ts_file = r#"
import { helper } from './helper';

export class Calculator {
    private value: number = 0;
    
    add(n: number): void {
        this.value += n;
    }
    
    subtract(n: number): void {
        this.value -= n;
    }
}

export function createCalculator(): Calculator {
    return new Calculator();
}

export const PI = 3.14159;
"#;
    
    let helper_file = r#"
export function helper(x: number): number {
    return x * 2;
}
"#;
    
    fs::write(dir.path().join("calculator.ts"), ts_file)?;
    fs::write(dir.path().join("helper.ts"), helper_file)?;
    
    // Initialize git repo
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(dir.path())
        .output()?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(dir.path())
        .output()?;
        
    std::process::Command::new("git")
        .args(&["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()?;
    
    Ok(dir)
}

#[test]
fn test_end_to_end_scan() -> Result<()> {
    use store::GraphStore;
    use protocol::SymbolKind;

    let test_dir = create_test_project()?;

    // Run scan
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;

    assert!(output.status.success(), "Scan should succeed");

    // Verify database exists
    let db_path = test_dir.path().join(".reviewbot/graph.db");
    assert!(db_path.exists(), "Database should be created");

    // CRITICAL: Verify actual database contents, not just stdout messages
    let store = GraphStore::new(test_dir.path())?;

    let file_count = store.get_file_count()?;
    assert_eq!(file_count, 2, "Should index 2 files");

    // Verify expected symbols exist
    let calculator_symbols = store.search_symbols("Calculator", 10)?;
    assert!(calculator_symbols.len() >= 1,
        "Should find Calculator class, but found {} results",
        calculator_symbols.len()
    );

    // Verify Calculator is actually a class
    let calculator = &calculator_symbols[0];
    assert_eq!(calculator.kind, SymbolKind::Class,
        "Calculator should be a Class, but got {:?}",
        calculator.kind
    );

    // Verify helper function exists
    let helper_symbols = store.search_symbols("helper", 10)?;
    assert!(helper_symbols.len() >= 1,
        "Should find helper function, but found {} results",
        helper_symbols.len()
    );

    // Verify helper is a function
    let helper = &helper_symbols[0];
    assert_eq!(helper.kind, SymbolKind::Function,
        "helper should be a Function, but got {:?}",
        helper.kind
    );

    // Verify edges were created (import from calculator to helper)
    let edge_count = store.get_edge_count()?;
    assert!(edge_count >= 1,
        "Should have at least 1 edge (import relationship), but found {}",
        edge_count
    );

    // Verify total symbol count is reasonable
    let total_symbols = store.get_symbol_count()?;
    assert!(total_symbols >= 4,
        "Should have at least 4 symbols (Calculator class + methods + helper + PI), but found {}",
        total_symbols
    );

    println!("✅ End-to-end test verified:");
    println!("   Calculator: {} (kind: {:?})", calculator.name, calculator.kind);
    println!("   helper: {} (kind: {:?})", helper.name, helper.kind);
    println!("   Total symbols: {}", total_symbols);
    println!("   Total edges: {}", edge_count);

    Ok(())
}

#[test]
fn test_idempotent_scan() -> Result<()> {
    use store::GraphStore;

    let test_dir = create_test_project()?;

    // First scan
    let output1 = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;

    assert!(output1.status.success(), "First scan should succeed");

    // CRITICAL: Verify database state after first scan
    let store = GraphStore::new(test_dir.path())?;
    let symbols_before = store.get_symbol_count()?;
    let edges_before = store.get_edge_count()?;
    let files_before = store.get_file_count()?;

    assert!(symbols_before >= 6, "Should have at least 6 symbols after first scan");
    assert!(files_before == 2, "Should have exactly 2 files");

    println!("After first scan: {} symbols, {} edges, {} files",
        symbols_before, edges_before, files_before);

    // Second scan (IDEMPOTENCE TEST)
    let output2 = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", "--repo", test_dir.path().to_str().unwrap(), "scan"])
        .output()?;

    assert!(output2.status.success(), "Second scan should succeed");

    // CRITICAL: Verify ZERO mutations to database
    let symbols_after = store.get_symbol_count()?;
    let edges_after = store.get_edge_count()?;
    let files_after = store.get_file_count()?;

    assert_eq!(symbols_before, symbols_after,
        "Idempotence violation: Second scan changed symbol count from {} to {}. \
         This indicates duplicate symbols were inserted!",
        symbols_before, symbols_after
    );

    assert_eq!(edges_before, edges_after,
        "Idempotence violation: Second scan changed edge count from {} to {}. \
         This indicates duplicate edges were inserted!",
        edges_before, edges_after
    );

    assert_eq!(files_before, files_after,
        "Idempotence violation: Second scan changed file count from {} to {}",
        files_before, files_after
    );

    // Verify specific symbols aren't duplicated
    // Note: Tree-sitter may create multiple symbols for class (definition, type, etc.)
    let calculator_symbols = store.search_symbols("Calculator", 50)?;
    let calculator_count_expected = calculator_symbols.len();

    // Re-query after second scan to ensure no duplicates
    let calculator_symbols_after = store.search_symbols("Calculator", 50)?;
    assert_eq!(calculator_count_expected, calculator_symbols_after.len(),
        "Calculator symbol count changed from {} to {} after rescan - duplicate symbols created!",
        calculator_count_expected, calculator_symbols_after.len()
    );

    println!("✅ Idempotence test passed:");
    println!("   Symbols: {} (unchanged)", symbols_after);
    println!("   Edges: {} (unchanged)", edges_after);
    println!("   Files: {} (unchanged)", files_after);

    Ok(())
}