#!/usr/bin/env python3
"""Test Python file for code graph analysis."""

import os
import sys
from typing import List, Dict, Optional
from dataclasses import dataclass

# Global variable
CONFIG = {
    "debug": True,
    "port": 8080
}

@dataclass
class User:
    """User model."""
    name: str
    email: str
    age: int = 0
    
    def get_display_name(self) -> str:
        """Get display name for the user."""
        return f"{self.name} <{self.email}>"
    
    def is_adult(self) -> bool:
        """Check if user is an adult."""
        return self.age >= 18

class UserManager:
    """Manages user operations."""
    
    def __init__(self):
        self.users: List[User] = []
        self.user_map: Dict[str, User] = {}
    
    def add_user(self, user: User) -> None:
        """Add a new user."""
        self.users.append(user)
        self.user_map[user.email] = user
    
    def find_user(self, email: str) -> Optional[User]:
        """Find user by email."""
        return self.user_map.get(email)
    
    def get_adult_users(self) -> List[User]:
        """Get all adult users."""
        return [u for u in self.users if u.is_adult()]

def process_data(data: List[int]) -> int:
    """Process a list of integers."""
    total = sum(data)
    return total * 2

def main():
    """Main entry point."""
    manager = UserManager()
    
    # Add some users
    user1 = User("Alice", "alice@example.com", 25)
    user2 = User("Bob", "bob@example.com", 17)
    
    manager.add_user(user1)
    manager.add_user(user2)
    
    # Find adult users
    adults = manager.get_adult_users()
    for user in adults:
        print(user.get_display_name())
    
    # Process some data
    result = process_data([1, 2, 3, 4, 5])
    print(f"Result: {result}")

if __name__ == "__main__":
    main()