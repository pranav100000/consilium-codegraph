"""Data models for the application."""

from typing import Protocol, runtime_checkable
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