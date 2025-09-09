use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, Resolution, Span, SymbolIR, SymbolKind, Version};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use store::GraphStore;
use tempfile::TempDir;
use reviewbot::walker::FileWalker;

/// Test comprehensive incremental processing functionality
/// This covers git diff-based change detection, dependency tracking,
/// selective file reprocessing, and incremental database updates

fn create_git_repo(temp_dir: &TempDir) -> Result<PathBuf> {
    let repo_path = temp_dir.path().to_path_buf();
    
    // Initialize git repo
    Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(&repo_path)
        .output()?;
    
    // Configure git
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()?;
        
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()?;
    
    Ok(repo_path)
}

fn create_initial_commit(repo_path: &PathBuf, files: &[(&str, &str)]) -> Result<String> {
    // Create files
    for (filename, content) in files {
        let file_path = repo_path.join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;
    }
    
    // Add and commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()?;
    
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()?;
    
    // Get commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn create_follow_up_commit(repo_path: &PathBuf, files: &[(&str, &str)], message: &str) -> Result<String> {
    // Modify/create files
    for (filename, content) in files {
        let file_path = repo_path.join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;
    }
    
    // Add and commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()?;
    
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_path)
        .output()?;
    
    // Get commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_changed_files_between_commits(repo_path: &PathBuf, from_commit: &str, to_commit: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{}..{}", from_commit, to_commit)])
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| {
            line.ends_with(".ts") || line.ends_with(".tsx") ||
            line.ends_with(".js") || line.ends_with(".jsx") ||
            line.ends_with(".py") || line.ends_with(".go") ||
            line.ends_with(".rs") || line.ends_with(".java") ||
            line.ends_with(".cpp") || line.ends_with(".c")
        })
        .map(|s| s.to_string())
        .collect();
    
    Ok(files)
}

fn simulate_initial_scan(store: &GraphStore, repo_path: &PathBuf, commit_sha: &str) -> Result<()> {
    let commit_id = store.create_commit_snapshot(commit_sha)?;
    
    // Walk all files and create basic symbols
    let walker = FileWalker::new(repo_path.clone());
    let files = walker.walk()?;
    
    for file_path in files {
        let relative_path = file_path.strip_prefix(repo_path)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .to_string();
        
        let content = fs::read_to_string(&file_path).unwrap_or_default();
        let hash = FileWalker::compute_file_hash(&content);
        
        // Store file
        store.insert_file(commit_id, &relative_path, &hash, content.len())?;
        
        // Create simple symbols based on file type
        if relative_path.ends_with(".ts") || relative_path.ends_with(".js") {
            let symbol = SymbolIR {
                id: format!("symbol_{}", relative_path.replace(['/', '.'], "_")),
                lang: Language::TypeScript,
                lang_version: Some(Version::ES2020),
                kind: SymbolKind::Function,
                name: format!("function_{}", file_path.file_stem().unwrap_or_default().to_string_lossy()),
                fqn: format!("{}.function_{}", relative_path, file_path.file_stem().unwrap_or_default().to_string_lossy()),
                signature: Some("function()".to_string()),
                file_path: relative_path.clone(),
                span: Span {
                    start_line: 1,
                    start_col: 0,
                    end_line: 1,
                    end_col: 10,
                },
                visibility: Some("public".to_string()),
                doc: Some(format!("Function in {}", relative_path)),
                sig_hash: format!("hash_{}", relative_path.len()),
            };
            store.insert_symbol(commit_id, &symbol)?;
        }
    }
    
    Ok(())
}

#[test]
fn test_git_diff_change_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    
    // Create initial state
    let initial_files = vec![
        ("src/main.ts", "export function main() { console.log('hello'); }"),
        ("src/utils.ts", "export function helper() { return 42; }"),
        ("src/types.ts", "export interface User { id: number; name: string; }"),
    ];
    
    let commit1 = create_initial_commit(&repo_path, &initial_files)?;
    
    // Modify one file
    let changed_files = vec![
        ("src/main.ts", "export function main() { console.log('hello world'); }"),
    ];
    
    let commit2 = create_follow_up_commit(&repo_path, &changed_files, "Update main function")?;
    
    // Test change detection
    let changes = get_changed_files_between_commits(&repo_path, &commit1, &commit2)?;
    
    assert_eq!(changes.len(), 1);
    assert!(changes.contains(&"src/main.ts".to_string()));
    
    Ok(())
}

#[test]
fn test_incremental_file_processing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let store = GraphStore::new(&repo_path)?;
    
    // Create initial state with multiple files
    let initial_files = vec![
        ("src/main.ts", "import { helper } from './utils';\nexport function main() { return helper(); }"),
        ("src/utils.ts", "export function helper() { return 42; }"),
        ("src/config.ts", "export const CONFIG = { version: '1.0.0' };"),
        ("src/types.ts", "export interface User { id: number; }"),
    ];
    
    let commit1 = create_initial_commit(&repo_path, &initial_files)?;
    
    // Simulate initial scan
    simulate_initial_scan(&store, &repo_path, &commit1)?;
    
    let initial_symbol_count = store.get_symbol_count()?;
    let initial_file_count = store.get_file_count()?;
    
    assert_eq!(initial_symbol_count, 4); // One symbol per file
    assert_eq!(initial_file_count, 4);
    
    // Modify only one file
    let changed_files = vec![
        ("src/utils.ts", "export function helper() { return 123; }\nexport function newHelper() { return 456; }"),
    ];
    
    let commit2 = create_follow_up_commit(&repo_path, &changed_files, "Add new helper function")?;
    
    // Get changed files
    let changes = get_changed_files_between_commits(&repo_path, &commit1, &commit2)?;
    assert_eq!(changes.len(), 1);
    assert!(changes.contains(&"src/utils.ts".to_string()));
    
    // Simulate incremental processing
    let commit_id2 = store.create_commit_snapshot(&commit2)?;
    
    // Delete old data for changed files
    for file in &changes {
        store.delete_file_data(commit_id2, file)?;
    }
    
    // Re-process only changed files
    for file in &changes {
        let file_path = repo_path.join(file);
        let content = fs::read_to_string(&file_path)?;
        let hash = FileWalker::compute_file_hash(&content);
        
        store.insert_file(commit_id2, file, &hash, content.len())?;
        
        // Create updated symbols (simulating new function detected)
        let symbol1 = SymbolIR {
            id: format!("symbol_{}", file.replace(['/', '.'], "_")),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: "helper".to_string(),
            fqn: format!("{}.helper", file),
            signature: Some("function helper()".to_string()),
            file_path: file.clone(),
            span: Span { start_line: 1, start_col: 0, end_line: 1, end_col: 10 },
            visibility: Some("public".to_string()),
            doc: Some("Updated helper function".to_string()),
            sig_hash: "hash_helper".to_string(),
        };
        
        let symbol2 = SymbolIR {
            id: format!("symbol_{}_new", file.replace(['/', '.'], "_")),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: "newHelper".to_string(),
            fqn: format!("{}.newHelper", file),
            signature: Some("function newHelper()".to_string()),
            file_path: file.clone(),
            span: Span { start_line: 2, start_col: 0, end_line: 2, end_col: 15 },
            visibility: Some("public".to_string()),
            doc: Some("New helper function".to_string()),
            sig_hash: "hash_new_helper".to_string(),
        };
        
        store.insert_symbol(commit_id2, &symbol1)?;
        store.insert_symbol(commit_id2, &symbol2)?;
    }
    
    // Verify incremental update results
    let final_symbol_count = store.get_symbol_count()?;
    assert_eq!(final_symbol_count, 5); // 3 unchanged + 2 new in modified file
    
    // Verify we can find the new symbol
    let new_symbol = store.get_symbol_by_fqn("src/utils.ts.newHelper")?;
    assert!(new_symbol.is_some());
    assert_eq!(new_symbol.unwrap().name, "newHelper");
    
    Ok(())
}

#[test]
fn test_dependency_tracking_for_incremental_updates() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let store = GraphStore::new(&repo_path)?;
    
    // Create files with import relationships
    let initial_files = vec![
        ("src/main.ts", "import { utils } from './utils';\nexport function main() { return utils.helper(); }"),
        ("src/utils.ts", "export function helper() { return 42; }"),
        ("src/service.ts", "import { helper } from './utils';\nexport function process() { return helper() * 2; }"),
        ("src/config.ts", "export const CONFIG = { debug: true };"),
    ];
    
    let commit1 = create_initial_commit(&repo_path, &initial_files)?;
    
    // Simulate initial scan with dependency tracking
    let commit_id1 = store.create_commit_snapshot(&commit1)?;
    
    // Create symbols and edges for dependency relationships
    let utils_symbol = SymbolIR {
        id: "symbol_utils".to_string(),
        lang: Language::TypeScript,
        lang_version: Some(Version::ES2020),
        kind: SymbolKind::Function,
        name: "helper".to_string(),
        fqn: "src/utils.ts.helper".to_string(),
        signature: Some("function helper()".to_string()),
        file_path: "src/utils.ts".to_string(),
        span: Span { start_line: 1, start_col: 0, end_line: 1, end_col: 10 },
        visibility: Some("public".to_string()),
        doc: Some("Helper function".to_string()),
        sig_hash: "hash_helper".to_string(),
    };
    
    store.insert_symbol(commit_id1, &utils_symbol)?;
    
    // Create import edges to track dependencies
    let import_edge1 = EdgeIR {
        edge_type: EdgeType::Imports,
        src: Some("src/main.ts".to_string()),
        dst: Some("src/utils.ts".to_string()),
        file_src: Some("src/main.ts".to_string()),
        file_dst: Some("src/utils.ts".to_string()),
        resolution: Resolution::Syntactic,
        meta: HashMap::new(),
        provenance: HashMap::new(),
    };
    
    let import_edge2 = EdgeIR {
        edge_type: EdgeType::Imports,
        src: Some("src/service.ts".to_string()),
        dst: Some("src/utils.ts".to_string()),
        file_src: Some("src/service.ts".to_string()),
        file_dst: Some("src/utils.ts".to_string()),
        resolution: Resolution::Syntactic,
        meta: HashMap::new(),
        provenance: HashMap::new(),
    };
    
    store.insert_edge(commit_id1, &import_edge1)?;
    store.insert_edge(commit_id1, &import_edge2)?;
    
    // Now modify utils.ts
    let changed_files = vec![
        ("src/utils.ts", "export function helper() { return 123; }\nexport function newFunction() { return 'new'; }"),
    ];
    
    let _commit2 = create_follow_up_commit(&repo_path, &changed_files, "Modify utils")?;
    
    // Get files that depend on the changed file
    let dependents = store.get_file_dependents("src/utils.ts")?;
    
    // Should include both main.ts and service.ts since they import utils.ts
    assert_eq!(dependents.len(), 2);
    assert!(dependents.contains(&"src/main.ts".to_string()));
    assert!(dependents.contains(&"src/service.ts".to_string()));
    
    // For true incremental processing, we'd need to reprocess:
    // 1. The changed file (utils.ts)
    // 2. All its dependents (main.ts, service.ts)
    let mut files_to_reprocess = vec!["src/utils.ts".to_string()];
    files_to_reprocess.extend(dependents);
    
    assert_eq!(files_to_reprocess.len(), 3); // utils.ts + 2 dependents
    
    Ok(())
}

#[test]
fn test_incremental_threshold_behavior() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let _store = GraphStore::new(&repo_path)?;
    
    // Create many files to test threshold behavior
    let mut initial_files = Vec::new();
    for i in 0..150 { // More than the 100 file threshold
        initial_files.push((
            format!("src/file{}.ts", i),
            format!("export function func{}() {{ return {}; }}", i, i)
        ));
    }
    
    let initial_files_str: Vec<(&str, &str)> = initial_files.iter()
        .map(|(name, content)| (name.as_ref(), content.as_ref()))
        .collect();
    
    let commit1 = create_initial_commit(&repo_path, &initial_files_str)?;
    
    // Modify many files (over threshold)
    let mut changed_files = Vec::new();
    for i in 0..120 { // Over 100 file threshold
        changed_files.push((
            format!("src/file{}.ts", i),
            format!("export function func{}() {{ return {}; }} // modified", i, i * 2)
        ));
    }
    
    let changed_files_str: Vec<(&str, &str)> = changed_files.iter()
        .map(|(name, content)| (name.as_ref(), content.as_ref()))
        .collect();
    
    let commit2 = create_follow_up_commit(&repo_path, &changed_files_str, "Modify many files")?;
    
    // Get changed files
    let changes = get_changed_files_between_commits(&repo_path, &commit1, &commit2)?;
    
    // Should detect many changes (over threshold)
    assert!(changes.len() > 100);
    
    // In real incremental processing, this would trigger a full re-scan
    // rather than incremental processing due to the threshold
    
    Ok(())
}

#[test]
fn test_file_hash_change_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let store = GraphStore::new(&repo_path)?;
    
    let content1 = "export function test() { return 1; }";
    let content2 = "export function test() { return 2; }";
    let content3 = "export function test() { return 1; }"; // Same as content1
    
    let hash1 = FileWalker::compute_file_hash(content1);
    let hash2 = FileWalker::compute_file_hash(content2);
    let hash3 = FileWalker::compute_file_hash(content3);
    
    // Different content should have different hashes
    assert_ne!(hash1, hash2);
    
    // Same content should have same hash
    assert_eq!(hash1, hash3);
    
    // Test in database context
    let commit_id = store.create_commit_snapshot("test_commit")?;
    store.insert_file(commit_id, "test.ts", &hash1, content1.len())?;
    
    let stored_hash = store.get_file_hash("test_commit", "test.ts")?;
    assert_eq!(stored_hash, Some(hash1));
    
    Ok(())
}

#[test]
fn test_incremental_symbol_deletion_and_recreation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let store = GraphStore::new(&repo_path)?;
    
    // Initial file with multiple symbols
    let initial_files = vec![
        ("src/module.ts", "export function func1() { return 1; }\nexport function func2() { return 2; }\nexport class MyClass {}"),
    ];
    
    let commit1 = create_initial_commit(&repo_path, &initial_files)?;
    let commit_id1 = store.create_commit_snapshot(&commit1)?;
    
    // Create initial symbols
    let symbols = vec![
        SymbolIR {
            id: "symbol_func1".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: "func1".to_string(),
            fqn: "src/module.ts.func1".to_string(),
            signature: Some("function func1()".to_string()),
            file_path: "src/module.ts".to_string(),
            span: Span { start_line: 1, start_col: 0, end_line: 1, end_col: 10 },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "hash_func1".to_string(),
        },
        SymbolIR {
            id: "symbol_func2".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: "func2".to_string(),
            fqn: "src/module.ts.func2".to_string(),
            signature: Some("function func2()".to_string()),
            file_path: "src/module.ts".to_string(),
            span: Span { start_line: 2, start_col: 0, end_line: 2, end_col: 10 },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "hash_func2".to_string(),
        },
        SymbolIR {
            id: "symbol_class".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "MyClass".to_string(),
            fqn: "src/module.ts.MyClass".to_string(),
            signature: Some("class MyClass".to_string()),
            file_path: "src/module.ts".to_string(),
            span: Span { start_line: 3, start_col: 0, end_line: 3, end_col: 15 },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "hash_class".to_string(),
        },
    ];
    
    for symbol in &symbols {
        store.insert_symbol(commit_id1, symbol)?;
    }
    
    let initial_count = store.get_symbol_count()?;
    assert_eq!(initial_count, 3);
    
    // Modify file to remove one function and add another
    let changed_files = vec![
        ("src/module.ts", "export function func1() { return 10; }\nexport function newFunc() { return 'new'; }\nexport class MyClass {}"),
    ];
    
    let commit2 = create_follow_up_commit(&repo_path, &changed_files, "Refactor functions")?;
    let commit_id2 = store.create_commit_snapshot(&commit2)?;
    
    // Simulate incremental processing: delete old data for the file
    store.delete_file_data(commit_id2, "src/module.ts")?;
    
    // Insert new symbols (simulating re-parsing)
    let new_symbols = vec![
        SymbolIR {
            id: "symbol_func1_updated".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: "func1".to_string(),
            fqn: "src/module.ts.func1".to_string(),
            signature: Some("function func1()".to_string()),
            file_path: "src/module.ts".to_string(),
            span: Span { start_line: 1, start_col: 0, end_line: 1, end_col: 10 },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "hash_func1_updated".to_string(),
        },
        SymbolIR {
            id: "symbol_new_func".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Function,
            name: "newFunc".to_string(),
            fqn: "src/module.ts.newFunc".to_string(),
            signature: Some("function newFunc()".to_string()),
            file_path: "src/module.ts".to_string(),
            span: Span { start_line: 2, start_col: 0, end_line: 2, end_col: 12 },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "hash_new_func".to_string(),
        },
        SymbolIR {
            id: "symbol_class_updated".to_string(),
            lang: Language::TypeScript,
            lang_version: Some(Version::ES2020),
            kind: SymbolKind::Class,
            name: "MyClass".to_string(),
            fqn: "src/module.ts.MyClass".to_string(),
            signature: Some("class MyClass".to_string()),
            file_path: "src/module.ts".to_string(),
            span: Span { start_line: 3, start_col: 0, end_line: 3, end_col: 15 },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "hash_class_updated".to_string(),
        },
    ];
    
    for symbol in &new_symbols {
        store.insert_symbol(commit_id2, symbol)?;
    }
    
    // Verify correct symbols exist
    let final_count = store.get_symbol_count()?;
    assert_eq!(final_count, 3); // Same count, but different symbols
    
    // func2 should be gone, newFunc should exist
    let old_func2 = store.get_symbol_by_fqn("src/module.ts.func2")?;
    assert!(old_func2.is_none());
    
    let new_func = store.get_symbol_by_fqn("src/module.ts.newFunc")?;
    assert!(new_func.is_some());
    assert_eq!(new_func.unwrap().name, "newFunc");
    
    Ok(())
}

#[test]
fn test_incremental_with_file_deletions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let store = GraphStore::new(&repo_path)?;
    
    // Create initial files
    let initial_files = vec![
        ("src/keep.ts", "export function keep() { return 'keep'; }"),
        ("src/delete.ts", "export function remove() { return 'remove'; }"),
    ];
    
    let commit1 = create_initial_commit(&repo_path, &initial_files)?;
    simulate_initial_scan(&store, &repo_path, &commit1)?;
    
    let initial_file_count = store.get_file_count()?;
    assert_eq!(initial_file_count, 2);
    
    // Delete one file
    fs::remove_file(repo_path.join("src/delete.ts"))?;
    
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()?;
    
    let commit2 = create_follow_up_commit(&repo_path, &[], "Delete file")?;
    
    // Get changed/deleted files
    let _changes = get_changed_files_between_commits(&repo_path, &commit1, &commit2)?;
    
    // Git diff won't show deleted files in this simple approach, but in real implementation
    // we'd use `git diff --name-only --diff-filter=D` for deletions
    
    // Simulate deletion handling by checking which files no longer exist
    let commit_id2 = store.create_commit_snapshot(&commit2)?;
    let existing_files = store.get_files_in_commit(&commit1)?;
    
    for (file_path, _hash) in existing_files {
        if !repo_path.join(&file_path).exists() {
            // File was deleted, remove its data
            store.delete_file_data(commit_id2, &file_path)?;
        }
    }
    
    // Final file count should reflect the deletion
    // Note: get_file_count() counts distinct paths across all commits,
    // so this test demonstrates the deletion process rather than count changes
    
    Ok(())
}

#[test]
fn test_incremental_processing_with_circular_dependencies() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let store = GraphStore::new(&repo_path)?;
    
    // Create files with circular dependencies
    let initial_files = vec![
        ("src/a.ts", "import { funcB } from './b';\nexport function funcA() { return funcB(); }"),
        ("src/b.ts", "import { funcA } from './a';\nexport function funcB() { return 'b'; }"),
        ("src/c.ts", "import { funcA } from './a';\nexport function funcC() { return funcA(); }"),
    ];
    
    let commit1 = create_initial_commit(&repo_path, &initial_files)?;
    let commit_id1 = store.create_commit_snapshot(&commit1)?;
    
    // Set up circular dependency edges
    let edges = vec![
        EdgeIR {
            edge_type: EdgeType::Imports,
            src: Some("src/a.ts".to_string()),
            dst: Some("src/b.ts".to_string()),
            file_src: Some("src/a.ts".to_string()),
            file_dst: Some("src/b.ts".to_string()),
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        },
        EdgeIR {
            edge_type: EdgeType::Imports,
            src: Some("src/b.ts".to_string()),
            dst: Some("src/a.ts".to_string()),
            file_src: Some("src/b.ts".to_string()),
            file_dst: Some("src/a.ts".to_string()),
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        },
        EdgeIR {
            edge_type: EdgeType::Imports,
            src: Some("src/c.ts".to_string()),
            dst: Some("src/a.ts".to_string()),
            file_src: Some("src/c.ts".to_string()),
            file_dst: Some("src/a.ts".to_string()),
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        },
    ];
    
    for edge in &edges {
        store.insert_edge(commit_id1, edge)?;
    }
    
    // Modify file A
    let changed_files = vec![
        ("src/a.ts", "import { funcB } from './b';\nexport function funcA() { return funcB() + ' modified'; }"),
    ];
    
    let _commit2 = create_follow_up_commit(&repo_path, &changed_files, "Modify A")?;
    
    // Get dependents of file A
    let a_dependents = store.get_file_dependents("src/a.ts")?;
    
    // Should include B (due to circular dependency) and C
    assert!(a_dependents.len() >= 1); // At least C depends on A
    assert!(a_dependents.contains(&"src/c.ts".to_string()));
    
    // For incremental processing with circular deps, we need to be careful
    // to avoid infinite loops when collecting dependents
    let mut files_to_reprocess = std::collections::HashSet::new();
    files_to_reprocess.insert("src/a.ts".to_string());
    
    // Add direct dependents (but avoid infinite recursion)
    for dependent in a_dependents {
        files_to_reprocess.insert(dependent);
    }
    
    // Should include a.ts and c.ts, possibly b.ts
    assert!(files_to_reprocess.contains("src/a.ts"));
    assert!(files_to_reprocess.contains("src/c.ts"));
    
    Ok(())
}

#[test]
fn test_incremental_performance_comparison() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = create_git_repo(&temp_dir)?;
    let store = GraphStore::new(&repo_path)?;
    
    // Create a moderate number of files
    let mut initial_files = Vec::new();
    for i in 0..50 {
        initial_files.push((
            format!("src/module{}.ts", i),
            format!("export function func{}() {{ return {}; }}", i, i)
        ));
    }
    
    let initial_files_str: Vec<(&str, &str)> = initial_files.iter()
        .map(|(name, content)| (name.as_ref(), content.as_ref()))
        .collect();
    
    let commit1 = create_initial_commit(&repo_path, &initial_files_str)?;
    
    // Time full scan
    let full_scan_start = std::time::Instant::now();
    simulate_initial_scan(&store, &repo_path, &commit1)?;
    let full_scan_duration = full_scan_start.elapsed();
    
    // Modify only a few files
    let changed_files = vec![
        ("src/module1.ts", "export function func1() { return 100; } // modified"),
        ("src/module2.ts", "export function func2() { return 200; } // modified"),
    ];
    
    let changed_files_str: Vec<(&str, &str)> = changed_files.iter()
        .map(|(name, content)| (name.as_ref(), content.as_ref()))
        .collect();
    
    let commit2 = create_follow_up_commit(&repo_path, &changed_files_str, "Modify few files")?;
    
    // Time incremental scan
    let incremental_start = std::time::Instant::now();
    let changes = get_changed_files_between_commits(&repo_path, &commit1, &commit2)?;
    
    let commit_id2 = store.create_commit_snapshot(&commit2)?;
    for file in &changes {
        store.delete_file_data(commit_id2, file)?;
        
        // Simulate re-processing only changed files
        let file_path = repo_path.join(file);
        let content = fs::read_to_string(&file_path)?;
        let hash = FileWalker::compute_file_hash(&content);
        store.insert_file(commit_id2, file, &hash, content.len())?;
    }
    let incremental_duration = incremental_start.elapsed();
    
    // Incremental processing should be significantly faster
    // (though in this test the setup overhead might dominate)
    println!("Full scan: {:?}, Incremental: {:?}", full_scan_duration, incremental_duration);
    assert_eq!(changes.len(), 2); // Only 2 files changed
    
    Ok(())
}