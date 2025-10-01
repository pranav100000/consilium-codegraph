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

    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16");

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

    // Verify specific symbol types
    let user_symbols = store.search_symbols("User", 50)?;
    let property_symbols = store.search_symbols("id", 50)?;

    // Golden assertions - values determined by running SCIP on standardized test file
    // If SCIP behavior changes, these values should be updated
    assert_eq!(symbols.len(), 4,
        "Interface file should produce exactly 4 semantic symbols (actual: {})", symbols.len());
    assert_eq!(occurrences.len(), 5,
        "Interface file should produce exactly 5 semantic occurrences (actual: {})", occurrences.len());
    assert_eq!(edges.len(), 0,
        "Simple interface file should produce no edges (actual: {})", edges.len());

    assert_eq!(user_symbols.len(), 4,
        "Should find exactly 4 User-related symbols (actual: {})", user_symbols.len());
    assert_eq!(property_symbols.len(), 1,
        "Should find exactly 1 id property symbol (actual: {})", property_symbols.len());

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

    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16");

    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "golden_test")?;

    // Store data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }

    // Verify method symbols (SCIP may create multiple symbols per method)
    let add_method_symbols = store.search_symbols("addItem", 50)?;
    let get_method_symbols = store.search_symbols("getCount", 50)?;

    // Golden assertions - values determined by running SCIP on standardized test file
    assert_eq!(symbols.len(), 8,
        "Class with methods should produce exactly 8 semantic symbols (actual: {})", symbols.len());
    assert_eq!(occurrences.len(), 19,
        "Class with methods should produce exactly 19 semantic occurrences (actual: {})", occurrences.len());

    assert_eq!(add_method_symbols.len(), 2,
        "Should find exactly 2 addItem-related symbols (actual: {})", add_method_symbols.len());
    assert_eq!(get_method_symbols.len(), 1,
        "Should find exactly 1 getCount method symbol (actual: {})", get_method_symbols.len());

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

    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16");

    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "golden_test")?;

    // Store data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }

    // Verify cross-file references (SCIP creates multiple Helper-related symbols)
    let util_symbols = store.search_symbols("Helper", 50)?;

    // Golden assertions - values determined by running SCIP on standardized test file
    assert_eq!(symbols.len(), 5,
        "Two-file import/export should produce exactly 5 semantic symbols (actual: {})", symbols.len());
    assert_eq!(occurrences.len(), 14,
        "Two-file import/export should produce exactly 14 semantic occurrences (actual: {})", occurrences.len());

    assert_eq!(util_symbols.len(), 1,
        "Should find exactly 1 Helper-related symbol (actual: {})", util_symbols.len());

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

    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16");

    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "golden_test")?;

    // Store data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }

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

    // Golden distribution assertions - values determined by running SCIP on standardized test file
    assert_eq!(interface_count, 0,
        "SCIP treats interfaces as classes (actual: {})", interface_count);
    assert_eq!(class_count, 2,
        "Should find exactly 2 class-related symbols (actual: {})", class_count);
    assert_eq!(function_count, 2,
        "Should find exactly 2 function-related symbols (actual: {})", function_count);
    assert_eq!(method_count, 4,
        "Should find exactly 4 method symbols (actual: {})", method_count);
    assert_eq!(variable_count, 1,
        "Should find exactly 1 variable symbol (actual: {})", variable_count);

    // Total count assertion (0 interface + 2 class + 2 function + 4 method + 1 variable = 9, but actual is 11)
    assert_eq!(symbols.len(), 11,
        "Comprehensive test should produce exactly 11 semantic symbols (actual: {})", symbols.len());
    assert_eq!(occurrences.len(), 27,
        "Comprehensive test should produce exactly 27 semantic occurrences (actual: {})", occurrences.len());
    
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