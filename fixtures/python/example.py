#!/usr/bin/env python3
"""Python fixture with various language features"""

import asyncio
import functools
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import (
    Any, Dict, Generic, List, Optional, TypeVar, Union,
    Protocol, Literal, TypedDict, Final, ClassVar
)
from contextlib import contextmanager
import logging

# Constants
MAX_RETRIES: Final = 3
DEFAULT_TIMEOUT: Final = 30

# Enum
class UserRole(Enum):
    ADMIN = auto()
    USER = auto()
    GUEST = auto()

# TypedDict
class UserDict(TypedDict):
    id: int
    name: str
    email: str
    role: UserRole

# Protocol (structural typing)
class Cacheable(Protocol):
    def get_cache_key(self) -> str: ...
    def serialize(self) -> bytes: ...

# Generic type variable
T = TypeVar('T', bound='BaseModel')

# Dataclass
@dataclass
class User:
    """User model with dataclass"""
    id: int
    name: str
    email: str
    role: UserRole = UserRole.USER
    is_active: bool = True
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    # Class variable
    instance_count: ClassVar[int] = 0
    
    def __post_init__(self):
        User.instance_count += 1
    
    def get_display_name(self) -> str:
        """Get formatted display name"""
        return f"{self.name} ({self.role.name})"
    
    @property
    def is_admin(self) -> bool:
        """Check if user is admin"""
        return self.role == UserRole.ADMIN
    
    @classmethod
    def from_dict(cls, data: UserDict) -> 'User':
        """Create user from dictionary"""
        return cls(**data)

# Abstract base class
class BaseService(ABC, Generic[T]):
    """Abstract base service class"""
    
    def __init__(self):
        self.items: List[T] = []
        self.logger = logging.getLogger(self.__class__.__name__)
    
    @abstractmethod
    def validate(self, item: T) -> bool:
        """Validate an item"""
        pass
    
    def add(self, item: T) -> None:
        """Add an item if valid"""
        if self.validate(item):
            self.items.append(item)
            self.logger.info(f"Added item: {item}")
    
    def find_by_id(self, item_id: int) -> Optional[T]:
        """Find item by ID"""
        for item in self.items:
            if hasattr(item, 'id') and item.id == item_id:
                return item
        return None

# Concrete implementation
class UserService(BaseService[User]):
    """User service implementation"""
    
    def __init__(self, cache_enabled: bool = True):
        super().__init__()
        self.cache_enabled = cache_enabled
        self._cache: Dict[int, User] = {}
    
    def validate(self, user: User) -> bool:
        """Validate user data"""
        import re
        email_pattern = r'^[^\s@]+@[^\s@]+\.[^\s@]+$'
        return (
            len(user.name) > 0 and
            re.match(email_pattern, user.email) is not None
        )
    
    async def create_user(self, name: str, email: str, **kwargs) -> User:
        """Create a new user asynchronously"""
        user = User(
            id=self._generate_id(),
            name=name,
            email=email,
            **kwargs
        )
        self.add(user)
        
        if self.cache_enabled:
            self._cache[user.id] = user
        
        # Simulate async operation
        await asyncio.sleep(0.1)
        return user
    
    def _generate_id(self) -> int:
        """Generate unique ID"""
        import random
        return random.randint(1000, 9999)
    
    def get_active_users(self) -> List[User]:
        """Get all active users"""
        return [u for u in self.items if u.is_active]
    
    def get_users_by_role(self, role: UserRole) -> List[User]:
        """Get users by role"""
        return [u for u in self.items if u.role == role]

# Decorator
def deprecated(reason: str):
    """Decorator to mark functions as deprecated"""
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            logging.warning(f"{func.__name__} is deprecated: {reason}")
            return func(*args, **kwargs)
        return wrapper
    return decorator

def retry(max_attempts: int = MAX_RETRIES):
    """Retry decorator with configurable attempts"""
    def decorator(func):
        @functools.wraps(func)
        async def async_wrapper(*args, **kwargs):
            for attempt in range(max_attempts):
                try:
                    return await func(*args, **kwargs)
                except Exception as e:
                    if attempt == max_attempts - 1:
                        raise
                    logging.warning(f"Attempt {attempt + 1} failed: {e}")
                    await asyncio.sleep(2 ** attempt)
        
        @functools.wraps(func)
        def sync_wrapper(*args, **kwargs):
            for attempt in range(max_attempts):
                try:
                    return func(*args, **kwargs)
                except Exception as e:
                    if attempt == max_attempts - 1:
                        raise
                    logging.warning(f"Attempt {attempt + 1} failed: {e}")
        
        return async_wrapper if asyncio.iscoroutinefunction(func) else sync_wrapper
    return decorator

# Context manager
@contextmanager
def user_context(user: User):
    """Context manager for user operations"""
    logging.info(f"Entering context for user: {user.name}")
    try:
        yield user
    finally:
        logging.info(f"Exiting context for user: {user.name}")

# Generator
def user_generator(count: int) -> Generator[User, None, None]:
    """Generate test users"""
    for i in range(count):
        yield User(
            id=i,
            name=f"User{i}",
            email=f"user{i}@example.com",
            role=UserRole.USER if i % 2 == 0 else UserRole.ADMIN
        )

# Async generator
async def async_user_generator(count: int) -> AsyncGenerator[User, None]:
    """Generate users asynchronously"""
    service = UserService()
    for i in range(count):
        user = await service.create_user(
            name=f"AsyncUser{i}",
            email=f"async{i}@example.com"
        )
        yield user

# Multiple inheritance
class Auditable:
    """Mixin for auditable entities"""
    
    def __init__(self):
        self.created_at = None
        self.updated_at = None
        self.created_by = None
    
    def audit_create(self, user: User):
        """Audit creation"""
        import datetime
        self.created_at = datetime.datetime.now()
        self.created_by = user.id

class TrackedUser(User, Auditable):
    """User with audit tracking"""
    
    def __init__(self, *args, **kwargs):
        User.__init__(self, *args, **kwargs)
        Auditable.__init__(self)

# Metaclass
class SingletonMeta(type):
    """Singleton metaclass"""
    _instances = {}
    
    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            cls._instances[cls] = super().__call__(*args, **kwargs)
        return cls._instances[cls]

class ConfigManager(metaclass=SingletonMeta):
    """Singleton configuration manager"""
    
    def __init__(self):
        self.config: Dict[str, Any] = {}
    
    def set(self, key: str, value: Any):
        """Set configuration value"""
        self.config[key] = value
    
    def get(self, key: str, default: Any = None) -> Any:
        """Get configuration value"""
        return self.config.get(key, default)

# Property decorators
class Temperature:
    """Temperature class with properties"""
    
    def __init__(self, celsius: float = 0.0):
        self._celsius = celsius
    
    @property
    def celsius(self) -> float:
        """Get temperature in Celsius"""
        return self._celsius
    
    @celsius.setter
    def celsius(self, value: float):
        """Set temperature in Celsius"""
        if value < -273.15:
            raise ValueError("Temperature below absolute zero is not possible")
        self._celsius = value
    
    @property
    def fahrenheit(self) -> float:
        """Get temperature in Fahrenheit"""
        return self._celsius * 9/5 + 32
    
    @fahrenheit.setter
    def fahrenheit(self, value: float):
        """Set temperature in Fahrenheit"""
        self.celsius = (value - 32) * 5/9

# Type annotations with Union and Optional
def process_value(value: Union[int, str, None]) -> Optional[str]:
    """Process different value types"""
    if value is None:
        return None
    if isinstance(value, int):
        return str(value)
    return value.upper()

# Literal types
def set_mode(mode: Literal['read', 'write', 'append']) -> None:
    """Set file mode with literal type"""
    valid_modes = ['read', 'write', 'append']
    if mode not in valid_modes:
        raise ValueError(f"Invalid mode: {mode}")
    logging.info(f"Mode set to: {mode}")

# Custom exception
class ValidationError(Exception):
    """Custom validation exception"""
    
    def __init__(self, field: str, message: str):
        self.field = field
        self.message = message
        super().__init__(f"Validation error on {field}: {message}")

# Main async function
async def main():
    """Main execution function"""
    logging.basicConfig(level=logging.INFO)
    
    # Create service
    service = UserService()
    
    # Create users
    admin = await service.create_user(
        "Admin User",
        "admin@example.com",
        role=UserRole.ADMIN
    )
    
    # Use context manager
    with user_context(admin):
        # Perform operations
        active_users = service.get_active_users()
        logging.info(f"Active users: {len(active_users)}")
    
    # Use generator
    for user in user_generator(3):
        service.add(user)
    
    # Use async generator
    async for user in async_user_generator(2):
        logging.info(f"Generated async user: {user.name}")
    
    # Test singleton
    config1 = ConfigManager()
    config2 = ConfigManager()
    assert config1 is config2
    
    config1.set('debug', True)
    assert config2.get('debug') is True
    
    # Test temperature properties
    temp = Temperature()
    temp.celsius = 25
    logging.info(f"Temperature: {temp.celsius}°C = {temp.fahrenheit}°F")
    
    # Test type annotations
    result = process_value(42)
    assert result == "42"
    
    result = process_value("hello")
    assert result == "HELLO"
    
    # Test tracked user
    tracked = TrackedUser(
        id=1000,
        name="Tracked User",
        email="tracked@example.com"
    )
    tracked.audit_create(admin)
    
    logging.info("All tests completed!")

# Module level execution
if __name__ == "__main__":
    asyncio.run(main())