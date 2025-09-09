use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use reviewbot::language_strategy::*;
use protocol::Language;

fn create_test_typescript_project(temp_dir: &Path) -> Result<()> {
    let package_json = r#"{
  "name": "test-typescript-project",
  "version": "1.0.0",
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#;
    fs::write(temp_dir.join("package.json"), package_json)?;
    
    let ts_content = r#"
export interface User {
    id: number;
    name: string;
    email: string;
}

export class UserService {
    private users: User[] = [];
    
    addUser(user: User): void {
        this.users.push(user);
    }
    
    getUser(id: number): User | null {
        return this.users.find(u => u.id === id) || null;
    }
    
    getUserCount(): number {
        return this.users.length;
    }
}
"#;
    fs::write(temp_dir.join("user.ts"), ts_content)?;
    fs::write(temp_dir.join("index.js"), "console.log('Hello World');")?;
    
    Ok(())
}

fn create_test_python_project(temp_dir: &Path) -> Result<()> {
    let py_content = r#"
from typing import List, Optional
from dataclasses import dataclass

@dataclass
class User:
    id: int
    name: str
    email: str

class UserService:
    def __init__(self):
        self.users: List[User] = []
    
    def add_user(self, user: User) -> None:
        self.users.append(user)
    
    def get_user(self, user_id: int) -> Optional[User]:
        for user in self.users:
            if user.id == user_id:
                return user
        return None
    
    def get_user_count(self) -> int:
        return len(self.users)
"#;
    fs::write(temp_dir.join("user.py"), py_content)?;
    fs::write(temp_dir.join("requirements.txt"), "dataclasses>=0.6\ntyping-extensions>=4.0")?;
    
    Ok(())
}

fn create_test_go_project(temp_dir: &Path) -> Result<()> {
    let go_mod = r#"module test-go-project

go 1.20
"#;
    fs::write(temp_dir.join("go.mod"), go_mod)?;
    
    let go_content = r#"
package main

import "fmt"

type User struct {
    ID    int    `json:"id"`
    Name  string `json:"name"`
    Email string `json:"email"`
}

type UserService struct {
    users []User
}

func NewUserService() *UserService {
    return &UserService{
        users: make([]User, 0),
    }
}

func (s *UserService) AddUser(user User) {
    s.users = append(s.users, user)
}

func (s *UserService) GetUser(id int) *User {
    for _, user := range s.users {
        if user.ID == id {
            return &user
        }
    }
    return nil
}

func (s *UserService) GetUserCount() int {
    return len(s.users)
}

func main() {
    service := NewUserService()
    user := User{ID: 1, Name: "Test", Email: "test@example.com"}
    service.AddUser(user)
    fmt.Printf("User count: %d\n", service.GetUserCount())
}
"#;
    fs::write(temp_dir.join("main.go"), go_content)?;
    
    Ok(())
}

fn create_test_rust_project(temp_dir: &Path) -> Result<()> {
    let cargo_toml = r#"[package]
name = "test-rust-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
"#;
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;
    
    fs::create_dir(temp_dir.join("src"))?;
    
    let rust_content = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

pub struct UserService {
    users: Vec<User>,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            users: Vec::new(),
        }
    }
    
    pub fn add_user(&mut self, user: User) {
        self.users.push(user);
    }
    
    pub fn get_user(&self, id: u32) -> Option<&User> {
        self.users.iter().find(|u| u.id == id)
    }
    
    pub fn get_user_count(&self) -> usize {
        self.users.len()
    }
}

fn main() {
    let mut service = UserService::new();
    let user = User {
        id: 1,
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
    };
    service.add_user(user);
    println!("User count: {}", service.get_user_count());
}
"#;
    fs::write(temp_dir.join("src").join("main.rs"), rust_content)?;
    
    Ok(())
}

fn create_test_java_project(temp_dir: &Path) -> Result<()> {
    let pom_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-java-project</artifactId>
    <version>1.0-SNAPSHOT</version>
    <properties>
        <maven.compiler.source>11</maven.compiler.source>
        <maven.compiler.target>11</maven.compiler.target>
    </properties>
</project>
"#;
    fs::write(temp_dir.join("pom.xml"), pom_xml)?;
    
    fs::create_dir_all(temp_dir.join("src").join("main").join("java").join("com").join("example"))?;
    
    let java_content = r#"
package com.example;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

public class User {
    private int id;
    private String name;
    private String email;
    
    public User(int id, String name, String email) {
        this.id = id;
        this.name = name;
        this.email = email;
    }
    
    public int getId() { return id; }
    public String getName() { return name; }
    public String getEmail() { return email; }
}

class UserService {
    private List<User> users = new ArrayList<>();
    
    public void addUser(User user) {
        users.add(user);
    }
    
    public Optional<User> getUser(int id) {
        return users.stream()
            .filter(u -> u.getId() == id)
            .findFirst();
    }
    
    public int getUserCount() {
        return users.size();
    }
    
    public static void main(String[] args) {
        UserService service = new UserService();
        User user = new User(1, "Test", "test@example.com");
        service.addUser(user);
        System.out.println("User count: " + service.getUserCount());
    }
}
"#;
    fs::write(
        temp_dir.join("src").join("main").join("java").join("com").join("example").join("User.java"), 
        java_content
    )?;
    
    Ok(())
}

fn create_test_cpp_project(temp_dir: &Path) -> Result<()> {
    let cmake_lists = r#"cmake_minimum_required(VERSION 3.10)
project(TestCppProject)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(main main.cpp user.cpp)
"#;
    fs::write(temp_dir.join("CMakeLists.txt"), cmake_lists)?;
    
    let header_content = r#"#ifndef USER_H
#define USER_H

#include <string>
#include <vector>
#include <optional>

class User {
private:
    int id;
    std::string name;
    std::string email;

public:
    User(int id, const std::string& name, const std::string& email);
    
    int getId() const { return id; }
    const std::string& getName() const { return name; }
    const std::string& getEmail() const { return email; }
};

class UserService {
private:
    std::vector<User> users;

public:
    void addUser(const User& user);
    std::optional<User> getUser(int id) const;
    size_t getUserCount() const;
};

#endif // USER_H
"#;
    fs::write(temp_dir.join("user.h"), header_content)?;
    
    let cpp_content = r#"#include "user.h"
#include <algorithm>

User::User(int id, const std::string& name, const std::string& email)
    : id(id), name(name), email(email) {}

void UserService::addUser(const User& user) {
    users.push_back(user);
}

std::optional<User> UserService::getUser(int id) const {
    auto it = std::find_if(users.begin(), users.end(),
        [id](const User& u) { return u.getId() == id; });
    
    if (it != users.end()) {
        return *it;
    }
    return std::nullopt;
}

size_t UserService::getUserCount() const {
    return users.size();
}
"#;
    fs::write(temp_dir.join("user.cpp"), cpp_content)?;
    
    let main_content = r#"#include "user.h"
#include <iostream>

int main() {
    UserService service;
    User user(1, "Test", "test@example.com");
    service.addUser(user);
    std::cout << "User count: " << service.getUserCount() << std::endl;
    return 0;
}
"#;
    fs::write(temp_dir.join("main.cpp"), main_content)?;
    
    Ok(())
}

#[test]
fn test_typescript_strategy_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_typescript_project(temp_dir.path())?;
    
    let strategy = TypeScriptStrategy;
    let files = strategy.detect_files(temp_dir.path());
    
    // Should detect package.json, .ts, and .js files
    assert!(!files.is_empty(), "TypeScript strategy should detect files");
    assert!(strategy.can_handle(temp_dir.path()), "TypeScript strategy should handle the project");
    assert_eq!(strategy.language(), Language::TypeScript);
    assert_eq!(strategy.name(), "TypeScript/JavaScript");
    
    // Check that specific files are detected
    let file_names: Vec<String> = files.iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .map(|s| s.to_string())
        .collect();
    
    assert!(file_names.contains(&"package.json".to_string()));
    assert!(file_names.contains(&"user.ts".to_string()));
    assert!(file_names.contains(&"index.js".to_string()));
    
    println!("✅ TypeScript strategy detection test passed");
    Ok(())
}

#[test]
fn test_python_strategy_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_python_project(temp_dir.path())?;
    
    let strategy = PythonStrategy;
    let files = strategy.detect_files(temp_dir.path());
    
    assert!(!files.is_empty(), "Python strategy should detect files");
    assert!(strategy.can_handle(temp_dir.path()), "Python strategy should handle the project");
    assert_eq!(strategy.language(), Language::Python);
    assert_eq!(strategy.name(), "Python");
    
    let file_names: Vec<String> = files.iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .map(|s| s.to_string())
        .collect();
    
    assert!(file_names.contains(&"user.py".to_string()));
    assert!(file_names.contains(&"requirements.txt".to_string()));
    
    println!("✅ Python strategy detection test passed");
    Ok(())
}

#[test]
fn test_go_strategy_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_go_project(temp_dir.path())?;
    
    let strategy = GoStrategy;
    let files = strategy.detect_files(temp_dir.path());
    
    assert!(!files.is_empty(), "Go strategy should detect files");
    assert!(strategy.can_handle(temp_dir.path()), "Go strategy should handle the project");
    assert_eq!(strategy.language(), Language::Go);
    assert_eq!(strategy.name(), "Go");
    
    let file_names: Vec<String> = files.iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .map(|s| s.to_string())
        .collect();
    
    assert!(file_names.contains(&"main.go".to_string()));
    assert!(file_names.contains(&"go.mod".to_string()));
    
    println!("✅ Go strategy detection test passed");
    Ok(())
}

#[test]
fn test_rust_strategy_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_rust_project(temp_dir.path())?;
    
    let strategy = RustStrategy;
    let files = strategy.detect_files(temp_dir.path());
    
    assert!(!files.is_empty(), "Rust strategy should detect files");
    assert!(strategy.can_handle(temp_dir.path()), "Rust strategy should handle the project");
    assert_eq!(strategy.language(), Language::Rust);
    assert_eq!(strategy.name(), "Rust");
    
    let file_names: Vec<String> = files.iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .map(|s| s.to_string())
        .collect();
    
    assert!(file_names.contains(&"Cargo.toml".to_string()));
    assert!(file_names.iter().any(|name| name.ends_with(".rs")), "Should detect Rust source files");
    
    println!("✅ Rust strategy detection test passed");
    Ok(())
}

#[test]
fn test_java_strategy_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_java_project(temp_dir.path())?;
    
    let strategy = JavaStrategy;
    let files = strategy.detect_files(temp_dir.path());
    
    assert!(!files.is_empty(), "Java strategy should detect files");
    assert!(strategy.can_handle(temp_dir.path()), "Java strategy should handle the project");
    assert_eq!(strategy.language(), Language::Java);
    assert_eq!(strategy.name(), "Java");
    
    let file_names: Vec<String> = files.iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .map(|s| s.to_string())
        .collect();
    
    assert!(file_names.contains(&"pom.xml".to_string()));
    assert!(file_names.iter().any(|name| name.ends_with(".java")), "Should detect Java source files");
    
    println!("✅ Java strategy detection test passed");
    Ok(())
}

#[test]
fn test_cpp_strategy_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_cpp_project(temp_dir.path())?;
    
    let strategy = CppStrategy;
    let files = strategy.detect_files(temp_dir.path());
    
    assert!(!files.is_empty(), "C++ strategy should detect files");
    assert!(strategy.can_handle(temp_dir.path()), "C++ strategy should handle the project");
    assert_eq!(strategy.language(), Language::Cpp);
    assert_eq!(strategy.name(), "C++");
    
    let file_names: Vec<String> = files.iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .map(|s| s.to_string())
        .collect();
    
    assert!(file_names.contains(&"CMakeLists.txt".to_string()));
    assert!(file_names.iter().any(|name| name.ends_with(".cpp") || name.ends_with(".h")), 
            "Should detect C++ source and header files");
    
    println!("✅ C++ strategy detection test passed");
    Ok(())
}

#[test]
fn test_language_strategy_registry() -> Result<()> {
    let registry = LanguageStrategyRegistry::new();
    
    // Test multi-language project detection
    let temp_dir = TempDir::new()?;
    
    // Create a mixed-language project
    create_test_typescript_project(temp_dir.path())?;
    create_test_python_project(temp_dir.path())?;
    create_test_go_project(temp_dir.path())?;
    create_test_rust_project(temp_dir.path())?;
    
    let detected_languages = registry.detect_languages(temp_dir.path());
    
    // Should detect all languages present
    assert!(detected_languages.len() >= 4, "Should detect at least 4 languages in mixed project");
    
    let language_names: Vec<_> = detected_languages.iter()
        .map(|s| s.language())
        .collect();
    
    assert!(language_names.contains(&Language::TypeScript));
    assert!(language_names.contains(&Language::Python));
    assert!(language_names.contains(&Language::Go));
    assert!(language_names.contains(&Language::Rust));
    
    println!("✅ Language registry multi-language detection test passed");
    println!("Detected {} languages: {:?}", detected_languages.len(), language_names);
    
    Ok(())
}

#[test]
fn test_empty_project_detection() -> Result<()> {
    let registry = LanguageStrategyRegistry::new();
    let temp_dir = TempDir::new()?;
    
    // Empty directory should detect no languages
    let detected_languages = registry.detect_languages(temp_dir.path());
    assert!(detected_languages.is_empty(), "Empty project should detect no languages");
    
    println!("✅ Empty project detection test passed");
    Ok(())
}

#[test]
fn test_language_specific_file_patterns() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Test various file extensions for each language
    
    // TypeScript/JavaScript files
    fs::write(temp_dir.path().join("test.ts"), "const x: number = 1;")?;
    fs::write(temp_dir.path().join("test.tsx"), "const Component = () => <div></div>;")?;
    fs::write(temp_dir.path().join("test.js"), "const x = 1;")?;
    fs::write(temp_dir.path().join("test.jsx"), "const Component = () => <div></div>;")?;
    
    let ts_strategy = TypeScriptStrategy;
    let ts_files = ts_strategy.detect_files(temp_dir.path());
    assert!(ts_files.len() >= 4, "Should detect all TypeScript/JavaScript variants");
    
    // Clean up for next test
    fs::remove_file(temp_dir.path().join("test.ts"))?;
    fs::remove_file(temp_dir.path().join("test.tsx"))?;
    fs::remove_file(temp_dir.path().join("test.js"))?;
    fs::remove_file(temp_dir.path().join("test.jsx"))?;
    
    // Python files
    fs::write(temp_dir.path().join("test.py"), "x = 1")?;
    fs::write(temp_dir.path().join("test.pyi"), "x: int")?;
    fs::write(temp_dir.path().join("__init__.py"), "")?;
    
    let py_strategy = PythonStrategy;
    let py_files = py_strategy.detect_files(temp_dir.path());
    assert!(py_files.len() >= 3, "Should detect all Python file variants");
    
    println!("✅ Language-specific file pattern detection test passed");
    Ok(())
}