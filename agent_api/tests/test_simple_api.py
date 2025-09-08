"""
Comprehensive tests for simple_api.py
"""

import pytest
import tempfile
import sqlite3
from pathlib import Path
from typing import Dict, Any

import sys
sys.path.insert(0, str(Path(__file__).parent.parent))

from simple_api import CodeGraphAPI, Symbol, Edge, analyze_codebase, find_related_code


class TestFixtures:
    """Shared test fixtures"""
    
    @staticmethod
    def create_test_database(with_complex_graph: bool = False) -> Path:
        """Create a test SQLite database with sample data"""
        temp_dir = tempfile.mkdtemp()
        db_dir = Path(temp_dir) / ".reviewbot"
        db_dir.mkdir()
        db_path = db_dir / "graph.db"
        
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Create schema
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
        
        # Insert basic test data
        test_files = [
            (1, "src/main.py", None, "python"),
            (2, "src/auth.py", None, "python"),
            (3, "src/database.py", None, "python"),
            (4, "src/utils.py", None, "python"),
            (5, "tests/test_main.py", None, "python")
        ]
        cursor.executemany("INSERT INTO files VALUES (?, ?, ?, ?)", test_files)
        
        test_symbols = [
            # main.py
            (1, 1, "main", "main", "function", 10, None, "def main()"),
            (2, 1, "process_data", "process_data", "function", 20, None, "def process_data(data: str)"),
            (3, 1, "validate_input", "validate_input", "function", 40, None, "def validate_input(input: str) -> bool"),
            
            # auth.py
            (4, 2, "AuthService", "AuthService", "class", 5, None, "class AuthService"),
            (5, 2, "AuthService::authenticate", "authenticate", "method", 10, None, "def authenticate(user: str, password: str) -> bool"),
            (6, 2, "AuthService::check_permission", "check_permission", "method", 25, None, "def check_permission(user: str, resource: str) -> bool"),
            (7, 2, "hash_password", "hash_password", "function", 50, None, "def hash_password(password: str) -> str"),
            
            # database.py
            (8, 3, "Database", "Database", "class", 5, None, "class Database"),
            (9, 3, "Database::connect", "connect", "method", 10, None, "def connect(self)"),
            (10, 3, "Database::query", "query", "method", 20, None, "def query(sql: str, params: dict = None)"),
            (11, 3, "Database::execute", "execute", "method", 35, None, "def execute(sql: str) -> int"),
            (12, 3, "DatabasePool", "DatabasePool", "class", 60, None, "class DatabasePool"),
            
            # utils.py
            (13, 4, "logger", "logger", "variable", 5, None, None),
            (14, 4, "format_date", "format_date", "function", 10, None, "def format_date(date: datetime) -> str"),
            (15, 4, "parse_config", "parse_config", "function", 25, None, "def parse_config(path: str) -> dict"),
            
            # test_main.py
            (16, 5, "test_main", "test_main", "function", 10, None, "def test_main()"),
            (17, 5, "test_process_data", "test_process_data", "function", 20, None, "def test_process_data()"),
        ]
        cursor.executemany("INSERT INTO symbols VALUES (?, ?, ?, ?, ?, ?, ?, ?)", test_symbols)
        
        # Insert edges
        test_edges = [
            # main calls
            ("main", "process_data", "calls", "syntactic"),
            ("main", "validate_input", "calls", "syntactic"),
            ("main", "AuthService::authenticate", "calls", "syntactic"),
            
            # process_data calls
            ("process_data", "validate_input", "calls", "syntactic"),
            ("process_data", "Database::query", "calls", "syntactic"),
            
            # AuthService methods
            ("AuthService::authenticate", "hash_password", "calls", "syntactic"),
            ("AuthService::authenticate", "Database::query", "calls", "syntactic"),
            ("AuthService::check_permission", "Database::query", "calls", "syntactic"),
            
            # Database methods
            ("Database::query", "Database::connect", "calls", "syntactic"),
            ("Database::execute", "Database::connect", "calls", "syntactic"),
            
            # Imports
            ("src/main.py", "src/auth.py", "imports", "syntactic"),
            ("src/main.py", "src/database.py", "imports", "syntactic"),
            ("src/auth.py", "src/database.py", "imports", "syntactic"),
            ("src/auth.py", "src/utils.py", "imports", "syntactic"),
            
            # Tests
            ("test_main", "main", "calls", "syntactic"),
            ("test_process_data", "process_data", "calls", "syntactic"),
        ]
        
        if with_complex_graph:
            # Add circular dependency for testing
            test_edges.append(("Database::connect", "AuthService::authenticate", "calls", "syntactic"))
            
            # Add more complex relationships
            test_edges.extend([
                ("validate_input", "format_date", "calls", "syntactic"),
                ("parse_config", "logger", "uses", "syntactic"),
                ("DatabasePool", "Database", "extends", "syntactic"),
            ])
        
        cursor.executemany("INSERT INTO edges (src, dst, edge_type, resolution) VALUES (?, ?, ?, ?)", test_edges)
        
        conn.commit()
        conn.close()
        
        return temp_dir


class TestCodeGraphAPI:
    """Test the main CodeGraphAPI class"""
    
    def test_init(self):
        """Test initialization"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        assert str(api.repo_path) == str(repo_path)
        assert api.db_path.exists()
        assert api.conn is not None
        
        api.close()
    
    def test_init_missing_db(self):
        """Test initialization with missing database"""
        with tempfile.TemporaryDirectory() as temp_dir:
            with pytest.raises(FileNotFoundError, match="Database not found"):
                CodeGraphAPI(temp_dir)
    
    def test_get_symbol(self):
        """Test getting a symbol by FQN"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Get existing symbol
        symbol = api.get_symbol("AuthService::authenticate")
        assert symbol is not None
        assert isinstance(symbol, Symbol)
        assert symbol.fqn == "AuthService::authenticate"
        assert symbol.name == "authenticate"
        assert symbol.kind == "method"
        assert symbol.file == "src/auth.py"
        assert symbol.line == 10
        assert "authenticate" in symbol.signature
        
        # Get non-existent symbol
        symbol = api.get_symbol("NonExistent")
        assert symbol is None
        
        api.close()
    
    def test_find_symbols(self):
        """Test finding symbols by pattern"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Find all auth-related symbols
        symbols = api.find_symbols("auth")
        assert len(symbols) >= 2
        assert any(s.name == "authenticate" for s in symbols)
        assert any(s.name == "AuthService" for s in symbols)
        
        # Find by kind
        functions = api.find_symbols("", kind="function")
        assert all(s.kind == "function" for s in functions)
        assert len(functions) >= 5
        
        # Find with specific pattern
        test_symbols = api.find_symbols("test_")
        assert all("test_" in s.name for s in test_symbols)
        
        api.close()
    
    def test_get_file_symbols(self):
        """Test getting all symbols in a file"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        symbols = api.get_file_symbols("src/auth.py")
        assert len(symbols) == 4  # AuthService, authenticate, check_permission, hash_password
        assert all(s.file == "src/auth.py" for s in symbols)
        
        # Check ordering by line number
        lines = [s.line for s in symbols]
        assert lines == sorted(lines)
        
        api.close()
    
    def test_get_callers(self):
        """Test finding functions that call a symbol"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Find callers of Database::query
        callers = api.get_callers("Database::query")
        assert "process_data" in callers
        assert "AuthService::authenticate" in callers
        assert "AuthService::check_permission" in callers
        
        # Find callers of main (should be test function)
        main_callers = api.get_callers("main")
        assert "test_main" in main_callers
        
        api.close()
    
    def test_get_callees(self):
        """Test finding functions called by a symbol"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Find what main calls
        callees = api.get_callees("main")
        assert "process_data" in callees
        assert "validate_input" in callees
        assert "AuthService::authenticate" in callees
        
        # Find what authenticate calls
        auth_callees = api.get_callees("AuthService::authenticate")
        assert "hash_password" in auth_callees
        assert "Database::query" in auth_callees
        
        api.close()
    
    def test_get_edges(self):
        """Test getting edges with filters"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Get all edges from main
        edges = api.get_edges(source="main")
        assert len(edges) >= 3
        assert all(e.source == "main" for e in edges)
        
        # Get all calls to Database::query
        edges = api.get_edges(target="Database::query", edge_type="calls")
        assert len(edges) >= 3
        assert all(e.target == "Database::query" and e.edge_type == "calls" for e in edges)
        
        # Get imports only
        edges = api.get_edges(edge_type="imports")
        assert all(e.edge_type == "imports" for e in edges)
        
        api.close()
    
    def test_find_paths(self):
        """Test finding paths between symbols"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Find path from main to Database::query
        paths = api.find_paths("main", "Database::query", max_depth=3)
        assert len(paths) > 0
        
        # Verify paths are valid
        for path in paths:
            assert path[0] == "main"
            assert path[-1] == "Database::query"
            assert len(path) <= 4  # max_depth + 1
        
        # No path should exist in reverse
        paths = api.find_paths("Database::query", "main", max_depth=5)
        assert len(paths) == 0
        
        api.close()
    
    def test_get_dependencies(self):
        """Test getting symbol dependencies"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        deps = api.get_dependencies("AuthService::authenticate")
        
        assert "calls" in deps
        assert "hash_password" in deps["calls"]
        assert "Database::query" in deps["calls"]
        
        # main imports auth.py and database.py
        main_deps = api.get_dependencies("src/main.py")
        if "imports" in main_deps:
            assert "src/auth.py" in main_deps["imports"]
        
        api.close()
    
    def test_get_impact_radius(self):
        """Test finding impacted symbols"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Changes to Database::query should impact multiple functions
        impacted = api.get_impact_radius("Database::query", max_depth=2)
        assert "process_data" in impacted
        assert "AuthService::authenticate" in impacted
        
        # With depth=3, should reach main
        impacted = api.get_impact_radius("Database::query", max_depth=3)
        assert "main" in impacted
        
        api.close()
    
    def test_get_stats(self):
        """Test getting statistics"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        stats = api.get_stats()
        
        assert "total_files" in stats
        assert stats["total_files"] == 5
        
        assert "total_symbols" in stats
        assert stats["total_symbols"] == 17
        
        assert "total_edges" in stats
        assert stats["total_edges"] >= 16
        
        assert "symbols_by_kind" in stats
        assert stats["symbols_by_kind"]["function"] >= 5
        assert stats["symbols_by_kind"]["class"] >= 2
        assert stats["symbols_by_kind"]["method"] >= 5
        
        assert "edges_by_type" in stats
        assert stats["edges_by_type"]["calls"] >= 10
        assert stats["edges_by_type"]["imports"] >= 4
        
        api.close()
    
    def test_find_cycles(self):
        """Test cycle detection"""
        repo_path = TestFixtures.create_test_database(with_complex_graph=True)
        api = CodeGraphAPI(repo_path)
        
        cycles = api.find_cycles()
        
        # Should find the circular dependency we added
        assert len(cycles) >= 1
        
        # Verify cycle contains expected symbols
        cycle_symbols = set()
        for cycle in cycles:
            cycle_symbols.update(cycle)
        
        assert "Database::connect" in cycle_symbols
        assert "AuthService::authenticate" in cycle_symbols
        
        api.close()


class TestConvenienceFunctions:
    """Test the convenience functions"""
    
    def test_analyze_codebase(self):
        """Test the analyze_codebase function"""
        repo_path = TestFixtures.create_test_database()
        
        analysis = analyze_codebase(str(repo_path))
        
        assert "stats" in analysis
        assert analysis["stats"]["total_symbols"] == 17
        
        assert "cycles" in analysis
        assert isinstance(analysis["cycles"], list)
        
        assert "entry_points" in analysis
        assert isinstance(analysis["entry_points"], list)
        # main and test functions should be entry points
        assert any("main" in ep for ep in analysis["entry_points"])
        
        assert "complex_functions" in analysis
        assert isinstance(analysis["complex_functions"], list)
    
    def test_find_related_code(self):
        """Test the find_related_code function"""
        repo_path = TestFixtures.create_test_database()
        
        related = find_related_code(str(repo_path), "process_data")
        
        assert related["symbol"] == "process_data"
        
        assert "callers" in related
        assert "main" in related["callers"]
        
        assert "callees" in related
        assert "validate_input" in related["callees"]
        assert "Database::query" in related["callees"]
        
        assert "impact" in related
        assert "main" in related["impact"]
        
        assert "dependencies" in related
        assert "calls" in related["dependencies"]


class TestUncoveredLines:
    """Test uncovered lines for better coverage"""
    
    def test_find_paths_max_depth_reached(self):
        """Test find_paths when max_depth is reached"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Test with depth 1 - should limit paths
        paths = api.find_paths("main", "database.execute_query", max_depth=1)
        # With depth 1, can't reach execute_query from main
        assert len(paths) == 0
        
        api.close()
    
    def test_get_dependencies_edge_type_handling(self):
        """Test edge type handling in get_dependencies"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Add custom edge type
        conn = api.conn
        cursor = conn.cursor()
        cursor.execute("INSERT INTO edges (src, dst, edge_type) VALUES (?, ?, ?)",
                      ("main", "custom_module", "custom_edge"))
        conn.commit()
        
        deps = api.get_dependencies("main")
        # Should handle custom edge type
        assert "custom_edge" in deps or "custom_module" in [item for sublist in deps.values() for item in sublist]
        
        api.close()
    
    def test_analyze_codebase_complex_functions(self):
        """Test analyze_codebase finding complex functions"""
        repo_path = TestFixtures.create_test_database()
        
        # Add a complex function with many callees
        conn = sqlite3.connect(Path(repo_path) / ".reviewbot" / "graph.db")
        cursor = conn.cursor()
        
        # Add a complex function
        cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES (?, ?, ?, ?, ?, ?)",
                      ("complex.func", "func", "function", 1, 1, "def func()"))
        
        # Add many callees
        for i in range(15):
            cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES (?, ?, ?, ?, ?, ?)",
                          (f"helper{i}", f"helper{i}", "function", i+10, 1, f"def helper{i}()"))
            cursor.execute("INSERT INTO edges (src, dst, edge_type) VALUES (?, ?, ?)",
                          ("complex.func", f"helper{i}", "calls"))
        
        conn.commit()
        conn.close()
        
        analysis = analyze_codebase(repo_path)
        
        # Should find the complex function
        assert len(analysis["complex_functions"]) > 0
        complex_funcs = [f["function"] for f in analysis["complex_functions"]]
        assert "complex.func" in complex_funcs
    
    def test_main_execution(self):
        """Test the __main__ execution block"""
        import subprocess
        import sys
        
        repo_path = TestFixtures.create_test_database()
        
        # Test with no arguments
        result = subprocess.run(
            [sys.executable, "simple_api.py"],
            capture_output=True,
            text=True
        )
        assert result.returncode == 1
        assert "Usage:" in result.stdout or "Usage:" in result.stderr
        
        # Test with valid repo path
        result = subprocess.run(
            [sys.executable, "simple_api.py", str(repo_path)],
            capture_output=True,
            text=True
        )
        assert result.returncode == 0
        assert "Codebase Statistics:" in result.stdout
        assert "Symbols:" in result.stdout


class TestEdgeCases:
    """Test edge cases and error handling"""
    
    def test_empty_database(self):
        """Test with empty database"""
        temp_dir = tempfile.mkdtemp()
        db_dir = Path(temp_dir) / ".reviewbot"
        db_dir.mkdir()
        db_path = db_dir / "graph.db"
        
        # Create empty database with schema only
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT, name TEXT, kind TEXT, line INTEGER, file_id INTEGER, signature TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        conn.commit()
        conn.close()
        
        api = CodeGraphAPI(temp_dir)
        
        # Should handle empty results gracefully
        assert api.get_symbol("anything") is None
        assert api.find_symbols("") == []
        assert api.get_callers("anything") == []
        assert api.find_cycles() == []
        
        stats = api.get_stats()
        assert stats["total_symbols"] == 0
        assert stats["total_edges"] == 0
        
        api.close()
    
    def test_special_characters_in_names(self):
        """Test handling special characters in symbol names"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Test with SQL wildcard characters
        symbols = api.find_symbols("%")
        assert isinstance(symbols, list)
        
        symbols = api.find_symbols("_")
        assert isinstance(symbols, list)
        
        # Test with quotes
        symbol = api.get_symbol("doesn't exist")
        assert symbol is None
        
        api.close()
    
    def test_large_depth_queries(self):
        """Test queries with large depth values"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # Should handle large depths without issues
        impacted = api.get_impact_radius("Database::query", max_depth=1000)
        assert isinstance(impacted, set)
        
        paths = api.find_paths("main", "Database::query", max_depth=1000)
        assert isinstance(paths, list)
        
        api.close()
    
    def test_disconnected_graph(self):
        """Test with disconnected components"""
        repo_path = TestFixtures.create_test_database()
        api = CodeGraphAPI(repo_path)
        
        # format_date is not connected to main
        paths = api.find_paths("main", "format_date", max_depth=10)
        assert len(paths) == 0
        
        # Should still find the symbols
        symbol = api.get_symbol("format_date")
        assert symbol is not None
        assert symbol.name == "format_date"
        
        api.close()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])