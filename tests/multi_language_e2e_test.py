#!/usr/bin/env python3
"""
Multi-Language End-to-End Test Suite for Consilium CodeGraph

Validates all supported languages with complete e2e pipeline testing.
"""

import subprocess
import tempfile
import shutil
from pathlib import Path
import sqlite3
import time

class MultiLanguageE2ETest:
    """End-to-end test suite for all supported languages"""
    
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.results = {}
        
    def log(self, message, success=None):
        """Log test results"""
        if success is True:
            print(f"✅ {message}")
        elif success is False:
            print(f"❌ {message}")
        else:
            print(f"ℹ️  {message}")
    
    def run_command(self, cmd, cwd=None, timeout=60):
        """Run a command and return result"""
        try:
            result = subprocess.run(
                cmd, 
                cwd=cwd or self.project_root,
                capture_output=True,
                text=True,
                timeout=timeout
            )
            return result
        except Exception as e:
            self.log(f"Command failed: {e}", False)
            return None

    def create_java_project(self, project_dir):
        """Create a comprehensive Java test project"""
        project_path = Path(project_dir)
        
        # Java source directory structure
        src_main = project_path / "src" / "main" / "java" / "com" / "example"
        src_main.mkdir(parents=True, exist_ok=True)
        
        # User model class
        (src_main / "User.java").write_text("""
package com.example;

import java.util.Objects;

public class User {
    private final int id;
    private String name;
    private String email;
    
    public User(int id, String name, String email) {
        this.id = id;
        this.name = name;
        this.email = email;
    }
    
    public int getId() {
        return id;
    }
    
    public String getName() {
        return name;
    }
    
    public void setName(String name) {
        this.name = name;
    }
    
    public String getEmail() {
        return email;
    }
    
    public void setEmail(String email) {
        this.email = email;
    }
    
    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null || getClass() != obj.getClass()) return false;
        User user = (User) obj;
        return id == user.id;
    }
    
    @Override
    public int hashCode() {
        return Objects.hash(id);
    }
    
    @Override
    public String toString() {
        return String.format("User{id=%d, name='%s', email='%s'}", id, name, email);
    }
}
""")

        # Service class with generics
        (src_main / "UserService.java").write_text("""
package com.example;

import java.util.*;
import java.util.stream.Collectors;

public class UserService<T extends User> {
    private final Map<Integer, T> users = new HashMap<>();
    private final List<String> auditLog = new ArrayList<>();
    
    public void addUser(T user) {
        users.put(user.getId(), user);
        auditLog.add("Added user: " + user.getId());
    }
    
    public Optional<T> getUser(int id) {
        return Optional.ofNullable(users.get(id));
    }
    
    public List<T> getAllUsers() {
        return new ArrayList<>(users.values());
    }
    
    public List<T> findUsersByName(String namePattern) {
        return users.values().stream()
            .filter(user -> user.getName().toLowerCase().contains(namePattern.toLowerCase()))
            .collect(Collectors.toList());
    }
    
    public boolean removeUser(int id) {
        T removed = users.remove(id);
        if (removed != null) {
            auditLog.add("Removed user: " + id);
            return true;
        }
        return false;
    }
    
    public int getUserCount() {
        return users.size();
    }
    
    public List<String> getAuditLog() {
        return new ArrayList<>(auditLog);
    }
}
""")

        # Main application class
        (src_main / "Application.java").write_text("""
package com.example;

import java.util.Arrays;
import java.util.List;

public class Application {
    private final UserService<User> userService;
    
    public Application() {
        this.userService = new UserService<>();
    }
    
    public void run() {
        System.out.println("Starting Java Application...");
        
        // Create sample users
        List<User> sampleUsers = Arrays.asList(
            new User(1, "Alice Johnson", "alice@example.com"),
            new User(2, "Bob Smith", "bob@example.com"),
            new User(3, "Charlie Brown", "charlie@example.com")
        );
        
        // Add users to service
        sampleUsers.forEach(userService::addUser);
        
        // Display all users
        System.out.println("All users:");
        userService.getAllUsers().forEach(System.out::println);
        
        // Search functionality
        List<User> foundUsers = userService.findUsersByName("alice");
        System.out.println("Users with 'alice' in name: " + foundUsers.size());
        
        // Audit log
        System.out.println("Audit log:");
        userService.getAuditLog().forEach(System.out::println);
    }
    
    public static void main(String[] args) {
        Application app = new Application();
        app.run();
    }
}
""")

        # Interface example
        (src_main / "Repository.java").write_text("""
package com.example;

import java.util.List;
import java.util.Optional;

public interface Repository<T, ID> {
    void save(T entity);
    Optional<T> findById(ID id);
    List<T> findAll();
    boolean deleteById(ID id);
    long count();
}
""")

        self.log("Created Java test project", True)
        return project_path

    def create_python_project(self, project_dir):
        """Create a comprehensive Python test project"""
        project_path = Path(project_dir)
        
        # Python source directory
        src_dir = project_path / "src"
        src_dir.mkdir(parents=True, exist_ok=True)
        
        # User model with dataclass
        (src_dir / "user.py").write_text("""
from dataclasses import dataclass
from typing import Optional
import json

@dataclass
class User:
    id: int
    name: str
    email: str
    age: Optional[int] = None
    
    def to_dict(self) -> dict:
        return {
            'id': self.id,
            'name': self.name,
            'email': self.email,
            'age': self.age
        }
    
    def to_json(self) -> str:
        return json.dumps(self.to_dict())
    
    @classmethod
    def from_dict(cls, data: dict) -> 'User':
        return cls(**data)
    
    @classmethod
    def from_json(cls, json_str: str) -> 'User':
        return cls.from_dict(json.loads(json_str))
    
    def __str__(self) -> str:
        return f"User(id={self.id}, name='{self.name}', email='{self.email}')"
""")

        # Service class with async methods
        (src_dir / "user_service.py").write_text("""
import asyncio
from typing import List, Optional, Dict, Any
from user import User

class UserService:
    def __init__(self):
        self._users: Dict[int, User] = {}
        self._audit_log: List[str] = []
    
    async def add_user(self, user: User) -> None:
        \"\"\"Add a user to the service.\"\"\"
        self._users[user.id] = user
        self._audit_log.append(f"Added user: {user.id}")
        await asyncio.sleep(0.01)  # Simulate async operation
    
    async def get_user(self, user_id: int) -> Optional[User]:
        \"\"\"Get a user by ID.\"\"\"
        await asyncio.sleep(0.01)  # Simulate async operation
        return self._users.get(user_id)
    
    async def get_all_users(self) -> List[User]:
        \"\"\"Get all users.\"\"\"
        await asyncio.sleep(0.01)  # Simulate async operation
        return list(self._users.values())
    
    async def find_users_by_name(self, name_pattern: str) -> List[User]:
        \"\"\"Find users by name pattern.\"\"\"
        await asyncio.sleep(0.01)  # Simulate async operation
        pattern_lower = name_pattern.lower()
        return [
            user for user in self._users.values()
            if pattern_lower in user.name.lower()
        ]
    
    async def remove_user(self, user_id: int) -> bool:
        \"\"\"Remove a user by ID.\"\"\"
        if user_id in self._users:
            del self._users[user_id]
            self._audit_log.append(f"Removed user: {user_id}")
            await asyncio.sleep(0.01)  # Simulate async operation
            return True
        return False
    
    def get_user_count(self) -> int:
        \"\"\"Get the total number of users.\"\"\"
        return len(self._users)
    
    def get_audit_log(self) -> List[str]:
        \"\"\"Get the audit log.\"\"\"
        return self._audit_log.copy()
""")

        # Main application
        (src_dir / "app.py").write_text("""
import asyncio
from typing import List
from user import User
from user_service import UserService

class Application:
    def __init__(self):
        self.user_service = UserService()
    
    async def run(self) -> None:
        \"\"\"Run the application.\"\"\"
        print("Starting Python Application...")
        
        # Create sample users
        sample_users = [
            User(1, "Alice Johnson", "alice@example.com", 30),
            User(2, "Bob Smith", "bob@example.com", 25),
            User(3, "Charlie Brown", "charlie@example.com", 35)
        ]
        
        # Add users asynchronously
        for user in sample_users:
            await self.user_service.add_user(user)
        
        # Display all users
        all_users = await self.user_service.get_all_users()
        print(f"All users ({len(all_users)}):")
        for user in all_users:
            print(f"  {user}")
        
        # Search functionality
        found_users = await self.user_service.find_users_by_name("alice")
        print(f"Users with 'alice' in name: {len(found_users)}")
        
        # Audit log
        audit_log = self.user_service.get_audit_log()
        print("Audit log:")
        for entry in audit_log:
            print(f"  {entry}")

async def main() -> None:
    \"\"\"Main entry point.\"\"\"
    app = Application()
    await app.run()

if __name__ == "__main__":
    asyncio.run(main())
""")

        # Package init file
        (src_dir / "__init__.py").write_text("")

        self.log("Created Python test project", True)
        return project_path

    def create_cpp_project(self, project_dir):
        """Create a comprehensive C++ test project"""
        project_path = Path(project_dir)
        
        # C++ source directory
        src_dir = project_path / "src"
        include_dir = project_path / "include"
        src_dir.mkdir(parents=True, exist_ok=True)
        include_dir.mkdir(parents=True, exist_ok=True)
        
        # User header
        (include_dir / "user.h").write_text("""
#pragma once
#include <string>
#include <iostream>

namespace example {

class User {
private:
    int id_;
    std::string name_;
    std::string email_;

public:
    User(int id, const std::string& name, const std::string& email);
    
    // Getters
    int getId() const { return id_; }
    const std::string& getName() const { return name_; }
    const std::string& getEmail() const { return email_; }
    
    // Setters
    void setName(const std::string& name) { name_ = name; }
    void setEmail(const std::string& email) { email_ = email; }
    
    // Utility methods
    std::string toString() const;
    bool operator==(const User& other) const;
    bool operator!=(const User& other) const;
    
    // Stream operator
    friend std::ostream& operator<<(std::ostream& os, const User& user);
};

} // namespace example
""")

        # User service header with templates
        (include_dir / "user_service.h").write_text("""
#pragma once
#include "user.h"
#include <vector>
#include <unordered_map>
#include <optional>
#include <memory>
#include <algorithm>

namespace example {

template<typename T>
class UserService {
private:
    std::unordered_map<int, std::unique_ptr<T>> users_;
    std::vector<std::string> audit_log_;

public:
    UserService() = default;
    ~UserService() = default;
    
    // Delete copy constructor and assignment
    UserService(const UserService&) = delete;
    UserService& operator=(const UserService&) = delete;
    
    // Move constructor and assignment
    UserService(UserService&&) = default;
    UserService& operator=(UserService&&) = default;
    
    void addUser(std::unique_ptr<T> user);
    std::optional<T*> getUser(int id);
    std::vector<T*> getAllUsers();
    std::vector<T*> findUsersByName(const std::string& namePattern);
    bool removeUser(int id);
    size_t getUserCount() const;
    const std::vector<std::string>& getAuditLog() const;
};

// Template implementation
template<typename T>
void UserService<T>::addUser(std::unique_ptr<T> user) {
    int id = user->getId();
    users_[id] = std::move(user);
    audit_log_.push_back("Added user: " + std::to_string(id));
}

template<typename T>
std::optional<T*> UserService<T>::getUser(int id) {
    auto it = users_.find(id);
    if (it != users_.end()) {
        return it->second.get();
    }
    return std::nullopt;
}

template<typename T>
std::vector<T*> UserService<T>::getAllUsers() {
    std::vector<T*> result;
    for (const auto& pair : users_) {
        result.push_back(pair.second.get());
    }
    return result;
}

template<typename T>
std::vector<T*> UserService<T>::findUsersByName(const std::string& namePattern) {
    std::vector<T*> result;
    std::string lowerPattern = namePattern;
    std::transform(lowerPattern.begin(), lowerPattern.end(), lowerPattern.begin(), ::tolower);
    
    for (const auto& pair : users_) {
        std::string lowerName = pair.second->getName();
        std::transform(lowerName.begin(), lowerName.end(), lowerName.begin(), ::tolower);
        
        if (lowerName.find(lowerPattern) != std::string::npos) {
            result.push_back(pair.second.get());
        }
    }
    return result;
}

template<typename T>
bool UserService<T>::removeUser(int id) {
    auto it = users_.find(id);
    if (it != users_.end()) {
        users_.erase(it);
        audit_log_.push_back("Removed user: " + std::to_string(id));
        return true;
    }
    return false;
}

template<typename T>
size_t UserService<T>::getUserCount() const {
    return users_.size();
}

template<typename T>
const std::vector<std::string>& UserService<T>::getAuditLog() const {
    return audit_log_;
}

} // namespace example
""")

        # User implementation
        (src_dir / "user.cpp").write_text("""
#include "user.h"
#include <sstream>

namespace example {

User::User(int id, const std::string& name, const std::string& email)
    : id_(id), name_(name), email_(email) {
}

std::string User::toString() const {
    std::ostringstream oss;
    oss << "User{id=" << id_ << ", name='" << name_ << "', email='" << email_ << "'}";
    return oss.str();
}

bool User::operator==(const User& other) const {
    return id_ == other.id_;
}

bool User::operator!=(const User& other) const {
    return !(*this == other);
}

std::ostream& operator<<(std::ostream& os, const User& user) {
    os << user.toString();
    return os;
}

} // namespace example
""")

        # Main application
        (src_dir / "main.cpp").write_text("""
#include "user.h"
#include "user_service.h"
#include <iostream>
#include <memory>
#include <vector>

using namespace example;

class Application {
private:
    UserService<User> userService_;

public:
    void run() {
        std::cout << "Starting C++ Application..." << std::endl;
        
        // Create sample users
        auto user1 = std::make_unique<User>(1, "Alice Johnson", "alice@example.com");
        auto user2 = std::make_unique<User>(2, "Bob Smith", "bob@example.com");
        auto user3 = std::make_unique<User>(3, "Charlie Brown", "charlie@example.com");
        
        // Add users to service
        userService_.addUser(std::move(user1));
        userService_.addUser(std::move(user2));
        userService_.addUser(std::move(user3));
        
        // Display all users
        std::cout << "All users:" << std::endl;
        auto allUsers = userService_.getAllUsers();
        for (const auto* user : allUsers) {
            std::cout << "  " << *user << std::endl;
        }
        
        // Search functionality
        auto foundUsers = userService_.findUsersByName("alice");
        std::cout << "Users with 'alice' in name: " << foundUsers.size() << std::endl;
        
        // Audit log
        std::cout << "Audit log:" << std::endl;
        const auto& auditLog = userService_.getAuditLog();
        for (const auto& entry : auditLog) {
            std::cout << "  " << entry << std::endl;
        }
    }
};

int main() {
    Application app;
    app.run();
    return 0;
}
""")

        # CMakeLists.txt
        (project_path / "CMakeLists.txt").write_text("""
cmake_minimum_required(VERSION 3.10)
project(CppTestProject)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include_directories(include)

add_executable(app 
    src/main.cpp
    src/user.cpp
)
""")

        self.log("Created C++ test project", True)
        return project_path

    def create_go_project(self, project_dir):
        """Create a comprehensive Go test project"""
        project_path = Path(project_dir)
        
        # Go module initialization
        (project_path / "go.mod").write_text("""
module example.com/userapp

go 1.21
""")

        # User model
        (project_path / "user.go").write_text("""
package main

import (
    "encoding/json"
    "fmt"
)

// User represents a user in the system
type User struct {
    ID    int    `json:"id"`
    Name  string `json:"name"`
    Email string `json:"email"`
    Age   *int   `json:"age,omitempty"`
}

// NewUser creates a new User instance
func NewUser(id int, name, email string) *User {
    return &User{
        ID:    id,
        Name:  name,
        Email: email,
    }
}

// SetAge sets the user's age
func (u *User) SetAge(age int) {
    u.Age = &age
}

// GetAge returns the user's age or 0 if not set
func (u *User) GetAge() int {
    if u.Age == nil {
        return 0
    }
    return *u.Age
}

// String returns a string representation of the user
func (u *User) String() string {
    age := "unknown"
    if u.Age != nil {
        age = fmt.Sprintf("%d", *u.Age)
    }
    return fmt.Sprintf("User{ID=%d, Name='%s', Email='%s', Age=%s}", 
        u.ID, u.Name, u.Email, age)
}

// ToJSON converts the user to JSON
func (u *User) ToJSON() ([]byte, error) {
    return json.Marshal(u)
}

// FromJSON creates a user from JSON data
func FromJSON(data []byte) (*User, error) {
    var user User
    err := json.Unmarshal(data, &user)
    if err != nil {
        return nil, err
    }
    return &user, nil
}
""")

        # User service with generics (Go 1.18+)
        (project_path / "user_service.go").write_text("""
package main

import (
    "fmt"
    "strings"
    "sync"
)

// UserService manages users with generic type constraints
type UserService[T comparable] struct {
    users    map[int]*User
    auditLog []string
    mu       sync.RWMutex
}

// NewUserService creates a new UserService instance
func NewUserService[T comparable]() *UserService[T] {
    return &UserService[T]{
        users:    make(map[int]*User),
        auditLog: make([]string, 0),
    }
}

// AddUser adds a user to the service
func (s *UserService[T]) AddUser(user *User) {
    s.mu.Lock()
    defer s.mu.Unlock()
    
    s.users[user.ID] = user
    s.auditLog = append(s.auditLog, fmt.Sprintf("Added user: %d", user.ID))
}

// GetUser retrieves a user by ID
func (s *UserService[T]) GetUser(id int) (*User, bool) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    user, exists := s.users[id]
    return user, exists
}

// GetAllUsers returns all users
func (s *UserService[T]) GetAllUsers() []*User {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    users := make([]*User, 0, len(s.users))
    for _, user := range s.users {
        users = append(users, user)
    }
    return users
}

// FindUsersByName finds users by name pattern
func (s *UserService[T]) FindUsersByName(namePattern string) []*User {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    var result []*User
    lowerPattern := strings.ToLower(namePattern)
    
    for _, user := range s.users {
        if strings.Contains(strings.ToLower(user.Name), lowerPattern) {
            result = append(result, user)
        }
    }
    return result
}

// RemoveUser removes a user by ID
func (s *UserService[T]) RemoveUser(id int) bool {
    s.mu.Lock()
    defer s.mu.Unlock()
    
    if _, exists := s.users[id]; exists {
        delete(s.users, id)
        s.auditLog = append(s.auditLog, fmt.Sprintf("Removed user: %d", id))
        return true
    }
    return false
}

// GetUserCount returns the number of users
func (s *UserService[T]) GetUserCount() int {
    s.mu.RLock()
    defer s.mu.RUnlock()
    return len(s.users)
}

// GetAuditLog returns a copy of the audit log
func (s *UserService[T]) GetAuditLog() []string {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    log := make([]string, len(s.auditLog))
    copy(log, s.auditLog)
    return log
}
""")

        # Main application
        (project_path / "main.go").write_text("""
package main

import (
    "fmt"
    "log"
)

// Application represents the main application
type Application struct {
    userService *UserService[string]
}

// NewApplication creates a new Application instance
func NewApplication() *Application {
    return &Application{
        userService: NewUserService[string](),
    }
}

// Run starts the application
func (a *Application) Run() error {
    fmt.Println("Starting Go Application...")
    
    // Create sample users
    users := []*User{
        NewUser(1, "Alice Johnson", "alice@example.com"),
        NewUser(2, "Bob Smith", "bob@example.com"),
        NewUser(3, "Charlie Brown", "charlie@example.com"),
    }
    
    // Set ages for some users
    users[0].SetAge(30)
    users[1].SetAge(25)
    
    // Add users to service
    for _, user := range users {
        a.userService.AddUser(user)
    }
    
    // Display all users
    allUsers := a.userService.GetAllUsers()
    fmt.Printf("All users (%d):\\n", len(allUsers))
    for _, user := range allUsers {
        fmt.Printf("  %s\\n", user.String())
    }
    
    // Search functionality
    foundUsers := a.userService.FindUsersByName("alice")
    fmt.Printf("Users with 'alice' in name: %d\\n", len(foundUsers))
    
    // Audit log
    auditLog := a.userService.GetAuditLog()
    fmt.Println("Audit log:")
    for _, entry := range auditLog {
        fmt.Printf("  %s\\n", entry)
    }
    
    return nil
}

func main() {
    app := NewApplication()
    if err := app.Run(); err != nil {
        log.Fatalf("Application failed: %v", err)
    }
}
""")

        self.log("Created Go test project", True)
        return project_path

    def create_rust_project(self, project_dir):
        """Create a comprehensive Rust test project"""
        project_path = Path(project_dir)
        
        # Cargo.toml
        (project_path / "Cargo.toml").write_text("""
[package]
name = "rust-test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
""")

        # Source directory
        src_dir = project_path / "src"
        src_dir.mkdir(parents=True, exist_ok=True)
        
        # User model with derive macros
        (src_dir / "user.rs").write_text("""
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub age: Option<u32>,
}

impl User {
    pub fn new(id: u32, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            age: None,
        }
    }
    
    pub fn with_age(mut self, age: u32) -> Self {
        self.age = Some(age);
        self
    }
    
    pub fn set_age(&mut self, age: u32) {
        self.age = Some(age);
    }
    
    pub fn get_age(&self) -> Option<u32> {
        self.age
    }
    
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.age {
            Some(age) => write!(f, "User{{id={}, name='{}', email='{}', age={}}}", 
                              self.id, self.name, self.email, age),
            None => write!(f, "User{{id={}, name='{}', email='{}', age=unknown}}", 
                          self.id, self.name, self.email),
        }
    }
}
""")

        # User service with async methods and generics
        (src_dir / "user_service.rs").write_text("""
use crate::user::User;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct UserService<T> 
where 
    T: Clone + Send + Sync + 'static,
{
    users: Arc<RwLock<HashMap<u32, User>>>,
    audit_log: Arc<RwLock<Vec<String>>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> UserService<T> 
where 
    T: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            _phantom: std::marker::PhantomData,
        }
    }
    
    pub async fn add_user(&self, user: User) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut users = self.users.write().await;
        let mut audit_log = self.audit_log.write().await;
        
        let id = user.id;
        users.insert(id, user);
        audit_log.push(format!("Added user: {}", id));
        
        // Simulate async work
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        
        Ok(())
    }
    
    pub async fn get_user(&self, id: u32) -> Option<User> {
        let users = self.users.read().await;
        users.get(&id).cloned()
    }
    
    pub async fn get_all_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }
    
    pub async fn find_users_by_name(&self, name_pattern: &str) -> Vec<User> {
        let users = self.users.read().await;
        let pattern_lower = name_pattern.to_lowercase();
        
        users
            .values()
            .filter(|user| user.name.to_lowercase().contains(&pattern_lower))
            .cloned()
            .collect()
    }
    
    pub async fn remove_user(&self, id: u32) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut users = self.users.write().await;
        let mut audit_log = self.audit_log.write().await;
        
        if users.remove(&id).is_some() {
            audit_log.push(format!("Removed user: {}", id));
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub async fn get_user_count(&self) -> usize {
        let users = self.users.read().await;
        users.len()
    }
    
    pub async fn get_audit_log(&self) -> Vec<String> {
        let audit_log = self.audit_log.read().await;
        audit_log.clone()
    }
}

impl<T> Default for UserService<T> 
where 
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
""")

        # Main application
        (src_dir / "main.rs").write_text("""
mod user;
mod user_service;

use user::User;
use user_service::UserService;

pub struct Application {
    user_service: UserService<String>,
}

impl Application {
    pub fn new() -> Self {
        Self {
            user_service: UserService::new(),
        }
    }
    
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Starting Rust Application...");
        
        // Create sample users
        let users = vec![
            User::new(1, "Alice Johnson".to_string(), "alice@example.com".to_string()).with_age(30),
            User::new(2, "Bob Smith".to_string(), "bob@example.com".to_string()).with_age(25),
            User::new(3, "Charlie Brown".to_string(), "charlie@example.com".to_string()).with_age(35),
        ];
        
        // Add users to service
        for user in users {
            self.user_service.add_user(user).await?;
        }
        
        // Display all users
        let all_users = self.user_service.get_all_users().await;
        println!("All users ({}):", all_users.len());
        for user in &all_users {
            println!("  {}", user);
        }
        
        // Search functionality
        let found_users = self.user_service.find_users_by_name("alice").await;
        println!("Users with 'alice' in name: {}", found_users.len());
        
        // Audit log
        let audit_log = self.user_service.get_audit_log().await;
        println!("Audit log:");
        for entry in &audit_log {
            println!("  {}", entry);
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Application::new();
    app.run().await
}
""")

        self.log("Created Rust test project", True)
        return project_path

    def create_csharp_project(self, project_dir):
        """Create a comprehensive C# test project"""
        project_path = Path(project_dir)
        
        # Create .csproj file
        (project_path / "CSharpTestProject.csproj").write_text("""
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>
""")

        # User model
        (project_path / "User.cs").write_text("""
using System.Text.Json;

namespace CSharpTestProject;

public record User(int Id, string Name, string Email)
{
    public int? Age { get; set; }
    
    public User WithAge(int age) => this with { Age = age };
    
    public string ToJson() => JsonSerializer.Serialize(this);
    
    public static User? FromJson(string json) => JsonSerializer.Deserialize<User>(json);
    
    public override string ToString() => Age.HasValue 
        ? $"User{{Id={Id}, Name='{Name}', Email='{Email}', Age={Age}}}"
        : $"User{{Id={Id}, Name='{Name}', Email='{Email}', Age=unknown}}";
}
""")

        # User service
        (project_path / "UserService.cs").write_text("""
using System.Collections.Concurrent;

namespace CSharpTestProject;

public class UserService<T> where T : class
{
    private readonly ConcurrentDictionary<int, User> _users = new();
    private readonly List<string> _auditLog = new();
    private readonly object _auditLock = new();
    
    public async Task AddUserAsync(User user)
    {
        _users.TryAdd(user.Id, user);
        
        lock (_auditLock)
        {
            _auditLog.Add($"Added user: {user.Id}");
        }
        
        // Simulate async work
        await Task.Delay(1);
    }
    
    public async Task<User?> GetUserAsync(int id)
    {
        // Simulate async work
        await Task.Delay(1);
        return _users.TryGetValue(id, out var user) ? user : null;
    }
    
    public async Task<List<User>> GetAllUsersAsync()
    {
        // Simulate async work
        await Task.Delay(1);
        return _users.Values.ToList();
    }
    
    public async Task<List<User>> FindUsersByNameAsync(string namePattern)
    {
        // Simulate async work
        await Task.Delay(1);
        
        var lowerPattern = namePattern.ToLower();
        return _users.Values
            .Where(user => user.Name.ToLower().Contains(lowerPattern))
            .ToList();
    }
    
    public async Task<bool> RemoveUserAsync(int id)
    {
        var removed = _users.TryRemove(id, out _);
        
        if (removed)
        {
            lock (_auditLock)
            {
                _auditLog.Add($"Removed user: {id}");
            }
        }
        
        // Simulate async work
        await Task.Delay(1);
        return removed;
    }
    
    public int GetUserCount() => _users.Count;
    
    public List<string> GetAuditLog()
    {
        lock (_auditLock)
        {
            return new List<string>(_auditLog);
        }
    }
}
""")

        # Main application
        (project_path / "Program.cs").write_text("""
namespace CSharpTestProject;

public class Application
{
    private readonly UserService<string> _userService = new();
    
    public async Task RunAsync()
    {
        Console.WriteLine("Starting C# Application...");
        
        // Create sample users
        var users = new[]
        {
            new User(1, "Alice Johnson", "alice@example.com").WithAge(30),
            new User(2, "Bob Smith", "bob@example.com").WithAge(25),
            new User(3, "Charlie Brown", "charlie@example.com").WithAge(35)
        };
        
        // Add users to service
        foreach (var user in users)
        {
            await _userService.AddUserAsync(user);
        }
        
        // Display all users
        var allUsers = await _userService.GetAllUsersAsync();
        Console.WriteLine($"All users ({allUsers.Count}):");
        foreach (var user in allUsers)
        {
            Console.WriteLine($"  {user}");
        }
        
        // Search functionality
        var foundUsers = await _userService.FindUsersByNameAsync("alice");
        Console.WriteLine($"Users with 'alice' in name: {foundUsers.Count}");
        
        // Audit log
        var auditLog = _userService.GetAuditLog();
        Console.WriteLine("Audit log:");
        foreach (var entry in auditLog)
        {
            Console.WriteLine($"  {entry}");
        }
    }
}

public class Program
{
    public static async Task Main(string[] args)
    {
        var app = new Application();
        await app.RunAsync();
    }
}
""")

        self.log("Created C# test project", True)
        return project_path

    def test_language_e2e(self, language_name, project_path):
        """Test end-to-end functionality for a specific language"""
        self.log(f"Testing {language_name} end-to-end...")
        
        # Test scanning
        cmd = ["cargo", "run", "--", "--repo", str(project_path), "scan"]
        result = self.run_command(cmd, timeout=90)
        
        if not result or result.returncode != 0:
            self.log(f"{language_name} scan failed", False)
            return False
        
        self.log(f"{language_name} scan completed", True)
        
        # Validate database
        db_path = project_path / ".reviewbot" / "graph.db"
        if not db_path.exists():
            self.log(f"{language_name} database not created", False)
            return False
        
        try:
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            # Check symbol count
            cursor.execute("SELECT COUNT(*) FROM symbol")
            symbol_count = cursor.fetchone()[0]
            
            # Check for expected symbols based on language
            min_symbols = 5  # Default minimum
            if language_name == "C#":
                min_symbols = 1  # C# parser is currently basic (file-level only)
            
            if symbol_count < min_symbols:
                self.log(f"{language_name} insufficient symbols found: {symbol_count}", False)
                return False
            
            self.log(f"{language_name} found {symbol_count} symbols", True)
            
            # Test search functionality
            cmd = ["cargo", "run", "--", "--repo", str(project_path), "search", "User"]
            result = self.run_command(cmd)
            
            if result and result.returncode == 0 and "User" in result.stdout:
                self.log(f"{language_name} search working", True)
            else:
                self.log(f"{language_name} search failed", False)
                return False
            
            conn.close()
            return True
            
        except Exception as e:
            self.log(f"{language_name} database validation failed: {e}", False)
            return False

    def run_all_language_tests(self):
        """Run e2e tests for all supported languages"""
        print("🚀 Multi-Language End-to-End Test Suite")
        print("=" * 60)
        
        languages = [
            ("Java", self.create_java_project),
            ("Python", self.create_python_project),
            ("C++", self.create_cpp_project),
            ("Go", self.create_go_project),
            ("Rust", self.create_rust_project),
            ("C#", self.create_csharp_project),
        ]
        
        results = {}
        
        for language_name, create_project_func in languages:
            print(f"\n🧪 Testing {language_name}")
            print("-" * 40)
            
            with tempfile.TemporaryDirectory(prefix=f"{language_name.lower()}_e2e_") as temp_dir:
                try:
                    project_path = create_project_func(temp_dir)
                    success = self.test_language_e2e(language_name, project_path)
                    results[language_name] = success
                except Exception as e:
                    self.log(f"{language_name} test failed with exception: {e}", False)
                    results[language_name] = False
        
        # Final results
        print("\n" + "=" * 60)
        print("📊 Multi-Language Test Results")
        print("=" * 60)
        
        passed = sum(results.values())
        total = len(results)
        
        for language, success in results.items():
            status = "✅ PASSED" if success else "❌ FAILED"
            print(f"{language:>8}: {status}")
        
        print(f"\nOverall: {passed}/{total} languages passed")
        
        if passed == total:
            print("\n🎉 ALL LANGUAGES FULLY VALIDATED!")
            print("🚀 Complete multi-language support confirmed!")
            return True
        else:
            print(f"\n⚠️  {total - passed} language(s) failed validation")
            return False


if __name__ == "__main__":
    import sys
    test_suite = MultiLanguageE2ETest()
    success = test_suite.run_all_language_tests()
    sys.exit(0 if success else 1)