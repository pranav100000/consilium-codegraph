"""
Tests for agent_tools.py - flexible tools for code review agents
"""

import unittest
import sqlite3
import tempfile
import os
from pathlib import Path
from typing import Dict, List, Any

import sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from agent_tools import CodeTools


class TestFixtures:
    """Create test database with realistic code graph data"""
    
    @staticmethod
    def create_test_database(db_path: str) -> None:
        """Create a test database with sample data"""
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Create tables
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT,
                language TEXT
            )
        """)
        
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY,
                file_id INTEGER NOT NULL,
                fqn TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                line INTEGER,
                signature TEXT,
                FOREIGN KEY (file_id) REFERENCES files(id)
            )
        """)
        
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS edges (
                id INTEGER PRIMARY KEY,
                src TEXT NOT NULL,
                dst TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                FOREIGN KEY (src) REFERENCES symbols(fqn),
                FOREIGN KEY (dst) REFERENCES symbols(fqn)
            )
        """)
        
        # Insert test files
        files = [
            (1, "app/main.py", "hash1", "python"),
            (2, "app/auth.py", "hash2", "python"),
            (3, "app/database.py", "hash3", "python"),
            (4, "app/utils.py", "hash4", "python"),
            (5, "tests/test_main.py", "hash5", "python"),
            (6, "app/api/endpoints.py", "hash6", "python"),
            (7, "app/models/user.py", "hash7", "python"),
        ]
        cursor.executemany("INSERT INTO files VALUES (?, ?, ?, ?)", files)
        
        # Insert test symbols
        symbols = [
            # main.py
            (1, 1, "app.main.main", "main", "function", 10, "def main()"),
            (2, 1, "app.main.process_request", "process_request", "function", 20, "def process_request(request)"),
            (3, 1, "app.main.validate_input", "validate_input", "function", 30, "def validate_input(data)"),
            
            # auth.py
            (4, 2, "app.auth.authenticate", "authenticate", "function", 10, "def authenticate(user, password)"),
            (5, 2, "app.auth.check_permissions", "check_permissions", "function", 20, "def check_permissions(user, action)"),
            (6, 2, "app.auth.AuthManager", "AuthManager", "class", 30, "class AuthManager"),
            (7, 2, "app.auth.AuthManager.login", "login", "method", 35, "def login(self, user, password)"),
            
            # database.py
            (8, 3, "app.database.execute_query", "execute_query", "function", 10, "def execute_query(sql)"),
            (9, 3, "app.database.get_user", "get_user", "function", 20, "def get_user(user_id)"),
            (10, 3, "app.database.DatabaseConnection", "DatabaseConnection", "class", 30, "class DatabaseConnection"),
            (11, 3, "app.database.DatabaseConnection.connect", "connect", "method", 35, "def connect(self)"),
            
            # utils.py
            (12, 4, "app.utils.logger", "logger", "variable", 5, "logger = logging.getLogger()"),
            (13, 4, "app.utils.format_response", "format_response", "function", 10, "def format_response(data)"),
            (14, 4, "app.utils.validate_email", "validate_email", "function", 20, "def validate_email(email)"),
            
            # test_main.py
            (15, 5, "tests.test_main.test_process_request", "test_process_request", "function", 10, "def test_process_request()"),
            (16, 5, "tests.test_main.test_validate_input", "test_validate_input", "function", 20, "def test_validate_input()"),
            
            # api/endpoints.py
            (17, 6, "app.api.endpoints.get_users", "get_users", "function", 10, "def get_users()"),
            (18, 6, "app.api.endpoints.create_user", "create_user", "function", 20, "def create_user(data)"),
            (19, 6, "app.api.endpoints.delete_user", "delete_user", "function", 30, "def delete_user(user_id)"),
            
            # models/user.py
            (20, 7, "app.models.user.User", "User", "class", 10, "class User"),
            (21, 7, "app.models.user.User.save", "save", "method", 20, "def save(self)"),
            (22, 7, "app.models.user.User.delete", "delete", "method", 30, "def delete(self)"),
        ]
        cursor.executemany("INSERT INTO symbols VALUES (?, ?, ?, ?, ?, ?, ?)", symbols)
        
        # Insert test edges
        edges = [
            # Call edges
            (1, "app.main.main", "app.main.process_request", "calls"),
            (2, "app.main.process_request", "app.main.validate_input", "calls"),
            (3, "app.main.process_request", "app.auth.authenticate", "calls"),
            (4, "app.main.process_request", "app.database.get_user", "calls"),
            (5, "app.auth.authenticate", "app.database.get_user", "calls"),
            (6, "app.auth.AuthManager.login", "app.auth.authenticate", "calls"),
            (7, "app.database.get_user", "app.database.execute_query", "calls"),
            (8, "app.api.endpoints.get_users", "app.database.execute_query", "calls"),
            (9, "app.api.endpoints.create_user", "app.models.user.User.save", "calls"),
            (10, "app.api.endpoints.delete_user", "app.models.user.User.delete", "calls"),
            
            # Import edges
            (11, "app.main", "app.auth", "imports"),
            (12, "app.main", "app.database", "imports"),
            (13, "app.auth", "app.database", "imports"),
            (14, "tests.test_main", "app.main", "imports"),
            (15, "app.api.endpoints", "app.models.user", "imports"),
            
            # Uses edges
            (16, "app.main.process_request", "app.utils.logger", "uses"),
            (17, "app.auth.authenticate", "app.utils.logger", "uses"),
            (18, "app.database.execute_query", "app.utils.logger", "uses"),
            
            # Inherits edges
            (19, "app.models.user.User", "object", "inherits"),
            (20, "app.auth.AuthManager", "object", "inherits"),
        ]
        cursor.executemany("INSERT INTO edges VALUES (?, ?, ?, ?)", edges)
        
        conn.commit()
        conn.close()


class TestCodeTools(unittest.TestCase):
    """Test the CodeTools class"""
    
    def setUp(self):
        """Create test database"""
        self.temp_dir = tempfile.mkdtemp()
        self.repo_path = Path(self.temp_dir)
        self.db_dir = self.repo_path / ".reviewbot"
        self.db_dir.mkdir()
        self.db_path = self.db_dir / "graph.db"
        
        TestFixtures.create_test_database(str(self.db_path))
        self.tools = CodeTools(str(self.repo_path))
    
    def tearDown(self):
        """Clean up test database"""
        self.tools.close()
        import shutil
        shutil.rmtree(self.temp_dir)
    
    def test_initialization(self):
        """Test CodeTools initialization"""
        self.assertEqual(self.tools.repo_path, self.repo_path)
        self.assertEqual(self.tools.db_path, self.db_path)
        self.assertIsNotNone(self.tools.conn)
    
    def test_query(self):
        """Test direct SQL query execution"""
        # Test basic query
        result = self.tools.query("SELECT COUNT(*) as count FROM symbols")
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["count"], 22)
        
        # Test query with parameters
        result = self.tools.query(
            "SELECT name FROM symbols WHERE kind = ?",
            ("function",)
        )
        self.assertGreater(len(result), 0)
        self.assertIn("main", [r["name"] for r in result])
        
        # Test join query
        result = self.tools.query("""
            SELECT s.name, f.path
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE f.path LIKE ?
        """, ("app/%.py",))
        self.assertGreater(len(result), 0)
    
    def test_get_symbol(self):
        """Test getting a symbol by FQN"""
        # Test existing symbol
        symbol = self.tools.get_symbol("app.main.process_request")
        self.assertIsNotNone(symbol)
        self.assertEqual(symbol["name"], "process_request")
        self.assertEqual(symbol["kind"], "function")
        self.assertEqual(symbol["file_path"], "app/main.py")
        
        # Test non-existent symbol
        symbol = self.tools.get_symbol("does.not.exist")
        self.assertIsNone(symbol)
    
    def test_find_symbols(self):
        """Test finding symbols by pattern"""
        # Test pattern matching
        symbols = self.tools.find_symbols("user")
        self.assertGreater(len(symbols), 0)
        names = [s["name"] for s in symbols]
        self.assertIn("get_user", names)
        self.assertIn("create_user", names)
        
        # Test with kind filter
        functions = self.tools.find_symbols("user", kind="function")
        for func in functions:
            self.assertEqual(func["kind"], "function")
        
        # Test with empty pattern
        all_symbols = self.tools.find_symbols("")
        self.assertEqual(len(all_symbols), 22)
    
    def test_get_relationships(self):
        """Test getting symbol relationships"""
        # Test outgoing relationships
        rels = self.tools.get_relationships("app.main.process_request", direction="from")
        self.assertIn("outgoing_calls", rels)
        self.assertIn("app.main.validate_input", rels["outgoing_calls"])
        self.assertIn("app.auth.authenticate", rels["outgoing_calls"])
        
        # Test incoming relationships
        rels = self.tools.get_relationships("app.database.execute_query", direction="to")
        self.assertIn("incoming_calls", rels)
        self.assertIn("app.database.get_user", rels["incoming_calls"])
        
        # Test both directions
        rels = self.tools.get_relationships("app.auth.authenticate", direction="both")
        self.assertIn("outgoing_calls", rels)
        self.assertIn("incoming_calls", rels)
    
    def test_trace_paths(self):
        """Test tracing paths between symbols"""
        # Test direct path
        paths = self.tools.trace_paths(
            "app.main.main",
            "app.database.execute_query",
            max_depth=5
        )
        self.assertGreater(len(paths), 0)
        
        # Test with edge type filter
        paths = self.tools.trace_paths(
            "app.main.main",
            "app.database.execute_query",
            max_depth=5,
            edge_type="calls"
        )
        self.assertGreater(len(paths), 0)
        
        # Test all reachable from start
        paths = self.tools.trace_paths(
            "app.main.main",
            end=None,
            max_depth=2
        )
        self.assertGreater(len(paths), 0)
        
        # Test no path exists
        paths = self.tools.trace_paths(
            "app.utils.logger",
            "app.main.main",
            max_depth=10
        )
        self.assertEqual(len(paths), 0)
    
    def test_get_neighborhood(self):
        """Test getting symbol neighborhood"""
        # Test radius 1
        neighborhood = self.tools.get_neighborhood("app.main.process_request", radius=1)
        self.assertIn("app.main.process_request", neighborhood)
        self.assertEqual(neighborhood["app.main.process_request"], 0)
        self.assertIn("app.main.validate_input", neighborhood)
        self.assertEqual(neighborhood["app.main.validate_input"], 1)
        
        # Test radius 2
        neighborhood = self.tools.get_neighborhood("app.auth.authenticate", radius=2)
        self.assertGreater(len(neighborhood), 3)
        
        # Test isolated symbol
        neighborhood = self.tools.get_neighborhood("app.utils.logger", radius=1)
        # Should include symbols that use logger
        self.assertGreater(len(neighborhood), 1)
    
    def test_find_patterns(self):
        """Test finding patterns with custom SQL"""
        # Test finding functions that call both auth and database
        pattern = """
            SELECT DISTINCT s.fqn, s.name
            FROM symbols s
            WHERE s.fqn IN (
                SELECT e1.src
                FROM edges e1
                JOIN edges e2 ON e1.src = e2.src
                WHERE e1.dst LIKE 'app.auth%'
                  AND e2.dst LIKE 'app.database%'
                  AND e1.edge_type = 'calls'
                  AND e2.edge_type = 'calls'
            )
        """
        results = self.tools.find_patterns(pattern)
        self.assertGreater(len(results), 0)
        
        # Test finding classes with methods
        pattern = """
            SELECT c.fqn as class_fqn, COUNT(m.fqn) as method_count
            FROM symbols c
            LEFT JOIN symbols m ON m.fqn LIKE c.fqn || '.%'
            WHERE c.kind = 'class' AND m.kind = 'method'
            GROUP BY c.fqn
        """
        results = self.tools.find_patterns(pattern)
        self.assertGreater(len(results), 0)
    
    def test_get_context(self):
        """Test getting symbol context"""
        context = self.tools.get_context("app.main.process_request", context_size=2)
        
        # Check basic symbol info
        self.assertIn("symbol", context)
        self.assertEqual(context["symbol"]["name"], "process_request")
        
        # Check file symbols
        self.assertIn("file_symbols", context)
        file_syms = [s["name"] for s in context["file_symbols"]]
        self.assertIn("main", file_syms)
        self.assertIn("validate_input", file_syms)
        
        # Check relationships
        self.assertIn("relationships", context)
        self.assertIn("outgoing_calls", context["relationships"])
        
        # Check callers and callees
        self.assertIn("callers", context)
        self.assertIn("callees", context)
        
        # Check neighborhood
        self.assertIn("neighborhood", context)
        self.assertGreater(len(context["neighborhood"]), 1)
        
        # Test non-existent symbol
        context = self.tools.get_context("does.not.exist")
        self.assertEqual(context, {})
    
    def test_compare_symbols(self):
        """Test comparing two symbols"""
        comparison = self.tools.compare_symbols(
            "app.main.process_request",
            "app.auth.authenticate"
        )
        
        # Check both symbols present
        self.assertIn("symbol1", comparison)
        self.assertIn("symbol2", comparison)
        self.assertEqual(comparison["symbol1"]["name"], "process_request")
        self.assertEqual(comparison["symbol2"]["name"], "authenticate")
        
        # Check relationships
        self.assertIn("relationships1", comparison)
        self.assertIn("relationships2", comparison)
        
        # Check shared analysis
        self.assertIn("shared_callees", comparison)
        self.assertIn("shared_callers", comparison)
        
        # Both call app.database.get_user
        self.assertIn("app.database.get_user", comparison["shared_callees"])
        
        # Test with non-existent symbol
        comparison = self.tools.compare_symbols(
            "app.main.process_request",
            "does.not.exist"
        )
        self.assertEqual(comparison["error"], "One or both symbols not found")
    
    def test_get_file_summary(self):
        """Test getting file summary"""
        summary = self.tools.get_file_summary("app/auth.py")
        
        # Check basic info
        self.assertEqual(summary["file"], "app/auth.py")
        self.assertIn("symbols", summary)
        self.assertIn("edges", summary)
        
        # Check counts
        self.assertEqual(summary["symbol_count"], 4)
        self.assertGreater(summary["edge_count"], 0)
        
        # Check aggregations
        self.assertIn("symbol_kinds", summary)
        self.assertIn("function", summary["symbol_kinds"])
        self.assertIn("class", summary["symbol_kinds"])
        self.assertIn("method", summary["symbol_kinds"])
        
        self.assertIn("edge_types", summary)
        
        # Test non-existent file
        summary = self.tools.get_file_summary("does/not/exist.py")
        self.assertEqual(summary["symbol_count"], 0)
        self.assertEqual(summary["edge_count"], 0)
    
    def test_explore(self):
        """Test exploring the codebase"""
        # Test finding entry points
        entry_points = self.tools.explore(start_point="")
        self.assertIsInstance(entry_points, list)
        self.assertGreater(len(entry_points), 0)
        
        # Should find main-like functions
        self.assertIn("app.main.main", entry_points)
        
        # Test breadth exploration
        symbols = self.tools.explore(
            start_point="app.main.main",
            strategy="breadth"
        )
        self.assertIsInstance(symbols, list)
        self.assertIn("app.main.main", symbols)
        self.assertGreater(len(symbols), 1)
        
        # Test depth exploration
        symbols = self.tools.explore(
            start_point="app.main.main",
            strategy="depth"
        )
        self.assertIsInstance(symbols, list)
        self.assertIn("app.main.main", symbols)
        
        # Test random exploration
        symbols = self.tools.explore(
            start_point="app.main.main",
            strategy="random"
        )
        self.assertIsInstance(symbols, list)
        self.assertGreater(len(symbols), 0)
    
    def test_count_by_key(self):
        """Test the internal count helper"""
        items = [
            {"kind": "function"},
            {"kind": "function"},
            {"kind": "class"},
            {"kind": "method"},
            {"kind": "function"},
        ]
        counts = self.tools._count_by_key(items, "kind")
        
        self.assertEqual(counts["function"], 3)
        self.assertEqual(counts["class"], 1)
        self.assertEqual(counts["method"], 1)


class TestEdgeCases(unittest.TestCase):
    """Test edge cases and error handling"""
    
    def test_missing_database(self):
        """Test handling of missing database"""
        with self.assertRaises(FileNotFoundError):
            tools = CodeTools("/tmp/nonexistent", db_path="/tmp/missing.db")
    
    def test_empty_database(self):
        """Test handling of empty database"""
        temp_dir = tempfile.mkdtemp()
        db_dir = Path(temp_dir) / ".reviewbot"
        db_dir.mkdir()
        db_path = db_dir / "graph.db"
        
        # Create empty database with schema
        conn = sqlite3.connect(str(db_path))
        cursor = conn.cursor()
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT, hash TEXT, language TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, file_id INTEGER, fqn TEXT, name TEXT, kind TEXT, line INTEGER, signature TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        conn.commit()
        conn.close()
        
        tools = CodeTools(temp_dir)
        
        # Test queries on empty database
        self.assertEqual(len(tools.find_symbols("")), 0)
        self.assertIsNone(tools.get_symbol("any.symbol"))
        self.assertEqual(tools.get_relationships("any.symbol"), {})
        self.assertEqual(tools.get_neighborhood("any.symbol"), {"any.symbol": 0})
        
        tools.close()
        import shutil
        shutil.rmtree(temp_dir)
    
    def test_sql_injection_safety(self):
        """Test that SQL injection is prevented"""
        temp_dir = tempfile.mkdtemp()
        repo_path = Path(temp_dir)
        db_dir = repo_path / ".reviewbot"
        db_dir.mkdir()
        db_path = db_dir / "graph.db"
        
        TestFixtures.create_test_database(str(db_path))
        tools = CodeTools(str(repo_path))
        
        # Try SQL injection in find_symbols
        malicious = "'; DROP TABLE symbols; --"
        result = tools.find_symbols(malicious)
        # Should not error, parameterized queries prevent injection
        self.assertIsInstance(result, list)
        
        # Verify table still exists
        check = tools.query("SELECT COUNT(*) as count FROM symbols")
        self.assertEqual(check[0]["count"], 22)
        
        tools.close()
        import shutil
        shutil.rmtree(temp_dir)


class TestAgentUsagePatterns(unittest.TestCase):
    """Test realistic agent usage patterns"""
    
    def setUp(self):
        """Create test database"""
        self.temp_dir = tempfile.mkdtemp()
        self.repo_path = Path(self.temp_dir)
        self.db_dir = self.repo_path / ".reviewbot"
        self.db_dir.mkdir()
        self.db_path = self.db_dir / "graph.db"
        
        TestFixtures.create_test_database(str(self.db_path))
        self.tools = CodeTools(str(self.repo_path))
    
    def tearDown(self):
        """Clean up"""
        self.tools.close()
        import shutil
        shutil.rmtree(self.temp_dir)
    
    def test_agent_finding_security_issues(self):
        """Test agent looking for security vulnerabilities"""
        # Agent might look for functions that handle user input and access database
        pattern = """
            SELECT DISTINCT s1.fqn, s1.name
            FROM symbols s1
            WHERE (s1.name LIKE '%request%' OR s1.name LIKE '%input%')
              AND s1.fqn IN (
                  SELECT DISTINCT e.src FROM edges e
                  JOIN symbols s2 ON e.dst = s2.fqn
                  WHERE (s2.name LIKE '%query%' OR s2.name LIKE '%database%' OR s2.name LIKE '%get_user%')
                    AND e.edge_type = 'calls'
              )
        """
        results = self.tools.find_patterns(pattern)
        
        # Should find process_request which validates input and calls database
        self.assertGreater(len(results), 0)
        fqns = [r["fqn"] for r in results]
        self.assertIn("app.main.process_request", fqns)
    
    def test_agent_analyzing_architecture(self):
        """Test agent analyzing system architecture"""
        # Find central nodes (many connections)
        pattern = """
            SELECT s.fqn, s.name,
                   COUNT(DISTINCT e1.dst) + COUNT(DISTINCT e2.src) as connection_count
            FROM symbols s
            LEFT JOIN edges e1 ON e1.src = s.fqn
            LEFT JOIN edges e2 ON e2.dst = s.fqn
            GROUP BY s.fqn, s.name
            HAVING connection_count > 2
            ORDER BY connection_count DESC
        """
        results = self.tools.find_patterns(pattern)
        
        # Should identify key architectural components
        self.assertGreater(len(results), 0)
        
        # Agent might then explore specific components
        for result in results[:3]:  # Top 3 central nodes
            context = self.tools.get_context(result["fqn"])
            self.assertIn("relationships", context)
    
    def test_agent_finding_test_coverage(self):
        """Test agent analyzing test coverage"""
        # Find functions that have tests
        pattern = """
            SELECT DISTINCT s.fqn, s.name,
                   CASE WHEN EXISTS (
                       SELECT 1 FROM edges e
                       WHERE e.dst = s.fqn
                         AND e.src LIKE 'tests.%'
                         AND e.edge_type = 'calls'
                   ) THEN 'tested' 
                   ELSE 'untested' 
                   END as status
            FROM symbols s
            WHERE s.kind = 'function'
              AND s.fqn NOT LIKE 'tests.%'
        """
        results = self.tools.find_patterns(pattern)
        
        # Should categorize functions by test status
        self.assertGreater(len(results), 0)
        
        # Check we found both tested and untested functions
        statuses = set(r["status"] for r in results)
        # In our test data, tests import but don't directly call functions
        # So we expect all to be untested with current data
        self.assertIn("untested", statuses)
    
    def test_agent_finding_circular_dependencies(self):
        """Test agent looking for circular dependencies"""
        # This would be a more complex pattern in real usage
        # For now, check if agent can explore bidirectional relationships
        
        # Find symbols that import each other
        pattern = """
            SELECT e1.src, e1.dst
            FROM edges e1
            JOIN edges e2 ON e1.src = e2.dst AND e1.dst = e2.src
            WHERE e1.edge_type = 'imports'
              AND e2.edge_type = 'imports'
              AND e1.src < e1.dst
        """
        results = self.tools.find_patterns(pattern)
        
        # In our test data, we don't have circular imports, so should be empty
        self.assertEqual(len(results), 0)
        
        # But agent could explore import chains
        import_chains = self.tools.trace_paths(
            "app.main",
            max_depth=3,
            edge_type="imports"
        )
        self.assertIsInstance(import_chains, list)


if __name__ == "__main__":
    unittest.main()