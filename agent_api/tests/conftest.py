"""
Pytest configuration and fixtures for agent_api tests
"""

import pytest
import sqlite3
import tempfile
import shutil
from pathlib import Path
from typing import Generator


@pytest.fixture
def temp_dir() -> Generator[Path, None, None]:
    """Create a temporary directory for test files"""
    temp = tempfile.mkdtemp()
    yield Path(temp)
    shutil.rmtree(temp)


@pytest.fixture
def mock_db(temp_dir: Path) -> Generator[Path, None, None]:
    """Create a mock SQLite database with test data"""
    db_dir = temp_dir / ".reviewbot"
    db_dir.mkdir()
    db_path = db_dir / "graph.db"
    
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    # Create tables matching Consilium schema
    cursor.execute("""
        CREATE TABLE files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL,
            hash TEXT,
            language TEXT
        )
    """)
    
    cursor.execute("""
        CREATE TABLE symbols (
            id INTEGER PRIMARY KEY,
            file_id INTEGER,
            fqn TEXT NOT NULL,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            line INTEGER,
            column INTEGER,
            signature TEXT,
            docstring TEXT,
            FOREIGN KEY (file_id) REFERENCES files(id)
        )
    """)
    
    cursor.execute("""
        CREATE TABLE edges (
            id INTEGER PRIMARY KEY,
            src TEXT NOT NULL,
            dst TEXT NOT NULL,
            edge_type TEXT NOT NULL,
            resolution TEXT
        )
    """)
    
    # Insert test data
    cursor.execute("INSERT INTO files (id, path, language) VALUES (1, 'src/main.py', 'python')")
    cursor.execute("INSERT INTO files (id, path, language) VALUES (2, 'src/auth.py', 'python')")
    cursor.execute("INSERT INTO files (id, path, language) VALUES (3, 'src/database.py', 'python')")
    
    # Insert test symbols
    test_symbols = [
        (1, 1, "main", "main", "function", 10, 0, "def main()", None),
        (2, 1, "process_data", "process_data", "function", 20, 0, "def process_data(data: str)", None),
        (3, 2, "AuthService", "AuthService", "class", 5, 0, "class AuthService", None),
        (4, 2, "AuthService::authenticate", "authenticate", "method", 10, 4, "def authenticate(user: str, password: str) -> bool", None),
        (5, 2, "AuthService::validate_token", "validate_token", "method", 25, 4, "def validate_token(token: str) -> bool", None),
        (6, 3, "Database", "Database", "class", 3, 0, "class Database", None),
        (7, 3, "Database::query", "query", "method", 10, 4, "def query(sql: str, params: dict)", None),
        (8, 3, "Database::execute", "execute", "method", 20, 4, "def execute(sql: str)", None),
        (9, 2, "check_permission", "check_permission", "function", 40, 0, "def check_permission(user: str, resource: str)", None),
        (10, 1, "complex_function", "complex_function", "function", 50, 0, "def complex_function(a, b, c, d, e, f)", None),
    ]
    
    cursor.executemany("""
        INSERT INTO symbols (id, file_id, fqn, name, kind, line, column, signature, docstring)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, test_symbols)
    
    # Insert test edges
    test_edges = [
        ("main", "process_data", "calls", "syntactic"),
        ("process_data", "Database::query", "calls", "syntactic"),
        ("AuthService::authenticate", "Database::query", "calls", "syntactic"),
        ("AuthService::authenticate", "AuthService::validate_token", "calls", "syntactic"),
        ("main", "AuthService::authenticate", "calls", "syntactic"),
        ("check_permission", "Database::query", "calls", "syntactic"),
        ("complex_function", "process_data", "calls", "syntactic"),
        ("complex_function", "AuthService::authenticate", "calls", "syntactic"),
        ("complex_function", "Database::execute", "calls", "syntactic"),
        ("src/main.py", "src/auth.py", "imports", "syntactic"),
        ("src/auth.py", "src/database.py", "imports", "syntactic"),
    ]
    
    cursor.executemany("""
        INSERT INTO edges (src, dst, edge_type, resolution)
        VALUES (?, ?, ?, ?)
    """, test_edges)
    
    conn.commit()
    conn.close()
    
    yield db_path


@pytest.fixture
def mock_repo(temp_dir: Path, mock_db: Path) -> Path:
    """Create a mock repository with test files"""
    # Create source files
    src_dir = temp_dir / "src"
    src_dir.mkdir()
    
    # main.py
    (src_dir / "main.py").write_text("""
def main():
    data = get_input()
    result = process_data(data)
    return result

def process_data(data: str):
    db = Database()
    return db.query("SELECT * FROM users WHERE name = " + data)

def complex_function(a, b, c, d, e, f):
    # This function is too complex
    for i in range(10):
        if a > b:
            process_data(str(i))
        else:
            authenticate("user", "pass")
    return None
""")
    
    # auth.py
    (src_dir / "auth.py").write_text("""
from database import Database

class AuthService:
    def authenticate(self, user: str, password: str) -> bool:
        db = Database()
        result = db.query(f"SELECT * FROM users WHERE name = {user}")
        return self.validate_token(result)
    
    def validate_token(self, token: str) -> bool:
        return token == "valid"

def check_permission(user: str, resource: str):
    db = Database()
    return db.query(f"SELECT * FROM permissions WHERE user = {user}")
""")
    
    # database.py
    (src_dir / "database.py").write_text("""
class Database:
    def query(self, sql: str, params: dict = None):
        # Execute SQL query
        return []
    
    def execute(self, sql: str):
        # Execute SQL command
        pass
""")
    
    return temp_dir


@pytest.fixture
def sample_symbols():
    """Provide sample Symbol objects for testing"""
    from agent_api.models import Symbol, SymbolKind, Location
    
    return [
        Symbol(
            fqn="AuthService::authenticate",
            name="authenticate",
            kind=SymbolKind.METHOD,
            location=Location(file="src/auth.py", line=10),
            signature="def authenticate(user: str, password: str) -> bool",
            analyzer="test",
            confidence=1.0
        ),
        Symbol(
            fqn="Database::query",
            name="query",
            kind=SymbolKind.METHOD,
            location=Location(file="src/database.py", line=10),
            signature="def query(sql: str, params: dict)",
            analyzer="test",
            confidence=1.0
        ),
        Symbol(
            fqn="process_data",
            name="process_data",
            kind=SymbolKind.FUNCTION,
            location=Location(file="src/main.py", line=20),
            signature="def process_data(data: str)",
            analyzer="test",
            confidence=1.0
        ),
    ]


@pytest.fixture
def sample_security_issues():
    """Provide sample SecurityIssue objects for testing"""
    from agent_api.models import SecurityIssue, Severity, Location
    
    return [
        SecurityIssue(
            issue_id="sql_injection_001",
            type="sql_injection",
            severity=Severity.CRITICAL,
            location=Location(file="src/auth.py", line=6),
            description="SQL injection vulnerability in authenticate method",
            evidence=["db.query(f\"SELECT * FROM users WHERE name = {user}\")"],
            fix_suggestion="Use parameterized queries",
            confidence=0.9,
            cwe_id="CWE-89"
        ),
        SecurityIssue(
            issue_id="missing_auth_001",
            type="missing_authentication",
            severity=Severity.HIGH,
            location=Location(file="src/main.py", line=10),
            description="Endpoint lacks authentication check",
            evidence=["No auth decorator found"],
            fix_suggestion="Add authentication middleware",
            confidence=0.8,
            cwe_id="CWE-306"
        ),
    ]