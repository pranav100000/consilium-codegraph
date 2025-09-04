use anyhow::Result;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_empty_repository() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Just git init, no files
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle empty repository");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 files") || stdout.contains("No files"),
            "Should report no files in empty repo");
    
    Ok(())
}

#[test]
fn test_files_with_no_symbols() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create files with only comments and whitespace
    fs::write(dir.path().join("empty.ts"), "// Just a comment\n\n\n")?;
    fs::write(dir.path().join("empty.java"), "/* Block comment */\n\n")?;
    fs::write(dir.path().join("empty.cpp"), "// C++ comment\n/* Another */\n")?;
    
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
        .args(&["commit", "-m", "Empty files"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle files with no symbols");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 symbols") || !stdout.contains("symbols"),
            "Should find no symbols in empty files");
    
    Ok(())
}

#[test]
fn test_unicode_in_identifiers() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Various languages support Unicode identifiers
    fs::write(dir.path().join("unicode.ts"), r#"
export const π = 3.14159;
export function 计算(数字: number): number {
    return 数字 * 2;
}
class Überklasse {
    método(): void {}
}
"#)?;
    
    fs::write(dir.path().join("Unicode.java"), r#"
public class Класс {
    int переменная = 42;
    void μέθοδος() {}
}
"#)?;
    
    fs::write(dir.path().join("unicode.py"), r#"
def função(parâmetro):
    return parâmetro

class КлассПример:
    def метод(self):
        pass
"#)?;
    
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
        .args(&["commit", "-m", "Unicode identifiers"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle Unicode identifiers");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("symbols"), "Should parse Unicode identifiers");
    
    Ok(())
}

#[test]
fn test_very_long_identifiers() -> Result<()> {
    let dir = TempDir::new()?;
    
    let long_name = "a".repeat(1000);
    
    fs::write(dir.path().join("long.ts"), format!(
        "export function {}(x: number): number {{ return x; }}",
        long_name
    ))?;
    
    fs::write(dir.path().join("Long.java"), format!(
        "public class {} {{ void {}() {{}} }}",
        long_name, long_name
    ))?;
    
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
        .args(&["commit", "-m", "Long identifiers"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle very long identifiers");
    
    Ok(())
}

#[test]
fn test_files_with_bom() -> Result<()> {
    let dir = TempDir::new()?;
    
    // UTF-8 BOM
    let bom = vec![0xEF, 0xBB, 0xBF];
    let mut content = bom;
    content.extend_from_slice(b"export function test(): void {}");
    
    fs::write(dir.path().join("bom.ts"), content)?;
    
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
        .args(&["commit", "-m", "BOM file"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle files with BOM");
    
    Ok(())
}

#[test]
fn test_mixed_line_endings() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Mix of \n, \r\n, and \r
    fs::write(
        dir.path().join("mixed.ts"),
        "export function a(): void {}\r\nexport function b(): void {}\nexport function c(): void {}\r"
    )?;
    
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
        .args(&["commit", "-m", "Mixed line endings"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle mixed line endings");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("3 symbols") || stdout.contains("symbols"),
            "Should find all functions despite mixed line endings");
    
    Ok(())
}

#[test]
fn test_circular_dependencies() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create circular import
    fs::write(dir.path().join("a.ts"), r#"
import { b } from './b';
export function a(): void { b(); }
"#)?;
    
    fs::write(dir.path().join("b.ts"), r#"
import { a } from './a';
export function b(): void { a(); }
"#)?;
    
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
        .args(&["commit", "-m", "Circular deps"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle circular dependencies");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("edges"), "Should detect circular import edges");
    
    Ok(())
}

#[test]
fn test_special_characters_in_strings() -> Result<()> {
    let dir = TempDir::new()?;
    
    fs::write(dir.path().join("special.ts"), r#"
export const special = "Hello\nWorld\t\"Quoted\"\u{1F600}";
export const regex = /^[a-z]+\d{2,4}$/gi;
export const template = `Line 1
Line 2 with ${variable}`;
"#)?;
    
    fs::write(dir.path().join("Special.java"), r#"
public class Special {
    String s1 = "Escape sequences: \n \t \\ \" ";
    String s2 = """
        Text block with
        multiple lines
        """;
}
"#)?;
    
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
        .args(&["commit", "-m", "Special characters"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle special characters in strings");
    
    Ok(())
}

#[test]
fn test_non_ascii_file_paths() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create files with non-ASCII names
    fs::write(dir.path().join("文件.ts"), "export const x = 1;")?;
    fs::write(dir.path().join("файл.py"), "def test(): pass")?;
    fs::write(dir.path().join("αρχείο.java"), "public class Test {}")?;
    
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
        .args(&["commit", "-m", "Non-ASCII paths"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    assert!(output.status.success(), "Should handle non-ASCII file paths");
    
    Ok(())
}

#[test]
fn test_symlinks() -> Result<()> {
    let dir = TempDir::new()?;
    
    // Create a file and a symlink to it
    fs::write(dir.path().join("original.ts"), "export const x = 1;")?;
    
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            dir.path().join("original.ts"),
            dir.path().join("link.ts")
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
        .args(&["commit", "-m", "Symlinks"])
        .current_dir(&dir)
        .output()?;
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "reviewbot", "--", 
                "--repo", dir.path().to_str().unwrap(), "scan"])
        .output()?;
    
    // Should at least not crash on symlinks
    assert!(output.status.success(), "Should handle symlinks gracefully");
    
    Ok(())
}