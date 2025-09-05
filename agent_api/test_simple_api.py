"""
Quick test for the simplified API
"""

import tempfile
import sqlite3
from pathlib import Path
from simple_api import CodeGraphAPI, analyze_codebase, find_related_code


def create_test_db():
    """Create a test database"""
    temp_dir = tempfile.mkdtemp()
    db_dir = Path(temp_dir) / ".reviewbot"
    db_dir.mkdir()
    db_path = db_dir / "graph.db"
    
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    # Create minimal schema
    cursor.execute("""
        CREATE TABLE files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL
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
            signature TEXT
        )
    """)
    
    cursor.execute("""
        CREATE TABLE edges (
            id INTEGER PRIMARY KEY,
            src TEXT NOT NULL,
            dst TEXT NOT NULL,
            edge_type TEXT NOT NULL
        )
    """)
    
    # Insert test data
    cursor.execute("INSERT INTO files (id, path) VALUES (1, 'main.py')")
    cursor.execute("INSERT INTO files (id, path) VALUES (2, 'utils.py')")
    
    cursor.executemany("""
        INSERT INTO symbols (file_id, fqn, name, kind, line, signature)
        VALUES (?, ?, ?, ?, ?, ?)
    """, [
        (1, "main", "main", "function", 1, "def main()"),
        (1, "process", "process", "function", 10, "def process(data)"),
        (2, "validate", "validate", "function", 5, "def validate(input)"),
        (2, "Database", "Database", "class", 15, "class Database"),
        (2, "Database::query", "query", "method", 20, "def query(sql)")
    ])
    
    cursor.executemany("""
        INSERT INTO edges (src, dst, edge_type)
        VALUES (?, ?, ?)
    """, [
        ("main", "process", "calls"),
        ("process", "validate", "calls"),
        ("process", "Database::query", "calls"),
        ("main", "utils.py", "imports")
    ])
    
    conn.commit()
    conn.close()
    
    return temp_dir


def test_basic_queries():
    """Test basic API functionality"""
    repo_path = create_test_db()
    
    print("Testing CodeGraphAPI...")
    api = CodeGraphAPI(repo_path)
    
    # Test get_symbol
    symbol = api.get_symbol("main")
    assert symbol is not None
    assert symbol.name == "main"
    assert symbol.kind == "function"
    print("✓ get_symbol works")
    
    # Test find_symbols
    functions = api.find_symbols("", kind="function")
    assert len(functions) == 3
    print(f"✓ find_symbols found {len(functions)} functions")
    
    # Test get_callers
    callers = api.get_callers("process")
    assert "main" in callers
    print("✓ get_callers works")
    
    # Test get_callees
    callees = api.get_callees("process")
    assert "validate" in callees
    assert "Database::query" in callees
    print(f"✓ get_callees found {callees}")
    
    # Test get_impact_radius
    impact = api.get_impact_radius("Database::query")
    assert "process" in impact
    print(f"✓ get_impact_radius found {len(impact)} impacted symbols")
    
    # Test stats
    stats = api.get_stats()
    assert stats["total_symbols"] == 5
    assert stats["total_edges"] == 4
    print("✓ get_stats works")
    
    api.close()
    
    # Test convenience functions
    print("\nTesting convenience functions...")
    
    analysis = analyze_codebase(repo_path)
    assert "stats" in analysis
    assert analysis["stats"]["total_symbols"] == 5
    print("✓ analyze_codebase works")
    
    related = find_related_code(repo_path, "process")
    assert "main" in related["callers"]
    assert "validate" in related["callees"]
    print("✓ find_related_code works")
    
    print("\n✅ All tests passed!")


if __name__ == "__main__":
    test_basic_queries()