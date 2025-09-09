"""Main module demonstrating Python classes and functions."""

from typing import List, Optional
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


def create_test_user(user_id: int, name: str) -> User:
    """Create a test user."""
    return User(
        id=user_id,
        name=name,
        email=f"{name.lower()}@example.com"
    )


def main() -> None:
    """Main function."""
    service = UserService()
    
    user1 = create_test_user(1, "Alice")
    user2 = create_test_user(2, "Bob")
    
    service.add_user(user1)
    service.add_user(user2)
    
    found = service.find_user(1)
    if found:
        print(f"Found user: {found.name}")
    
    all_users = service.get_all_users()
    print(f"Total users: {len(all_users)}")


if __name__ == "__main__":
    main()