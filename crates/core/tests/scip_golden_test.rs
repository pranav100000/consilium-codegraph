use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use store::GraphStore;
use scip_mapper::ScipMapper;

/// Golden tests that verify exact symbol/edge counts for regression testing
/// 
/// These tests establish baseline expectations that must remain stable:
/// - Symbol counts per file type and language feature
/// - Cross-file reference counts
/// - Semantic vs syntactic symbol distribution
/// 
/// When these counts change, it indicates either:
/// 1. A regression that needs fixing
/// 2. An improvement that needs new golden values

#[test]
fn test_typescript_interface_golden_counts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    
    // Create a standardized TypeScript interface file
    create_interface_test_file(repo_path)?;
    
    let store = GraphStore::new(repo_path)?;
    let commit_id = store.get_or_create_commit("golden_test")?;
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "golden_test")?;
    
    // Store data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    for occurrence in &occurrences {
        store.insert_occurrence(commit_id, occurrence)?;
    }
    
    // Golden assertions - these should remain stable
    assert_eq!(symbols.len(), 5, "Interface file should produce exactly 5 semantic symbols");
    assert_eq!(occurrences.len(), 5, "Interface file should produce exactly 5 semantic occurrences");
    assert_eq!(edges.len(), 0, "Simple interface file should produce no edges");
    
    // Verify specific symbol types (SCIP creates multiple User-related symbols)
    let user_symbols = store.search_symbols("User", 50)?;
    assert_eq!(user_symbols.len(), 5, "Should find exactly 5 User-related symbols");
    
    let property_symbols = store.search_symbols("id", 50)?;
    assert_eq!(property_symbols.len(), 1, "Should find exactly 1 id property symbol");
    
    println!("✅ TypeScript interface golden test passed:");
    println!("   {} symbols", symbols.len());
    println!("   {} occurrences", occurrences.len());
    println!("   {} edges", edges.len());
    
    Ok(())
}

#[test]
fn test_typescript_class_methods_golden_counts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    
    create_class_methods_test_file(repo_path)?;
    
    let store = GraphStore::new(repo_path)?;
    let commit_id = store.get_or_create_commit("golden_test")?;
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "golden_test")?;
    
    // Store data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    
    // Golden assertions for class with methods
    assert_eq!(symbols.len(), 9, "Class with methods should produce exactly 9 semantic symbols");
    assert_eq!(occurrences.len(), 19, "Class with methods should produce exactly 19 semantic occurrences");
    
    // Verify method symbols (SCIP may create multiple symbols per method)
    let add_method_symbols = store.search_symbols("addItem", 50)?;
    assert_eq!(add_method_symbols.len(), 2, "Should find exactly 2 addItem-related symbols");
    
    let get_method_symbols = store.search_symbols("getCount", 50)?;
    assert_eq!(get_method_symbols.len(), 1, "Should find exactly 1 getCount method symbol");
    
    println!("✅ TypeScript class methods golden test passed:");
    println!("   {} symbols", symbols.len());
    println!("   {} occurrences", occurrences.len());
    
    Ok(())
}

#[test]
fn test_typescript_cross_file_imports_golden_counts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    
    create_import_export_files(repo_path)?;
    
    let store = GraphStore::new(repo_path)?;
    let commit_id = store.get_or_create_commit("golden_test")?;
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "golden_test")?;
    
    // Store data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    
    // Golden assertions for cross-file imports
    assert_eq!(symbols.len(), 7, "Two-file import/export should produce exactly 7 semantic symbols");
    assert_eq!(occurrences.len(), 14, "Two-file import/export should produce exactly 14 semantic occurrences");
    
    // Verify cross-file references (SCIP creates multiple Helper-related symbols)
    let util_symbols = store.search_symbols("Helper", 50)?;
    assert_eq!(util_symbols.len(), 3, "Should find exactly 3 Helper-related symbols");
    
    println!("✅ TypeScript cross-file imports golden test passed:");
    println!("   {} symbols", symbols.len());
    println!("   {} occurrences", occurrences.len());
    
    Ok(())
}

#[test]
fn test_semantic_symbol_distribution_golden() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    
    create_comprehensive_test_file(repo_path)?;
    
    let store = GraphStore::new(repo_path)?;
    let commit_id = store.get_or_create_commit("golden_test")?;
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "golden_test")?;
    
    // Store data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    
    // Symbols created and stored successfully
    
    // Categorize symbols by type
    let mut interface_count = 0;
    let mut class_count = 0;
    let mut function_count = 0;
    let mut method_count = 0;
    let mut variable_count = 0;
    
    for symbol in &symbols {
        match symbol.kind {
            protocol::SymbolKind::Interface => interface_count += 1,
            protocol::SymbolKind::Class => class_count += 1,
            protocol::SymbolKind::Function => function_count += 1,
            protocol::SymbolKind::Method => method_count += 1,
            protocol::SymbolKind::Variable => variable_count += 1,
            _ => {}
        }
    }
    
    // Golden distribution assertions (based on actual SCIP output)
    assert_eq!(interface_count, 0, "SCIP treats interfaces as classes");
    assert_eq!(class_count, 8, "Should find exactly 8 class-related symbols (including interface)");
    assert_eq!(function_count, 2, "Should find exactly 2 function-related symbols");
    assert_eq!(method_count, 0, "Methods are classified as class symbols");
    assert_eq!(variable_count, 2, "Should find exactly 2 variable symbols");
    
    // Total count assertion
    assert_eq!(symbols.len(), 12, "Comprehensive test should produce exactly 12 semantic symbols");
    assert_eq!(occurrences.len(), 27, "Comprehensive test should produce exactly 27 semantic occurrences");
    
    println!("✅ Semantic symbol distribution golden test passed:");
    println!("   {} total symbols", symbols.len());
    println!("   {} interfaces, {} classes, {} functions, {} methods, {} variables", 
             interface_count, class_count, function_count, method_count, variable_count);
    println!("   {} total occurrences", occurrences.len());
    
    Ok(())
}

// Helper functions to create standardized test files

fn create_interface_test_file(repo_path: &Path) -> Result<()> {
    fs::write(
        repo_path.join("user.ts"),
        r#"export interface User {
  id: number;
  name: string;
  email: string;
}"#,
    )?;
    
    create_config_files(repo_path)?;
    Ok(())
}

fn create_class_methods_test_file(repo_path: &Path) -> Result<()> {
    fs::write(
        repo_path.join("container.ts"),
        r#"export class Container<T> {
  private items: T[] = [];

  addItem(item: T): void {
    this.items.push(item);
  }

  getItem(index: number): T | undefined {
    return this.items[index];
  }

  getCount(): number {
    return this.items.length;
  }
}"#,
    )?;
    
    create_config_files(repo_path)?;
    Ok(())
}

fn create_import_export_files(repo_path: &Path) -> Result<()> {
    fs::write(
        repo_path.join("utils.ts"),
        r#"export class Helper {
  static format(value: string): string {
    return value.trim();
  }
}"#,
    )?;
    
    fs::write(
        repo_path.join("main.ts"),
        r#"import { Helper } from "./utils";

function processData(data: string): string {
  return Helper.format(data);
}"#,
    )?;
    
    create_config_files(repo_path)?;
    Ok(())
}

fn create_comprehensive_test_file(repo_path: &Path) -> Result<()> {
    fs::write(
        repo_path.join("comprehensive.ts"),
        r#"export interface Config {
  debug: boolean;
}

export class DataProcessor {
  private config: Config;

  constructor(config: Config) {
    this.config = config;
  }

  process(data: string): string {
    if (this.config.debug) {
      console.log("Processing:", data);
    }
    return data.toUpperCase();
  }
}

export function createProcessor(debug = false): DataProcessor {
  return new DataProcessor({ debug });
}

const defaultProcessor = createProcessor();"#,
    )?;
    
    create_config_files(repo_path)?;
    Ok(())
}

fn create_config_files(repo_path: &Path) -> Result<()> {
    fs::write(
        repo_path.join("package.json"),
        r#"{
  "name": "golden-test",
  "version": "1.0.0",
  "dependencies": {
    "typescript": "^5.0.0"
  }
}"#,
    )?;
    
    fs::write(
        repo_path.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": true
  }
}"#,
    )?;
    
    Ok(())
}