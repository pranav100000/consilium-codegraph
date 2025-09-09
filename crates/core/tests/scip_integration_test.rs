use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use store::GraphStore;
use scip_mapper::ScipMapper;

/// Integration tests for SCIP semantic analysis
/// 
/// These tests verify that:
/// 1. SCIP indexing works end-to-end
/// 2. Semantic symbols are correctly parsed and stored
/// 3. Cross-file symbol resolution works
/// 4. Semantic + syntactic data coexist properly

#[test]
fn test_scip_typescript_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    
    // Create a TypeScript project with cross-file dependencies
    create_typescript_test_files(repo_path)?;
    
    // Initialize SCIP mapper
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    // Generate SCIP index
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    assert!(Path::new(&scip_file).exists(), "SCIP index file should be created");
    
    // Parse SCIP index
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    assert!(!scip_index.documents.is_empty(), "Should find documents in SCIP index");
    
    // Convert to IR
    let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test_commit")?;
    
    // Verify semantic data quality
    assert!(!symbols.is_empty(), "Should find semantic symbols");
    assert!(!occurrences.is_empty(), "Should find semantic occurrences");
    
    println!("✅ SCIP integration found:");
    println!("   {} symbols", symbols.len());
    println!("   {} edges", edges.len()); 
    println!("   {} occurrences", occurrences.len());
    
    Ok(())
}

#[test]
fn test_semantic_symbol_storage() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let db_path = repo_path.join(".reviewbot").join("graph.db");
    
    // Create test files
    create_typescript_test_files(repo_path)?;
    
    // Initialize store and SCIP mapper  
    let store = GraphStore::new(repo_path)?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    // Run semantic analysis
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test_commit")?;
    
    // Store semantic data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    for occurrence in &occurrences {
        store.insert_occurrence(commit_id, occurrence)?;
    }
    
    // Verify storage
    let stored_symbols = store.search_symbols("User", 50)?;
    assert!(!stored_symbols.is_empty(), "Should find User symbols in database");
    
    let stored_symbols = store.search_symbols("UserService", 50)?;
    assert!(!stored_symbols.is_empty(), "Should find UserService symbols in database");
    
    println!("✅ Semantic storage verified:");
    println!("   {} symbols stored", symbols.len());
    println!("   {} occurrences stored", occurrences.len());
    
    Ok(())
}

#[test] 
fn test_cross_file_semantic_resolution() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let db_path = repo_path.join(".reviewbot").join("graph.db");
    
    // Create files that reference each other
    create_cross_file_typescript_project(repo_path)?;
    
    // Initialize store and run full semantic analysis
    let store = GraphStore::new(repo_path)?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (symbols, _edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test_commit")?;
    
    // Store all data
    for symbol in &symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    for occurrence in &occurrences {
        store.insert_occurrence(commit_id, occurrence)?;
    }
    
    // Test cross-file resolution
    let user_symbols = store.search_symbols("User", 50)?;
    let service_symbols = store.search_symbols("Service", 50)?;
    
    // Should find symbols from both files
    let mut found_files = std::collections::HashSet::new();
    for symbol in &user_symbols {
        found_files.insert(&symbol.file_path);
    }
    for symbol in &service_symbols {
        found_files.insert(&symbol.file_path);
    }
    
    assert!(found_files.len() > 1, "Should find symbols across multiple files: {:?}", found_files);
    
    println!("✅ Cross-file resolution verified:");
    println!("   {} files with User symbols", found_files.len());
    println!("   {} total symbols found", user_symbols.len() + service_symbols.len());
    
    Ok(())
}

#[test]
fn test_semantic_vs_syntactic_coexistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let db_path = repo_path.join(".reviewbot").join("graph.db");
    
    create_typescript_test_files(repo_path)?;
    
    let store = GraphStore::new(repo_path)?;
    let commit_id = store.get_or_create_commit("test_commit")?;
    
    // Run syntactic analysis (simulate what would happen in main scan)
    // This would normally be done by the ts_harness, but we'll create sample syntactic symbols
    let syntactic_symbols = create_sample_syntactic_symbols();
    for symbol in &syntactic_symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    
    // Run semantic analysis
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16")
        .with_scip_cli_path("/Users/pranavsharan/go/bin/scip".to_string());
    
    let scip_file = scip_mapper.run_scip_typescript(&repo_path.to_string_lossy())?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (semantic_symbols, _edges, _occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test_commit")?;
    
    for symbol in &semantic_symbols {
        store.insert_symbol(commit_id, symbol)?;
    }
    
    // Verify both types coexist
    let all_symbols = store.search_symbols("User", 100)?;
    
    // Should have both syntactic and semantic symbols
    let mut semantic_count = 0;
    let mut syntactic_count = 0;
    
    for symbol in &all_symbols {
        if symbol.fqn.contains('.') {
            semantic_count += 1; // SCIP symbols typically use dot notation
        } else {
            syntactic_count += 1; // Tree-sitter symbols use slash notation
        }
    }
    
    assert!(semantic_count > 0, "Should find semantic symbols");
    assert!(syntactic_count > 0, "Should find syntactic symbols");
    
    println!("✅ Coexistence verified:");
    println!("   {} semantic symbols", semantic_count);
    println!("   {} syntactic symbols", syntactic_count);
    
    Ok(())
}

// Helper functions

fn create_typescript_test_files(repo_path: &Path) -> Result<()> {
    fs::write(
        repo_path.join("user.ts"),
        r#"export interface User {
  id: number;
  name: string;
  email: string;
}

export class UserService {
  private users: User[] = [];

  addUser(user: User): void {
    this.users.push(user);
  }

  findUser(id: number): User | undefined {
    return this.users.find(u => u.id === id);
  }

  getAllUsers(): User[] {
    return this.users;
  }
}"#,
    )?;
    
    fs::write(
        repo_path.join("main.ts"),
        r#"import { User, UserService } from "./user";

function createTestUser(id: number, name: string): User {
  return {
    id,
    name,
    email: `${name}@example.com`
  };
}

function main(): void {
  const service = new UserService();
  
  const user1 = createTestUser(1, "Alice");
  const user2 = createTestUser(2, "Bob");
  
  service.addUser(user1);
  service.addUser(user2);
  
  const found = service.findUser(1);
  if (found) {
    console.log(`Found user: ${found.name}`);
  }
  
  const allUsers = service.getAllUsers();
  console.log(`Total users: ${allUsers.length}`);
}

main();"#,
    )?;
    
    // Create package.json and tsconfig.json for proper TypeScript setup
    fs::write(
        repo_path.join("package.json"),
        r#"{
  "name": "scip-test",
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

fn create_cross_file_typescript_project(repo_path: &Path) -> Result<()> {
    // Create multiple files with dependencies
    
    fs::write(
        repo_path.join("types.ts"),
        r#"export interface User {
  id: number;
  name: string;
  email: string;
}

export interface Product {
  id: number;
  title: string;
  price: number;
}"#,
    )?;
    
    fs::write(
        repo_path.join("user-service.ts"),
        r#"import { User } from "./types";

export class UserService {
  private users: User[] = [];

  addUser(user: User): void {
    this.users.push(user);
  }

  findUser(id: number): User | undefined {
    return this.users.find(u => u.id === id);
  }
}"#,
    )?;
    
    fs::write(
        repo_path.join("product-service.ts"),
        r#"import { Product } from "./types";

export class ProductService {
  private products: Product[] = [];

  addProduct(product: Product): void {
    this.products.push(product);
  }

  findProduct(id: number): Product | undefined {
    return this.products.find(p => p.id === id);
  }
}"#,
    )?;
    
    fs::write(
        repo_path.join("app.ts"),
        r#"import { User, Product } from "./types";
import { UserService } from "./user-service";
import { ProductService } from "./product-service";

class App {
  private userService = new UserService();
  private productService = new ProductService();

  init(): void {
    const user: User = { id: 1, name: "Alice", email: "alice@example.com" };
    const product: Product = { id: 1, title: "Laptop", price: 999 };
    
    this.userService.addUser(user);
    this.productService.addProduct(product);
  }
}"#,
    )?;
    
    // Add configuration files
    fs::write(
        repo_path.join("package.json"),
        r#"{
  "name": "cross-file-test",
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

fn create_sample_syntactic_symbols() -> Vec<protocol::SymbolIR> {
    use protocol::*;
    
    vec![
        SymbolIR {
            id: "test/user.ts#sym(TypeScript:user/User:4)".to_string(),
            lang: Language::TypeScript,
            lang_version: None,
            kind: SymbolKind::Interface,
            name: "User".to_string(),
            fqn: "user/User".to_string(),
            signature: None,
            file_path: "user.ts".to_string(),
            span: Span { start_line: 1, start_col: 0, end_line: 5, end_col: 1 },
            visibility: None,
            doc: None,
            sig_hash: "4".to_string(),
        },
        SymbolIR {
            id: "test/user.ts#sym(TypeScript:user/UserService:11)".to_string(),
            lang: Language::TypeScript,
            lang_version: None,
            kind: SymbolKind::Class,
            name: "UserService".to_string(),
            fqn: "user/UserService".to_string(),
            signature: None,
            file_path: "user.ts".to_string(),
            span: Span { start_line: 7, start_col: 0, end_line: 21, end_col: 1 },
            visibility: None,
            doc: None,
            sig_hash: "11".to_string(),
        },
    ]
}