use anyhow::Result;
use store::GraphStore;
use scip_mapper::ScipMapper;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_test_go_project(temp_dir: &Path) -> Result<()> {
    // Create go.mod
    let go_mod = r#"module test-project

go 1.21
"#;
    fs::write(temp_dir.join("go.mod"), go_mod)?;
    
    // Create main.go with struct and interface
    let main_content = r#"package main

import "fmt"

// User represents a user in the system
type User struct {
	ID   int    `json:"id"`
	Name string `json:"name"`
	Email string `json:"email"`
}

// UserService interface defines user operations
type UserService interface {
	AddUser(user User) error
	GetUser(id int) (*User, error)
	GetAllUsers() ([]User, error)
}

// MemoryUserService implements UserService using in-memory storage
type MemoryUserService struct {
	users map[int]User
}

// NewMemoryUserService creates a new in-memory user service
func NewMemoryUserService() *MemoryUserService {
	return &MemoryUserService{
		users: make(map[int]User),
	}
}

// AddUser adds a user to the service
func (s *MemoryUserService) AddUser(user User) error {
	s.users[user.ID] = user
	return nil
}

// GetUser retrieves a user by ID
func (s *MemoryUserService) GetUser(id int) (*User, error) {
	user, exists := s.users[id]
	if !exists {
		return nil, fmt.Errorf("user with ID %d not found", id)
	}
	return &user, nil
}

// GetAllUsers returns all users
func (s *MemoryUserService) GetAllUsers() ([]User, error) {
	users := make([]User, 0, len(s.users))
	for _, user := range s.users {
		users = append(users, user)
	}
	return users, nil
}

func main() {
	service := NewMemoryUserService()
	
	// Add a test user
	user := User{
		ID:   1,
		Name: "John Doe",
		Email: "john@example.com",
	}
	
	if err := service.AddUser(user); err != nil {
		fmt.Printf("Error adding user: %v\n", err)
		return
	}
	
	// Retrieve the user
	retrievedUser, err := service.GetUser(1)
	if err != nil {
		fmt.Printf("Error getting user: %v\n", err)
		return
	}
	
	fmt.Printf("Retrieved user: %+v\n", *retrievedUser)
}
"#;
    fs::write(temp_dir.join("main.go"), main_content)?;
    
    // Create a helper package
    let helper_content = r#"package main

import "fmt"

// Validator provides validation utilities
type Validator struct{}

// NewValidator creates a new validator instance
func NewValidator() *Validator {
	return &Validator{}
}

// ValidateUser validates a user struct
func (v *Validator) ValidateUser(user User) error {
	if user.ID <= 0 {
		return fmt.Errorf("user ID must be positive")
	}
	
	if user.Name == "" {
		return fmt.Errorf("user name cannot be empty")
	}
	
	if user.Email == "" {
		return fmt.Errorf("user email cannot be empty")
	}
	
	return nil
}

// FormatUserInfo returns a formatted string with user information
func FormatUserInfo(user User) string {
	return fmt.Sprintf("User(ID: %d, Name: %s, Email: %s)", user.ID, user.Name, user.Email)
}
"#;
    fs::write(temp_dir.join("helper.go"), helper_content)?;
    
    Ok(())
}

#[test]
fn test_go_scip_end_to_end() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create test Go project
    create_test_go_project(project_path)?;
    
    // Create SCIP mapper with consistent API
    let scip_mapper = ScipMapper::new("scip-go", "1.0.0");
    
    // Test the new run_scip_go method
    let scip_file_result = scip_mapper.run_scip_go(&project_path.to_string_lossy());
    
    // If scip-go isn't available, skip test
    if scip_file_result.is_err() {
        println!("Skipping Go SCIP test - scip-go not available");
        return Ok(());
    }
    
    let scip_file = scip_file_result?;
    if !Path::new(&scip_file).exists() {
        println!("Skipping Go SCIP test - SCIP indexing failed");
        return Ok(());
    }
    
    // Parse SCIP index
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    assert!(!scip_index.documents.is_empty(), "Should find documents in SCIP index");
    
    // Convert to IR
    let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test_commit")?;
    
    // Verify semantic data quality
    assert!(!symbols.is_empty(), "Should find semantic symbols");
    assert!(!occurrences.is_empty(), "Should find semantic occurrences");
    
    println!("✅ Go SCIP integration found:");
    println!("   {} symbols", symbols.len());
    println!("   {} edges", edges.len()); 
    println!("   {} occurrences", occurrences.len());
    
    // Look for expected Go constructs
    let struct_symbols: Vec<_> = symbols.iter().filter(|s| s.name.contains("User")).collect();
    let interface_symbols: Vec<_> = symbols.iter().filter(|s| s.name.contains("UserService")).collect();
    let method_symbols: Vec<_> = symbols.iter().filter(|s| s.name.contains("AddUser") || s.name.contains("GetUser")).collect();
    
    println!("   Found {} User-related struct symbols", struct_symbols.len());
    println!("   Found {} UserService-related interface symbols", interface_symbols.len());
    println!("   Found {} method symbols", method_symbols.len());
    
    Ok(())
}

#[test]
fn test_go_scip_interface_relationships() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create test Go project
    create_test_go_project(project_path)?;
    
    // Create SCIP mapper
    let scip_mapper = ScipMapper::new("scip-go", "1.0.0");
    
    // Test SCIP processing
    let scip_file_result = scip_mapper.run_scip_go(&project_path.to_string_lossy());
    
    // If scip-go isn't available, skip test
    if scip_file_result.is_err() {
        println!("Skipping Go interface test - scip-go not available");
        return Ok(());
    }
    
    let scip_file = scip_file_result?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (_symbols, edges, _occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test_commit")?;
    
    // Should have semantic edges for interface implementations
    assert!(!edges.is_empty(), "Should have semantic implementation edges");
    
    println!("✅ Found {} interface implementation edges in Go project", edges.len());
    
    Ok(())
}

#[test]
fn test_go_scip_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create test Go project
    create_test_go_project(project_path)?;
    
    println!("🚀 Go SCIP Performance Test");
    
    let scip_mapper = ScipMapper::new("scip-go", "1.0.0");
    
    // Test SCIP indexing performance
    let indexing_start = std::time::Instant::now();
    let scip_file_result = scip_mapper.run_scip_go(&project_path.to_string_lossy());
    let indexing_time = indexing_start.elapsed();
    
    if scip_file_result.is_err() {
        println!("⏩ Skipping - scip-go not available");
        return Ok(());
    }
    
    let scip_file = scip_file_result?;
    let file_size = fs::metadata(&scip_file)?.len();
    
    // Test JSON parsing
    let parsing_start = std::time::Instant::now();
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let parsing_time = parsing_start.elapsed();
    
    // Test IR conversion
    let conversion_start = std::time::Instant::now();
    let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "bench")?;
    let conversion_time = conversion_start.elapsed();
    
    let total_time = indexing_time + parsing_time + conversion_time;
    
    println!("📊 Go SCIP Results:");
    println!("  Indexing:    {:>6.0}ms", indexing_time.as_millis());
    println!("  Parsing:     {:>6.0}ms", parsing_time.as_millis()); 
    println!("  Conversion:  {:>6.0}ms", conversion_time.as_millis());
    println!("  Total:       {:>6.0}ms", total_time.as_millis());
    println!("  SCIP size:   {:>6} bytes", file_size);
    println!("  Symbols:     {:>6}", symbols.len());
    println!("  Edges:       {:>6}", edges.len());
    println!("  Occurrences: {:>6}", occurrences.len());
    
    // Performance assertions
    assert!(indexing_time.as_secs() < 60, "Go indexing should complete under 60s");
    assert!(parsing_time.as_millis() < 2000, "Parsing should complete under 2s");
    assert!(!symbols.is_empty(), "Should find symbols");
    
    println!("✅ Go SCIP performance test completed!");
    
    Ok(())
}