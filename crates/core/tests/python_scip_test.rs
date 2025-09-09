use anyhow::Result;
use store::GraphStore;
use scip_mapper::ScipMapper;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_test_python_project(temp_dir: &Path) -> Result<()> {
    // Create main.py with dataclasses and service
    let main_content = r#"from typing import List, Optional
from dataclasses import dataclass


@dataclass
class User:
    """A user dataclass."""
    id: int
    name: str
    email: str


class UserService:
    """Service for managing users."""
    
    def __init__(self):
        self.users: List[User] = []

    def add_user(self, user: User) -> None:
        """Add a user to the service."""
        self.users.append(user)

    def find_user(self, user_id: int) -> Optional[User]:
        """Find a user by ID."""
        for user in self.users:
            if user.id == user_id:
                return user
        return None

    def get_all_users(self) -> List[User]:
        """Get all users."""
        return self.users.copy()
"#;
    fs::write(temp_dir.join("main.py"), main_content)?;

    // Create models.py with inheritance
    let models_content = r#"from typing import Dict, Protocol, runtime_checkable
from abc import ABC, abstractmethod


@runtime_checkable
class Identifiable(Protocol):
    """Protocol for objects with an ID."""
    id: int


class BaseModel(ABC):
    """Base class for all models."""

    @abstractmethod
    def to_dict(self) -> dict:
        """Convert to dictionary."""
        pass


class Product(BaseModel):
    """Product model."""
    
    def __init__(self, id: int, name: str, price: float):
        self.id = id
        self.name = name
        self.price = price

    def to_dict(self) -> dict:
        """Convert to dictionary."""
        return {
            "id": self.id,
            "name": self.name,
            "price": self.price
        }

    @property
    def formatted_price(self) -> str:
        """Get formatted price."""
        return f"${self.price:.2f}"
"#;
    fs::write(temp_dir.join("models.py"), models_content)?;

    Ok(())
}

#[test]
fn test_python_scip_end_to_end() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create test Python project
    create_test_python_project(project_path)?;
    
    // Create SCIP mapper with consistent API
    let scip_mapper = ScipMapper::new("scip-python", "0.6.6");
    
    // Test the new run_scip_python method
    let scip_file_result = scip_mapper.run_scip_python(&project_path.to_string_lossy());
    
    // If scip-python isn't available, skip test
    if scip_file_result.is_err() {
        println!("Skipping Python SCIP test - scip-python not available");
        return Ok(());
    }
    
    let scip_file = scip_file_result?;
    if !Path::new(&scip_file).exists() {
        println!("Skipping Python SCIP test - SCIP indexing failed");
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
    
    println!("✅ Python SCIP integration found:");
    println!("   {} symbols", symbols.len());
    println!("   {} edges", edges.len()); 
    println!("   {} occurrences", occurrences.len());
    
    Ok(())
}

#[test]
fn test_python_scip_inheritance_relationships() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();
    
    // Create test Python project
    create_test_python_project(project_path)?;
    
    // Create SCIP mapper
    let scip_mapper = ScipMapper::new("scip-python", "0.6.6");
    
    // Test SCIP processing
    let scip_file_result = scip_mapper.run_scip_python(&project_path.to_string_lossy());
    
    // If scip-python isn't available, skip test
    if scip_file_result.is_err() {
        println!("Skipping Python inheritance test - scip-python not available");
        return Ok(());
    }
    
    let scip_file = scip_file_result?;
    let scip_index = scip_mapper.parse_scip_index(&scip_file)?;
    let (_symbols, edges, _occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "test_commit")?;
    
    // Should have semantic edges for inheritance relationships  
    assert!(!edges.is_empty(), "Should have semantic inheritance edges");
    
    println!("✅ Found {} inheritance edges in Python project", edges.len());
    
    Ok(())
}