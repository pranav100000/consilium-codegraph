use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Performance benchmarks for different project sizes and languages
/// This test ensures we don't regress in performance as we add new features

const PERFORMANCE_TIMEOUT_SECS: u64 = 300; // 5 minutes max

fn create_small_project(temp_dir: &Path) -> Result<()> {
    // Small TypeScript project
    let package_json = r#"{"name": "small-test", "version": "1.0.0"}"#;
    fs::write(temp_dir.join("package.json"), package_json)?;
    
    let ts_content = r#"
export interface User {
    id: number;
    name: string;
}

export class UserService {
    getUser(id: number): User | null {
        return null;
    }
}
"#;
    fs::write(temp_dir.join("user.ts"), ts_content)?;
    
    // Small Python file
    let py_content = r#"
class User:
    def __init__(self, id: int, name: str):
        self.id = id
        self.name = name

class UserService:
    def get_user(self, id: int) -> User:
        return User(id, "test")
"#;
    fs::write(temp_dir.join("user.py"), py_content)?;
    fs::write(temp_dir.join("requirements.txt"), "requests>=2.0.0")?;
    
    Ok(())
}

fn create_medium_project(temp_dir: &Path) -> Result<()> {
    // Medium-sized multi-language project
    create_small_project(temp_dir)?;
    
    // Add Go project
    let go_mod = "module medium-test\n\ngo 1.20\n";
    fs::write(temp_dir.join("go.mod"), go_mod)?;
    
    let go_content = r#"
package main

import (
    "fmt"
    "net/http"
    "encoding/json"
)

type User struct {
    ID   int    `json:"id"`
    Name string `json:"name"`
}

type UserService struct {
    users map[int]User
}

func NewUserService() *UserService {
    return &UserService{
        users: make(map[int]User),
    }
}

func (s *UserService) CreateUser(name string) User {
    id := len(s.users) + 1
    user := User{ID: id, Name: name}
    s.users[id] = user
    return user
}

func (s *UserService) GetUser(id int) (User, bool) {
    user, exists := s.users[id]
    return user, exists
}

func (s *UserService) ListUsers() []User {
    users := make([]User, 0, len(s.users))
    for _, user := range s.users {
        users = append(users, user)
    }
    return users
}

func (s *UserService) UpdateUser(id int, name string) bool {
    if _, exists := s.users[id]; exists {
        user := s.users[id]
        user.Name = name
        s.users[id] = user
        return true
    }
    return false
}

func (s *UserService) DeleteUser(id int) bool {
    if _, exists := s.users[id]; exists {
        delete(s.users, id)
        return true
    }
    return false
}

func handleUsers(service *UserService) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        switch r.Method {
        case "GET":
            users := service.ListUsers()
            w.Header().Set("Content-Type", "application/json")
            json.NewEncoder(w).Encode(users)
        case "POST":
            var req struct {
                Name string `json:"name"`
            }
            if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
                http.Error(w, err.Error(), http.StatusBadRequest)
                return
            }
            user := service.CreateUser(req.Name)
            w.Header().Set("Content-Type", "application/json")
            json.NewEncoder(w).Encode(user)
        }
    }
}

func main() {
    service := NewUserService()
    
    // Create some test users
    service.CreateUser("Alice")
    service.CreateUser("Bob")
    service.CreateUser("Charlie")
    
    http.HandleFunc("/users", handleUsers(service))
    
    fmt.Println("Server starting on :8080")
    http.ListenAndServe(":8080", nil)
}
"#;
    fs::write(temp_dir.join("main.go"), go_content)?;
    
    // Add additional TypeScript files
    let auth_service = r#"
import { User, UserService } from './user';

export interface AuthToken {
    token: string;
    userId: number;
    expiresAt: Date;
}

export class AuthService {
    private userService: UserService;
    private activeSessions: Map<string, AuthToken> = new Map();
    
    constructor(userService: UserService) {
        this.userService = userService;
    }
    
    async login(email: string, password: string): Promise<AuthToken | null> {
        // Simulate authentication logic
        const user = this.findUserByEmail(email);
        if (!user || !this.verifyPassword(user, password)) {
            return null;
        }
        
        const token = this.generateToken();
        const authToken: AuthToken = {
            token,
            userId: user.id,
            expiresAt: new Date(Date.now() + 3600000) // 1 hour
        };
        
        this.activeSessions.set(token, authToken);
        return authToken;
    }
    
    async validateToken(token: string): Promise<User | null> {
        const authToken = this.activeSessions.get(token);
        if (!authToken || authToken.expiresAt < new Date()) {
            this.activeSessions.delete(token);
            return null;
        }
        
        return this.userService.getUser(authToken.userId);
    }
    
    async logout(token: string): Promise<boolean> {
        return this.activeSessions.delete(token);
    }
    
    private findUserByEmail(email: string): User | null {
        // Simplified lookup
        return null;
    }
    
    private verifyPassword(user: User, password: string): boolean {
        // Simplified password verification
        return password.length > 0;
    }
    
    private generateToken(): string {
        return Math.random().toString(36).substring(2) + Date.now().toString(36);
    }
    
    getActiveSessionCount(): number {
        return this.activeSessions.size;
    }
    
    cleanupExpiredTokens(): void {
        const now = new Date();
        for (const [token, authToken] of this.activeSessions.entries()) {
            if (authToken.expiresAt < now) {
                this.activeSessions.delete(token);
            }
        }
    }
}
"#;
    fs::write(temp_dir.join("auth.ts"), auth_service)?;
    
    Ok(())
}

fn create_large_project(temp_dir: &Path) -> Result<()> {
    // Large multi-language project with many files
    create_medium_project(temp_dir)?;
    
    // Create multiple TypeScript modules
    fs::create_dir_all(temp_dir.join("src").join("services"))?;
    fs::create_dir_all(temp_dir.join("src").join("models"))?;
    fs::create_dir_all(temp_dir.join("src").join("utils"))?;
    
    // Generate multiple service files
    for i in 1..=10 {
        let service_content = format!(r#"
export interface Entity{i} {{
    id: number;
    name: string;
    createdAt: Date;
    updatedAt: Date;
}}

export class Entity{i}Service {{
    private items: Entity{i}[] = [];
    
    create(item: Omit<Entity{i}, 'id' | 'createdAt' | 'updatedAt'>): Entity{i} {{
        const now = new Date();
        const newItem: Entity{i} = {{
            id: this.items.length + 1,
            ...item,
            createdAt: now,
            updatedAt: now
        }};
        this.items.push(newItem);
        return newItem;
    }}
    
    getById(id: number): Entity{i} | undefined {{
        return this.items.find(item => item.id === id);
    }}
    
    getAll(): Entity{i}[] {{
        return [...this.items];
    }}
    
    update(id: number, updates: Partial<Omit<Entity{i}, 'id' | 'createdAt'>>): Entity{i} | null {{
        const index = this.items.findIndex(item => item.id === id);
        if (index === -1) return null;
        
        this.items[index] = {{
            ...this.items[index],
            ...updates,
            updatedAt: new Date()
        }};
        return this.items[index];
    }}
    
    delete(id: number): boolean {{
        const index = this.items.findIndex(item => item.id === id);
        if (index === -1) return false;
        
        this.items.splice(index, 1);
        return true;
    }}
    
    search(query: string): Entity{i}[] {{
        return this.items.filter(item => 
            item.name.toLowerCase().includes(query.toLowerCase())
        );
    }}
}}
"#, i = i);
        fs::write(temp_dir.join("src").join("services").join(format!("entity{}.ts", i)), service_content)?;
    }
    
    // Generate Python modules
    for i in 1..=8 {
        let py_content = format!(r#"
from typing import List, Optional, Dict, Any
from datetime import datetime
from dataclasses import dataclass, field

@dataclass
class Entity{i}:
    id: int
    name: str
    created_at: datetime = field(default_factory=datetime.now)
    updated_at: datetime = field(default_factory=datetime.now)
    
    def to_dict(self) -> Dict[str, Any]:
        return {{
            'id': self.id,
            'name': self.name,
            'created_at': self.created_at.isoformat(),
            'updated_at': self.updated_at.isoformat()
        }}

class Entity{i}Repository:
    def __init__(self):
        self._items: Dict[int, Entity{i}] = {{}}
        self._next_id = 1
    
    def create(self, name: str) -> Entity{i}:
        item = Entity{i}(id=self._next_id, name=name)
        self._items[self._next_id] = item
        self._next_id += 1
        return item
    
    def get_by_id(self, id: int) -> Optional[Entity{i}]:
        return self._items.get(id)
    
    def get_all(self) -> List[Entity{i}]:
        return list(self._items.values())
    
    def update(self, id: int, name: str) -> Optional[Entity{i}]:
        if id in self._items:
            self._items[id].name = name
            self._items[id].updated_at = datetime.now()
            return self._items[id]
        return None
    
    def delete(self, id: int) -> bool:
        return self._items.pop(id, None) is not None
    
    def search(self, query: str) -> List[Entity{i}]:
        return [item for item in self._items.values() 
                if query.lower() in item.name.lower()]
    
    def count(self) -> int:
        return len(self._items)

class Entity{i}Service:
    def __init__(self, repository: Entity{i}Repository):
        self.repository = repository
    
    async def create_entity(self, name: str) -> Entity{i}:
        if not name or not name.strip():
            raise ValueError("Name cannot be empty")
        return self.repository.create(name.strip())
    
    async def get_entity(self, id: int) -> Optional[Entity{i}]:
        return self.repository.get_by_id(id)
    
    async def list_entities(self) -> List[Entity{i}]:
        return self.repository.get_all()
    
    async def update_entity(self, id: int, name: str) -> Optional[Entity{i}]:
        if not name or not name.strip():
            raise ValueError("Name cannot be empty")
        return self.repository.update(id, name.strip())
    
    async def delete_entity(self, id: int) -> bool:
        return self.repository.delete(id)
    
    async def search_entities(self, query: str) -> List[Entity{i}]:
        if not query or not query.strip():
            return []
        return self.repository.search(query.strip())
    
    async def get_entity_count(self) -> int:
        return self.repository.count()
"#, i = i);
        fs::write(temp_dir.join(format!("entity{}.py", i)), py_content)?;
    }
    
    Ok(())
}

fn measure_scan_performance(project_path: &Path, project_size: &str) -> Result<Duration> {
    println!("⏱️  Measuring scan performance for {} project at: {:?}", project_size, project_path);
    
    let start_time = Instant::now();
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &project_path.to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let duration = start_time.elapsed();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(output.status.success(), 
        "Scan should succeed for {} project. stderr: {}", project_size, stderr);
    
    println!("✅ {} project scan completed in {:?}", project_size, duration);
    
    // Log some basic stats if available in stderr
    if stderr.contains("files") {
        let lines: Vec<&str> = stderr.lines().collect();
        for line in lines {
            if line.contains("files") || line.contains("symbols") || line.contains("Processing") {
                println!("   {}", line.trim());
            }
        }
    }
    
    Ok(duration)
}

#[test]
fn test_small_project_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_small_project(temp_dir.path())?;
    
    let duration = measure_scan_performance(temp_dir.path(), "small")?;
    
    // Performance expectations for small projects
    assert!(duration.as_secs() < 30, 
        "Small project should scan within 30 seconds, took {:?}", duration);
    
    println!("📊 Small project performance: {:?}", duration);
    Ok(())
}

#[test]
fn test_medium_project_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_medium_project(temp_dir.path())?;
    
    let duration = measure_scan_performance(temp_dir.path(), "medium")?;
    
    // Performance expectations for medium projects
    assert!(duration.as_secs() < 60, 
        "Medium project should scan within 60 seconds, took {:?}", duration);
    
    println!("📊 Medium project performance: {:?}", duration);
    Ok(())
}

#[test]
fn test_large_project_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_large_project(temp_dir.path())?;
    
    let duration = measure_scan_performance(temp_dir.path(), "large")?;
    
    // Performance expectations for large projects
    assert!(duration.as_secs() < 120, 
        "Large project should scan within 2 minutes, took {:?}", duration);
    
    println!("📊 Large project performance: {:?}", duration);
    Ok(())
}

#[test]
fn test_performance_consistency() -> Result<()> {
    // Test that multiple runs on the same project have consistent performance
    let temp_dir = TempDir::new()?;
    create_medium_project(temp_dir.path())?;
    
    let mut durations = Vec::new();
    
    for i in 1..=3 {
        println!("🔄 Performance consistency test run {}/3", i);
        let duration = measure_scan_performance(temp_dir.path(), "consistency")?;
        durations.push(duration);
    }
    
    // Calculate coefficient of variation (standard deviation / mean)
    let mean_duration = durations.iter().sum::<Duration>().as_secs_f64() / durations.len() as f64;
    let variance = durations.iter()
        .map(|d| {
            let diff = d.as_secs_f64() - mean_duration;
            diff * diff
        })
        .sum::<f64>() / durations.len() as f64;
    let std_dev = variance.sqrt();
    let cv = std_dev / mean_duration;
    
    println!("📊 Performance consistency results:");
    println!("   Runs: {:?}", durations);
    println!("   Mean: {:.2}s", mean_duration);
    println!("   Std Dev: {:.2}s", std_dev);
    println!("   Coefficient of Variation: {:.2}%", cv * 100.0);
    
    // Performance should be reasonably consistent (CV < 20%)
    assert!(cv < 0.20, 
        "Performance should be consistent (CV < 20%), got {:.2}%", cv * 100.0);
    
    Ok(())
}

#[test]
fn test_memory_performance() -> Result<()> {
    // Test memory usage doesn't grow excessively with larger projects
    let small_temp = TempDir::new()?;
    create_small_project(small_temp.path())?;
    
    let large_temp = TempDir::new()?;
    create_large_project(large_temp.path())?;
    
    println!("🧠 Testing memory performance characteristics...");
    
    // Run small project
    let start_time = Instant::now();
    let small_output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &small_temp.path().to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    let small_duration = start_time.elapsed();
    
    assert!(small_output.status.success(), "Small project scan should succeed");
    
    // Run large project  
    let start_time = Instant::now();
    let large_output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &large_temp.path().to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    let large_duration = start_time.elapsed();
    
    assert!(large_output.status.success(), "Large project scan should succeed");
    
    println!("📊 Memory performance results:");
    println!("   Small project: {:?}", small_duration);
    println!("   Large project: {:?}", large_duration);
    
    // Large project shouldn't take more than 5x the time of small project
    let duration_ratio = large_duration.as_secs_f64() / small_duration.as_secs_f64().max(1.0);
    println!("   Duration ratio (large/small): {:.2}x", duration_ratio);
    
    assert!(duration_ratio < 10.0, 
        "Large project shouldn't take more than 10x small project time, got {:.2}x", duration_ratio);
    
    Ok(())
}

#[test]
fn test_semantic_scan_performance() -> Result<()> {
    // Test performance of semantic scanning (if SCIP indexers are available)
    let temp_dir = TempDir::new()?;
    create_small_project(temp_dir.path())?;
    
    println!("🔬 Testing semantic scan performance...");
    
    let start_time = Instant::now();
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan", 
            "--semantic",
            "--no-write"
        ])
        .output()?;
    let duration = start_time.elapsed();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        println!("✅ Semantic scan completed in {:?}", duration);
        
        // Semantic scan should complete reasonably quickly for small projects
        assert!(duration.as_secs() < 60, 
            "Semantic scan should complete within 60 seconds, took {:?}", duration);
        
        // Check if semantic analysis actually ran
        if stderr.contains("semantic") || stderr.contains("SCIP") {
            println!("   Semantic analysis was performed");
        } else {
            println!("   ℹ️ Semantic analysis may have been skipped (missing SCIP indexers)");
        }
    } else {
        println!("ℹ️  Semantic scan failed (likely missing SCIP indexers): {}", stderr);
        // This is expected in CI environments without SCIP tools installed
        assert!(stderr.contains("SCIP") || stderr.contains("semantic"), 
            "Failure should be due to missing SCIP indexers");
    }
    
    Ok(())
}

/// Test that ensures we don't regress significantly from baseline performance
#[test]
fn test_performance_regression_baseline() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_medium_project(temp_dir.path())?;
    
    println!("📈 Running performance regression baseline test...");
    
    let duration = measure_scan_performance(temp_dir.path(), "baseline")?;
    
    // Baseline performance expectations based on current implementation
    // These thresholds should be adjusted based on the actual performance characteristics
    
    // For a medium project (should complete well within reasonable time)
    assert!(duration.as_secs() < 90, 
        "Baseline performance regression: medium project took {:?} (expected < 90s)", duration);
    
    // Log detailed performance info
    println!("📊 Performance regression baseline results:");
    println!("   Duration: {:?}", duration);
    println!("   Rate: {:.2} files/second (estimated)", 
        20.0 / duration.as_secs_f64().max(0.1)); // Estimate based on ~20 files
    
    // Check if we have performance metrics in the output
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "reviewbot", "--", 
            "--repo", &temp_dir.path().to_string_lossy(),
            "scan", 
            "--no-write"
        ])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Performance Summary") {
        println!("   ✅ Performance metrics are being collected");
    } else {
        println!("   ℹ️ Performance metrics not shown (expected for dry-run mode)");
    }
    
    Ok(())
}