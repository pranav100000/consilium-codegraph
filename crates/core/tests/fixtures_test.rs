use anyhow::Result;
use std::path::PathBuf;
use store::GraphStore;
use tempfile::TempDir;

fn setup_test_repo() -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("fixtures");
    
    // Copy fixtures to temp directory
    for lang in &["typescript", "python", "go"] {
        let src = fixtures_path.join(lang);
        let dst = temp_dir.path().join(lang);
        std::fs::create_dir_all(&dst)?;
        
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let file_name = entry.file_name();
            std::fs::copy(entry.path(), dst.join(file_name))?;
        }
    }
    
    let repo_path = temp_dir.path().to_path_buf();
    Ok((temp_dir, repo_path))
}

#[test]
fn test_typescript_parsing() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;
    
    // Run scan
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_reviewbot"))
        .arg("scan")
        .arg("--repo")
        .arg(&repo_path)
        .output()?;
    
    assert!(output.status.success(), "Scan failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Check results
    let store = GraphStore::new(&repo_path)?;
    
    // Verify TypeScript symbols were found
    let symbols = store.search_symbols("UserService", 10)?;
    assert!(!symbols.is_empty(), "UserService not found");
    
    let symbols = store.search_symbols("IUser", 10)?;
    assert!(!symbols.is_empty(), "IUser interface not found");
    
    let symbols = store.search_symbols("BaseService", 10)?;
    assert!(!symbols.is_empty(), "BaseService class not found");
    
    let symbols = store.search_symbols("generateUsers", 10)?;
    assert!(!symbols.is_empty(), "generateUsers async generator not found");
    
    Ok(())
}

#[test]
fn test_python_parsing() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;
    
    // Run scan
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_reviewbot"))
        .arg("scan")
        .arg("--repo")
        .arg(&repo_path)
        .output()?;
    
    assert!(output.status.success());
    
    // Check results
    let store = GraphStore::new(&repo_path)?;
    
    // Verify Python symbols were found
    let symbols = store.search_symbols("UserService", 10)?;
    assert!(!symbols.is_empty(), "Python UserService not found");
    
    let symbols = store.search_symbols("User", 10)?;
    assert!(!symbols.is_empty(), "Python User dataclass not found");
    
    let symbols = store.search_symbols("BaseService", 10)?;
    assert!(!symbols.is_empty(), "Python BaseService ABC not found");
    
    let symbols = store.search_symbols("deprecated", 10)?;
    assert!(!symbols.is_empty(), "deprecated decorator not found");
    
    let symbols = store.search_symbols("ConfigManager", 10)?;
    assert!(!symbols.is_empty(), "ConfigManager metaclass not found");
    
    Ok(())
}

#[test]
fn test_go_parsing() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;
    
    // Run scan
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_reviewbot"))
        .arg("scan")
        .arg("--repo")
        .arg(&repo_path)
        .output()?;
    
    assert!(output.status.success());
    
    // Check results
    let store = GraphStore::new(&repo_path)?;
    
    // Verify Go symbols were found
    let symbols = store.search_symbols("UserService", 10)?;
    assert!(!symbols.is_empty(), "Go UserService not found");
    
    let symbols = store.search_symbols("User", 10)?;
    assert!(!symbols.is_empty(), "Go User struct not found");
    
    let symbols = store.search_symbols("Cache", 10)?;
    assert!(!symbols.is_empty(), "Go generic Cache not found");
    
    let symbols = store.search_symbols("Cacheable", 10)?;
    assert!(!symbols.is_empty(), "Cacheable interface not found");
    
    let symbols = store.search_symbols("ProcessUsersAsync", 10)?;
    assert!(!symbols.is_empty(), "ProcessUsersAsync method not found");
    
    Ok(())
}

#[test]
fn test_graph_building() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;
    
    // Run scan
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_reviewbot"))
        .arg("scan")
        .arg("--repo")
        .arg(&repo_path)
        .output()?;
    
    assert!(output.status.success());
    
    // Build graph
    let store = GraphStore::new(&repo_path)?;
    let graph = store.build_graph()?;
    let stats = graph.stats();
    
    // Should have parsed all the symbols
    assert!(stats.node_count > 50, "Expected more than 50 symbols, got {}", stats.node_count);
    
    // Should have found relationships
    assert!(stats.edge_count > 10, "Expected more than 10 edges, got {}", stats.edge_count);
    
    Ok(())
}

#[test]
fn test_incremental_scan() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;
    
    // First scan
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_reviewbot"))
        .arg("scan")
        .arg("--repo")
        .arg(&repo_path)
        .output()?;
    
    assert!(output.status.success());
    let initial_output = String::from_utf8_lossy(&output.stdout);
    
    // Second scan (should be incremental)
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_reviewbot"))
        .arg("scan")
        .arg("--repo")
        .arg(&repo_path)
        .output()?;
    
    assert!(output.status.success());
    let second_output = String::from_utf8_lossy(&output.stdout);
    
    // Should report no changes
    assert!(
        second_output.contains("Repository unchanged") || 
        second_output.contains("0 files"),
        "Expected incremental scan to find no changes"
    );
    
    Ok(())
}