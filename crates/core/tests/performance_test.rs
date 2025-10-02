use anyhow::Result;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_large_file_performance() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Generate a large TypeScript file with 5000 functions
    let mut content = String::new();
    for i in 0..5000 {
        content.push_str(&format!(
            "export function func_{i}(x: number): number {{
    const result = x + {i};
    return result * 2;
}}

",
            i = i
        ));
    }
    
    fs::write(dir.path().join("large.ts"), content)?;
    
    // Generate a large Java file with many classes
    let mut java_content = String::new();
    for i in 0..1000 {
        java_content.push_str(&format!(
            "class Class{i} {{
    private int field{i} = {i};
    
    public void method{i}() {{
        System.out.println(field{i});
    }}
}}

",
            i = i
        ));
    }
    
    fs::write(dir.path().join("large.java"), java_content)?;
    
    // Generate a large C++ file
    let mut cpp_content = String::new();
    for i in 0..1000 {
        cpp_content.push_str(&format!(
            "class Class{i} {{
public:
    int method{i}(int x) {{
        return x + {i};
    }}
private:
    int field{i} = {i};
}};

",
            i = i
        ));
    }
    
    fs::write(dir.path().join("large.cpp"), cpp_content)?;
    
    // Git init
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["commit", "-m", "Large files"])
        .current_dir(&dir)
        .output()?;
    
    // Measure scan time
    let start = Instant::now();
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--release", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    let elapsed = start.elapsed();
    
    assert!(output.status.success(), "Scan should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Performance assertions
    println!("Scan took {:?} for large files", elapsed);
    assert!(elapsed.as_secs() < 30, "Should scan large files in under 30 seconds");

    // CRITICAL: Verify actual symbol count in database, not just stdout
    use store::GraphStore;
    let store = GraphStore::new(dir.path())?;
    let symbol_count = store.get_symbol_count()?;

    // Expected: ~5000 TS functions + ~2000 Java symbols (class+method+field per class) + ~2000 C++ symbols
    // Actual varies by parser, so allow range
    assert!(symbol_count >= 6000,
        "Expected at least 6000 symbols (5000 TS + 1000 Java + 1000 C++), but found {}. \
         This indicates the parser failed to extract symbols correctly.",
        symbol_count
    );

    println!("✅ Performance test verified: {} symbols indexed in {:?}", symbol_count, elapsed);

    Ok(())
}

#[test]
fn test_incremental_performance() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create initial files
    for i in 0..100 {
        let content = format!(
            "export function func{}(x: number): number {{ return x + {}; }}",
            i, i
        );
        fs::write(dir.path().join(format!("file{}.ts", i)), content)?;
    }
    
    // Git setup
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial"])
        .current_dir(&dir)
        .output()?;
    
    // First scan
    let output1 = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output1.status.success());
    
    // Modify one file
    fs::write(
        dir.path().join("file1.ts"),
        "export function func1(x: number): number { return x * 2; }"
    )?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["commit", "-m", "Modify one file"])
        .current_dir(&dir)
        .output()?;
    
    // Measure incremental scan time
    let start = Instant::now();
    
    let output2 = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    let elapsed = start.elapsed();
    
    assert!(output2.status.success());

    // Incremental scan should be fast
    println!("Incremental scan took {:?} for 1 changed file out of 100", elapsed);
    assert!(elapsed.as_secs() < 5, "Incremental scan should be under 5 seconds");

    // Verify actual database state - only 1 file should have changed
    use store::GraphStore;
    let store = GraphStore::new(dir.path())?;
    let file_count = store.get_file_count()?;

    // Note: We check that total files is still 100, incremental processing is internal
    assert_eq!(file_count, 100,
        "Should still have 100 files total after incremental scan");

    Ok(())
}

#[test] 
fn test_concurrent_file_processing() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create files in different languages
    for i in 0..50 {
        fs::write(
            dir.path().join(format!("file{}.ts", i)),
            format!("export const x{} = {};", i, i)
        )?;
        
        fs::write(
            dir.path().join(format!("file{}.py", i)),
            format!("def func{}(): return {}", i, i)
        )?;
        
        fs::write(
            dir.path().join(format!("File{}.java", i)),
            format!("public class File{} {{ int x = {}; }}", i, i)
        )?;
        
        fs::write(
            dir.path().join(format!("file{}.cpp", i)),
            format!("int func{}() {{ return {}; }}", i, i)
        )?;
    }
    
    // Git setup
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["commit", "-m", "Many files"])
        .current_dir(&dir)
        .output()?;
    
    let start = Instant::now();
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    let elapsed = start.elapsed();
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    println!("Processing 200 files took {:?}", elapsed);
    assert!(elapsed.as_secs() < 10, "Should process 200 small files quickly");

    // Verify actual database contents
    use store::GraphStore;
    let store = GraphStore::new(dir.path())?;
    let file_count = store.get_file_count()?;

    assert_eq!(file_count, 200,
        "Should process all 200 files");

    Ok(())
}

#[test]
fn test_memory_efficiency() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create a file with deeply nested structures
    let mut content = String::from("namespace level0 {\n");
    for i in 1..100 {
        content.push_str(&format!("namespace level{} {{\n", i));
    }
    
    content.push_str("class DeepClass {\n");
    for i in 0..1000 {
        content.push_str(&format!("  int field{} = {};\n", i, i));
    }
    content.push_str("};\n");
    
    for _ in 0..100 {
        content.push_str("}\n");
    }
    
    fs::write(dir.path().join("deep.cpp"), content)?;
    
    // Git setup
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(&dir)
        .output()?;
    
    std::process::Command::new("git")
        .args(&["commit", "-m", "Deep nesting"])
        .current_dir(&dir)
        .output()?;
    
    // Should handle deep nesting without stack overflow
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle deep nesting without crashing");
    
    Ok(())
}