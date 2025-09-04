#[cfg(test)]
pub mod fixtures {
    pub const SIMPLE_FUNCTION: &str = r#"
def add(a, b):
    return a + b

def multiply(x, y):
    return x * y
"#;

    pub const CLASS_WITH_METHODS: &str = r#"
class Calculator:
    def __init__(self):
        self.value = 0
    
    def add(self, n):
        self.value += n
    
    def get_value(self):
        return self.value
"#;

    pub const DECORATORS_AND_PROPERTIES: &str = r#"
from functools import wraps
from typing import Any

def memoize(func):
    cache = {}
    @wraps(func)
    def wrapper(*args, **kwargs):
        key = str(args) + str(kwargs)
        if key not in cache:
            cache[key] = func(*args, **kwargs)
        return cache[key]
    return wrapper

class Person:
    def __init__(self, name: str, age: int):
        self._name = name
        self._age = age
    
    @property
    def name(self) -> str:
        return self._name
    
    @name.setter
    def name(self, value: str):
        if not value:
            raise ValueError("Name cannot be empty")
        self._name = value
    
    @property
    def age(self) -> int:
        return self._age
    
    @age.setter  
    def age(self, value: int):
        if value < 0:
            raise ValueError("Age cannot be negative")
        self._age = value
    
    @staticmethod
    def species():
        return "Homo sapiens"
    
    @classmethod
    def from_birth_year(cls, name: str, birth_year: int):
        import datetime
        age = datetime.datetime.now().year - birth_year
        return cls(name, age)
    
    @memoize
    def expensive_operation(self, n: int) -> int:
        import time
        time.sleep(1)
        return n * 2
"#;

    pub const ASYNC_AND_GENERATORS: &str = r#"
import asyncio
from typing import AsyncIterator, Iterator, Generator

async def fetch_data(url: str) -> dict:
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.json()

async def process_urls(urls: list[str]) -> list[dict]:
    tasks = [fetch_data(url) for url in urls]
    return await asyncio.gather(*tasks)

def fibonacci() -> Generator[int, None, None]:
    a, b = 0, 1
    while True:
        yield a
        a, b = b, a + b

async def async_counter(start: int, end: int) -> AsyncIterator[int]:
    for i in range(start, end):
        await asyncio.sleep(0.1)
        yield i

class AsyncContextManager:
    async def __aenter__(self):
        print("Entering async context")
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        print("Exiting async context")
        
async def main():
    async with AsyncContextManager() as manager:
        async for num in async_counter(1, 5):
            print(num)
"#;

    pub const COMPLEX_INHERITANCE: &str = r#"
from abc import ABC, abstractmethod
from typing import Protocol, TypeVar, Generic

T = TypeVar('T')

class Animal(ABC):
    def __init__(self, name: str):
        self.name = name
    
    @abstractmethod
    def make_sound(self) -> str:
        pass
    
    @abstractmethod
    def move(self):
        pass

class Flyable(Protocol):
    def fly(self) -> None: ...

class Swimmable(Protocol):
    def swim(self) -> None: ...

class Dog(Animal):
    def make_sound(self) -> str:
        return "Woof!"
    
    def move(self):
        print(f"{self.name} is running")

class Duck(Animal, Flyable, Swimmable):
    def make_sound(self) -> str:
        return "Quack!"
    
    def move(self):
        print(f"{self.name} is waddling")
    
    def fly(self):
        print(f"{self.name} is flying")
    
    def swim(self):
        print(f"{self.name} is swimming")

class Container(Generic[T]):
    def __init__(self, value: T):
        self._value = value
    
    def get(self) -> T:
        return self._value
    
    def set(self, value: T) -> None:
        self._value = value

# Multiple inheritance with MRO
class A:
    def method(self):
        print("A")

class B(A):
    def method(self):
        print("B")
        super().method()

class C(A):
    def method(self):
        print("C")
        super().method()

class D(B, C):
    def method(self):
        print("D")
        super().method()
"#;

    pub const COMPREHENSIONS_AND_LAMBDAS: &str = r#"
# List comprehensions
squares = [x**2 for x in range(10)]
even_squares = [x**2 for x in range(10) if x % 2 == 0]
matrix = [[i+j for j in range(3)] for i in range(3)]

# Dict comprehensions
word_lengths = {word: len(word) for word in ["hello", "world", "python"]}
inverted_dict = {v: k for k, v in original_dict.items()}

# Set comprehensions
unique_lengths = {len(word) for word in text.split()}

# Generator expressions
sum_of_squares = sum(x**2 for x in range(100))
filtered = (x for x in data if x > threshold)

# Lambda functions
add = lambda x, y: x + y
sort_key = lambda item: item[1]
nested_lambda = lambda x: (lambda y: x + y)

# Complex comprehensions
result = [
    func(x)
    for sublist in nested_list
    for x in sublist
    if predicate(x)
]

# Walrus operator in comprehensions (Python 3.8+)
filtered_data = [y for x in data if (y := process(x)) is not None]
"#;

    pub const CONTEXT_MANAGERS: &str = r#"
from contextlib import contextmanager
import threading

class FileManager:
    def __init__(self, filename, mode):
        self.filename = filename
        self.mode = mode
        self.file = None
    
    def __enter__(self):
        self.file = open(self.filename, self.mode)
        return self.file
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.file:
            self.file.close()
        if exc_type is not None:
            print(f"Exception occurred: {exc_val}")
        return False  # Don't suppress exceptions

@contextmanager
def database_connection(host, port):
    conn = connect_to_db(host, port)
    try:
        yield conn
    finally:
        conn.close()

class ThreadSafeLock:
    def __init__(self):
        self._lock = threading.Lock()
    
    def __enter__(self):
        self._lock.acquire()
        return self
    
    def __exit__(self, *args):
        self._lock.release()

# Using multiple context managers
with FileManager("input.txt", "r") as infile, \
     FileManager("output.txt", "w") as outfile:
    data = infile.read()
    outfile.write(process(data))
"#;

    pub const METACLASSES: &str = r#"
from typing import Any

class SingletonMeta(type):
    _instances = {}
    
    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            cls._instances[cls] = super().__call__(*args, **kwargs)
        return cls._instances[cls]

class Singleton(metaclass=SingletonMeta):
    def __init__(self):
        self.value = 42

class AutoPropertyMeta(type):
    def __new__(mcs, name, bases, namespace):
        for key, value in namespace.items():
            if key.startswith('_') and not key.startswith('__'):
                prop_name = key[1:]
                namespace[prop_name] = property(
                    lambda self, k=key: getattr(self, k),
                    lambda self, v, k=key: setattr(self, k, v)
                )
        return super().__new__(mcs, name, bases, namespace)

class AutoProperty(metaclass=AutoPropertyMeta):
    def __init__(self):
        self._x = 10
        self._y = 20

# Custom metaclass with __prepare__
class OrderedMeta(type):
    @classmethod
    def __prepare__(mcs, name, bases, **kwargs):
        from collections import OrderedDict
        return OrderedDict()
    
    def __new__(mcs, name, bases, namespace, **kwargs):
        namespace['_order'] = list(namespace.keys())
        return super().__new__(mcs, name, bases, namespace)
"#;

    pub const TYPE_HINTS_AND_ANNOTATIONS: &str = r#"
from typing import (
    List, Dict, Tuple, Set, Optional, Union, Any, Callable,
    TypeVar, Generic, Protocol, Literal, Final, TypedDict,
    overload, cast, get_type_hints, NewType
)
from typing_extensions import TypeAlias, ParamSpec, Concatenate

T = TypeVar('T')
P = ParamSpec('P')
UserId = NewType('UserId', int)

# Type aliases
Vector = List[float]
Matrix = List[Vector]
JsonValue: TypeAlias = Union[None, bool, int, float, str, List['JsonValue'], Dict[str, 'JsonValue']]

# TypedDict
class PersonDict(TypedDict):
    name: str
    age: int
    email: Optional[str]

# Protocol
class Drawable(Protocol):
    def draw(self) -> None: ...

# Complex type hints
def complex_function(
    data: Dict[str, List[Tuple[int, str]]],
    callback: Callable[[int], Optional[str]],
    items: Union[List[T], Tuple[T, ...]],
    flags: Literal["read", "write", "append"] = "read"
) -> Optional[Dict[str, Any]]:
    pass

# Overloaded function
@overload
def process(x: int) -> str: ...

@overload
def process(x: str) -> int: ...

def process(x: Union[int, str]) -> Union[int, str]:
    if isinstance(x, int):
        return str(x)
    return len(x)

# Self type
from typing import Self

class Node:
    def __init__(self, value: Any):
        self.value = value
        self.next: Optional[Self] = None
    
    def append(self, value: Any) -> Self:
        self.next = Node(value)
        return self.next
"#;

    pub const DATACLASSES_AND_ATTRS: &str = r#"
from dataclasses import dataclass, field, InitVar, asdict, astuple
from typing import ClassVar, Optional
import attrs

@dataclass
class Point:
    x: float
    y: float
    
    def distance_from_origin(self) -> float:
        return (self.x ** 2 + self.y ** 2) ** 0.5

@dataclass(frozen=True)
class ImmutablePoint:
    x: float
    y: float

@dataclass
class Employee:
    name: str
    id: int
    salary: InitVar[float] = None
    _salary: float = field(init=False, repr=False)
    department: str = "Engineering"
    projects: list = field(default_factory=list)
    company: ClassVar[str] = "TechCorp"
    
    def __post_init__(self, salary: Optional[float]):
        if salary is not None:
            self._salary = salary
        else:
            self._salary = 50000.0

@attrs.define
class AttrsExample:
    x: int
    y: int = attrs.field(default=0)
    z: int = attrs.field(factory=int)
    
    @x.validator
    def check_x(self, attribute, value):
        if value < 0:
            raise ValueError("x must be non-negative")

# Slots with dataclass
@dataclass
class SlottedClass:
    __slots__ = ('x', 'y')
    x: int
    y: int
"#;

    pub const PATTERN_MATCHING: &str = r#"
# Pattern matching (Python 3.10+)
def process_command(command):
    match command:
        case ["quit"]:
            return "Quitting"
        case ["move", direction] if direction in ["north", "south", "east", "west"]:
            return f"Moving {direction}"
        case ["move", *directions]:
            return f"Moving in sequence: {directions}"
        case {"action": "jump", "height": h} if h > 0:
            return f"Jumping {h} units high"
        case {"action": action, **rest}:
            return f"Performing {action} with params {rest}"
        case Point(x=0, y=0):
            return "At origin"
        case Point(x=x, y=y) if x == y:
            return f"On diagonal at {x}"
        case [*items] if len(items) > 3:
            return f"Long list with {len(items)} items"
        case _:
            return "Unknown command"

def match_types(value):
    match value:
        case int(x) if x > 0:
            return f"Positive integer: {x}"
        case float(x):
            return f"Float: {x}"
        case str(s) if s.isupper():
            return f"Uppercase string: {s}"
        case list() as lst if len(lst) > 0:
            return f"Non-empty list with {len(lst)} items"
        case None:
            return "None value"
        case _:
            return "Other"
"#;

    pub const UNICODE_AND_SPECIAL_NAMES: &str = r#"
# Unicode identifiers
def 计算(参数1, 参数2):
    return 参数1 + 参数2

class МойКласс:
    def __init__(self):
        self.поле = "значение"
    
    def метод(self):
        return self.поле

مرحبا = "Hello in Arabic"
你好 = "Hello in Chinese"
𝛼 = 0.5
π = 3.14159

# Special method names
class SpecialMethods:
    def __init__(self):
        self.data = []
    
    def __len__(self):
        return len(self.data)
    
    def __getitem__(self, key):
        return self.data[key]
    
    def __setitem__(self, key, value):
        self.data[key] = value
    
    def __delitem__(self, key):
        del self.data[key]
    
    def __contains__(self, item):
        return item in self.data
    
    def __iter__(self):
        return iter(self.data)
    
    def __reversed__(self):
        return reversed(self.data)
    
    def __repr__(self):
        return f"SpecialMethods({self.data!r})"
    
    def __str__(self):
        return str(self.data)
    
    def __bytes__(self):
        return bytes(self.data)
    
    def __format__(self, format_spec):
        return format(self.data, format_spec)
    
    def __lt__(self, other):
        return self.data < other.data
    
    def __le__(self, other):
        return self.data <= other.data
"#;

    pub const MALFORMED_CODE: &str = r#"
# Missing colon
def broken(x)
    return x * 2

# Incorrect indentation
def bad_indent():
return "bad"

# Unclosed string
text = "hello world

# Missing parenthesis
print("Hello"

# Invalid syntax
class
    def method(self):
        pass

# Mixed tabs and spaces (will cause IndentationError)
def mixed():
	if True:
        print("mixed")

# Incomplete function
def incomplete(

# Invalid decorator
@
def decorated():
    pass
"#;

    pub const EMPTY_FILE: &str = "";

    pub const ONLY_COMMENTS: &str = r#"
# This is a comment-only file
# No actual code here

"""
This is a multi-line docstring
but there's no function or class to attach it to
"""

# TODO: Implement this module
# FIXME: Fix the bug
# NOTE: Important information
# WARNING: Be careful

'''
Another multi-line string
that serves as a comment
'''
"#;

    pub const NESTED_FUNCTIONS_AND_CLOSURES: &str = r#"
def outer_function(x):
    message = f"Outer value: {x}"
    
    def inner_function(y):
        nonlocal x
        x += y
        return f"{message}, Inner value: {y}, Sum: {x}"
    
    def another_inner():
        def deeply_nested():
            def very_deep():
                return x * 2
            return very_deep()
        return deeply_nested()
    
    return inner_function, another_inner

def make_multiplier(n):
    def multiplier(x):
        return x * n
    return multiplier

def decorator_factory(prefix):
    def decorator(func):
        def wrapper(*args, **kwargs):
            print(f"{prefix}: Calling {func.__name__}")
            result = func(*args, **kwargs)
            print(f"{prefix}: Done")
            return result
        return wrapper
    return decorator

# Nested classes
class Outer:
    class Inner:
        class DeepInner:
            value = 42
"#;

    pub const EXCEPTION_HANDLING: &str = r#"
class CustomError(Exception):
    pass

class ValidationError(ValueError):
    def __init__(self, field, value, message):
        self.field = field
        self.value = value
        super().__init__(message)

def complex_exception_handling():
    try:
        risky_operation()
    except (ValueError, TypeError) as e:
        print(f"Type or Value error: {e}")
    except CustomError:
        print("Custom error occurred")
        raise
    except Exception as e:
        print(f"Unexpected error: {e}")
        raise RuntimeError("Wrapped error") from e
    else:
        print("No exception occurred")
    finally:
        cleanup()

def context_manager_exceptions():
    try:
        with open("file.txt") as f:
            data = f.read()
    except FileNotFoundError:
        print("File not found")
    except PermissionError:
        print("Permission denied")
    except OSError as e:
        print(f"OS error: {e}")

# Exception chaining
def chained_exceptions():
    try:
        process_data()
    except DataError as e:
        raise ProcessingError("Failed to process") from e

# Suppressing exceptions
from contextlib import suppress

with suppress(FileNotFoundError):
    os.remove("tempfile.txt")
"#;

    pub const MODULE_AND_PACKAGE_IMPORTS: &str = r#"
# Various import styles
import os
import sys
import os.path
from typing import *
from collections import defaultdict, Counter, OrderedDict
from ..parent_package import module
from ...grandparent import something
from . import sibling
from .sibling import specific_function

# Conditional imports
import platform
if platform.system() == "Windows":
    import winreg
else:
    import pwd

# Import with alias
import numpy as np
import pandas as pd
from matplotlib import pyplot as plt

# Dynamic imports
import importlib
module_name = "dynamic_module"
dynamic_module = importlib.import_module(module_name)

# Import in function
def lazy_import():
    import expensive_module
    return expensive_module.process()

# __all__ definition
__all__ = ["public_function", "PublicClass"]

def public_function():
    pass

def _private_function():
    pass

class PublicClass:
    pass

class _PrivateClass:
    pass
"#;

    pub const GLOBAL_AND_NONLOCAL: &str = r#"
global_var = 100

def modify_global():
    global global_var
    global_var += 1
    
    global new_global
    new_global = "created inside function"

def outer():
    outer_var = 10
    
    def middle():
        middle_var = 20
        
        def inner():
            nonlocal middle_var, outer_var
            middle_var += 1
            outer_var += 1
            
            global global_var
            global_var += 1
            
            local_var = 30
            return local_var + middle_var + outer_var + global_var
        
        return inner()
    
    return middle()

class Scope:
    class_var = "class"
    
    def method(self):
        # Access class variable
        print(self.class_var)
        print(Scope.class_var)
        
        # Modify instance variable
        self.instance_var = "instance"
        
        # This creates a local variable, doesn't modify class var
        class_var = "local"
"#;

    pub const LARGE_FILE: &str = r#"
def func1(param): return param + 1
def func2(param): return param + 2
def func3(param): return param + 3
def func4(param): return param + 4
def func5(param): return param + 5

class Class1:
    def method1(self): pass
    def method2(self): pass

class Class2:
    def method1(self): pass
    def method2(self): pass

class Class3:
    def method1(self): pass
    def method2(self): pass

class Class4:
    def method1(self): pass
    def method2(self): pass

class Class5:
    def method1(self): pass
    def method2(self): pass

variable1 = 1
variable2 = 2
variable3 = 3
variable4 = 4
variable5 = 5

list1 = [1, 2, 3]
list2 = [4, 5, 6]
list3 = [7, 8, 9]
dict1 = {'a': 1}
dict2 = {'b': 2}
dict3 = {'c': 3}

for i in range(10):
    print(i)

if True:
    x = 1
else:
    x = 2

try:
    y = risky()
except:
    y = 0

while False:
    pass

with open('file') as f:
    content = f.read()
"#;
}