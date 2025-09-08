"""
Critical missing tests for agent APIs
"""

import unittest
import sqlite3
import tempfile
import os
import threading
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

import sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from simple_api import CodeGraphAPI
from agent_tools import CodeTools


class TestTransactionHandling(unittest.TestCase):
    """Test transaction handling and rollback"""
    
    def test_transaction_rollback_on_error(self):
        """Test that failed operations don't corrupt database"""
        temp_dir = tempfile.mkdtemp()
        db_path = Path(temp_dir) / ".reviewbot" / "graph.db"
        db_path.parent.mkdir()
        
        # Create initial database
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("PRAGMA foreign_keys=ON")  # Enable foreign key constraints
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT, name TEXT, kind TEXT, line INTEGER, file_id INTEGER, signature TEXT, FOREIGN KEY (file_id) REFERENCES files(id))")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        cursor.execute("INSERT INTO files (id, path) VALUES (1, 'test.py')")
        cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES ('test', 'test', 'function', 1, 1, 'def test()')")
        conn.commit()
        
        # Get initial count
        cursor.execute("SELECT COUNT(*) FROM symbols")
        initial_count = cursor.fetchone()[0]
        conn.close()
        
        # Try to insert invalid data in a transaction
        conn = sqlite3.connect(db_path)
        conn.execute("PRAGMA foreign_keys=ON")
        cursor = conn.cursor()
        
        try:
            cursor.execute("BEGIN TRANSACTION")
            cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES ('new', 'new', 'function', 2, 1, 'def new()')")
            # This should fail due to foreign key constraint (file_id=999 doesn't exist)
            cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES ('bad', 'bad', 'function', 3, 999, 'def bad()')")
            conn.commit()
        except sqlite3.Error:
            conn.rollback()
        
        # Verify rollback worked
        cursor.execute("SELECT COUNT(*) FROM symbols")
        final_count = cursor.fetchone()[0]
        conn.close()
        
        # Count should be unchanged (rollback should undo the 'new' symbol)
        assert final_count == initial_count, f"Expected {initial_count}, got {final_count}"
        
        import shutil
        shutil.rmtree(temp_dir)


class TestConcurrentAccess(unittest.TestCase):
    """Test concurrent access patterns"""
    
    def setUp(self):
        """Create shared test database"""
        self.temp_dir = tempfile.mkdtemp()
        self.db_path = Path(self.temp_dir) / ".reviewbot" / "graph.db"
        self.db_path.parent.mkdir()
        
        # Create database with test data
        conn = sqlite3.connect(self.db_path, check_same_thread=False)
        cursor = conn.cursor()
        
        # Enable WAL mode for better concurrency
        cursor.execute("PRAGMA journal_mode=WAL")
        
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT UNIQUE, name TEXT, kind TEXT, line INTEGER, file_id INTEGER, signature TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        
        # Add file first
        cursor.execute("INSERT INTO files (id, path) VALUES (1, 'test.py')")
        
        # Add test data
        for i in range(100):
            cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES (?, ?, ?, ?, ?, ?)",
                          (f"symbol_{i}", f"symbol_{i}", "function", i, 1, f"def symbol_{i}()"))
        
        for i in range(99):
            cursor.execute("INSERT INTO edges (src, dst, edge_type) VALUES (?, ?, ?)",
                          (f"symbol_{i}", f"symbol_{i+1}", "calls"))
        
        conn.commit()
        conn.close()
    
    def tearDown(self):
        """Clean up"""
        import shutil
        shutil.rmtree(self.temp_dir)
    
    def test_concurrent_reads(self):
        """Test multiple agents reading simultaneously"""
        results = []
        errors = []
        
        def read_symbols(thread_id):
            try:
                # Use check_same_thread=False for multi-threaded access
                api = CodeGraphAPI(self.temp_dir, check_same_thread=False)
                symbols = api.find_symbols("")
                results.append((thread_id, len(symbols)))
                api.close()
            except Exception as e:
                errors.append((thread_id, str(e)))
        
        # Launch multiple threads
        with ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(read_symbols, i) for i in range(10)]
            for future in as_completed(futures):
                future.result()
        
        # All reads should succeed
        assert len(errors) == 0, f"Errors occurred: {errors}"
        assert len(results) == 10
        # All should read same number of symbols
        assert all(r[1] == 100 for r in results), f"Inconsistent results: {results}"
    
    def test_concurrent_mixed_operations(self):
        """Test concurrent reads and writes"""
        results = []
        errors = []
        
        def worker(thread_id):
            try:
                if thread_id % 2 == 0:
                    # Reader
                    api = CodeGraphAPI(self.temp_dir)
                    symbols = api.find_symbols(f"symbol_{thread_id}")
                    results.append(("read", thread_id, len(symbols)))
                    api.close()
                else:
                    # Writer (add edges)
                    conn = sqlite3.connect(self.db_path, timeout=10.0)
                    cursor = conn.cursor()
                    cursor.execute("INSERT INTO edges (src, dst, edge_type) VALUES (?, ?, ?)",
                                 (f"symbol_{thread_id}", f"symbol_{thread_id+1}", "uses"))
                    conn.commit()
                    conn.close()
                    results.append(("write", thread_id, 1))
            except Exception as e:
                errors.append((thread_id, str(e)))
        
        # Launch mixed operations
        with ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(worker, i) for i in range(10)]
            for future in as_completed(futures):
                future.result()
        
        # Should have some successful operations
        assert len(results) > 0
        # Errors are acceptable due to locking, but shouldn't crash
        assert len(errors) < 10


class TestLargeDatasets(unittest.TestCase):
    """Test with large datasets"""
    
    def test_large_codebase_performance(self):
        """Test with 1000+ symbols"""
        temp_dir = tempfile.mkdtemp()
        db_path = Path(temp_dir) / ".reviewbot" / "graph.db"
        db_path.parent.mkdir()
        
        # Create large dataset
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT, name TEXT, kind TEXT, line INTEGER, file_id INTEGER, signature TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        
        # Add 1000 symbols across 50 files
        for file_id in range(1, 51):
            cursor.execute("INSERT INTO files (id, path) VALUES (?, ?)",
                          (file_id, f"src/file_{file_id}.py"))
            
            for sym_id in range(20):
                global_id = (file_id - 1) * 20 + sym_id
                cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES (?, ?, ?, ?, ?, ?)",
                              (f"module_{file_id}.func_{sym_id}", f"func_{sym_id}", "function", sym_id * 10, file_id, f"def func_{sym_id}()"))
        
        # Add 2000 edges (average 2 per symbol)
        for i in range(2000):
            src_file = (i % 50) + 1
            src_sym = i % 20
            dst_file = ((i + 7) % 50) + 1
            dst_sym = (i + 3) % 20
            cursor.execute("INSERT INTO edges (src, dst, edge_type) VALUES (?, ?, ?)",
                          (f"module_{src_file}.func_{src_sym}", f"module_{dst_file}.func_{dst_sym}", "calls"))
        
        conn.commit()
        conn.close()
        
        # Test performance
        api = CodeGraphAPI(temp_dir)
        
        start = time.time()
        symbols = api.find_symbols("")
        find_time = time.time() - start
        
        assert len(symbols) == 1000
        assert find_time < 1.0  # Should complete in under 1 second
        
        # Test path finding performance
        start = time.time()
        paths = api.find_paths("module_1.func_0", "module_25.func_10", max_depth=5)
        path_time = time.time() - start
        
        assert path_time < 2.0  # Should complete in under 2 seconds
        
        # Test impact radius performance
        start = time.time()
        impact = api.get_impact_radius("module_1.func_0", max_depth=3)
        impact_time = time.time() - start
        
        assert impact_time < 1.0  # Should complete in under 1 second
        
        api.close()
        
        import shutil
        shutil.rmtree(temp_dir)


class TestUnicodeAndEncoding(unittest.TestCase):
    """Test Unicode and special characters"""
    
    def test_unicode_symbols(self):
        """Test non-ASCII symbol names"""
        temp_dir = tempfile.mkdtemp()
        db_path = Path(temp_dir) / ".reviewbot" / "graph.db"
        db_path.parent.mkdir()
        
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT, name TEXT, kind TEXT, line INTEGER, file_id INTEGER, signature TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        
        # Unicode test cases
        unicode_symbols = [
            ("测试函数", "测试函数", "function"),  # Chinese
            ("тест_функция", "тест_функция", "function"),  # Russian
            ("🚀_deploy", "🚀_deploy", "function"),  # Emoji
            ("café_müller", "café_müller", "function"),  # Accented
            ("λ_function", "λ_function", "function"),  # Greek
        ]
        
        cursor.execute("INSERT INTO files (id, path) VALUES (1, 'unicode.py')")
        
        for i, (fqn, name, kind) in enumerate(unicode_symbols):
            cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES (?, ?, ?, ?, ?, ?)",
                          (fqn, name, kind, i * 10, 1, f"def {name}()"))
        
        conn.commit()
        conn.close()
        
        # Test reading Unicode
        api = CodeGraphAPI(temp_dir)
        
        # Find Chinese function
        chinese = api.get_symbol("测试函数")
        assert chinese is not None
        assert chinese.name == "测试函数"
        
        # Find emoji function
        emoji = api.get_symbol("🚀_deploy")
        assert emoji is not None
        
        # Search with Unicode
        results = api.find_symbols("测试")
        assert len(results) == 1
        
        api.close()
        
        import shutil
        shutil.rmtree(temp_dir)


class TestConnectionManagement(unittest.TestCase):
    """Test proper connection cleanup"""
    
    def test_connection_cleanup(self):
        """Ensure connections are properly closed"""
        temp_dir = tempfile.mkdtemp()
        db_path = Path(temp_dir) / ".reviewbot" / "graph.db"
        db_path.parent.mkdir()
        
        # Create database
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT, name TEXT, kind TEXT, line INTEGER, file_id INTEGER, signature TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        conn.commit()
        conn.close()
        
        # Create and destroy many connections
        for i in range(100):
            api = CodeGraphAPI(temp_dir)
            _ = api.get_stats()
            api.close()
        
        # Verify we can still connect (no lock issues)
        api = CodeGraphAPI(temp_dir)
        stats = api.get_stats()
        assert stats is not None
        api.close()
        
        import shutil
        shutil.rmtree(temp_dir)
    
    def test_context_manager_pattern(self):
        """Test using API as context manager"""
        temp_dir = tempfile.mkdtemp()
        db_path = Path(temp_dir) / ".reviewbot" / "graph.db"
        db_path.parent.mkdir()
        
        # Create database
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT, name TEXT, kind TEXT, line INTEGER, file_id INTEGER, signature TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT, edge_type TEXT)")
        cursor.execute("INSERT INTO files (id, path) VALUES (1, 'test.py')")
        cursor.execute("INSERT INTO symbols (fqn, name, kind, line, file_id, signature) VALUES ('test', 'test', 'function', 1, 1, 'def test()')")
        conn.commit()
        conn.close()
        
        # Test context manager with CodeGraphAPI
        with CodeGraphAPI(temp_dir) as api:
            stats = api.get_stats()
            assert stats is not None
            assert stats["total_symbols"] == 1
            # Connection should be open
            assert api.conn is not None
        
        # After context exit, connection should be closed
        assert api.conn is None
        
        # Test context manager with CodeTools
        with CodeTools(temp_dir) as tools:
            symbols = tools.find_symbols("")
            assert len(symbols) == 1
            assert tools.conn is not None
        
        # After context exit, connection should be closed
        assert tools.conn is None
        
        import shutil
        shutil.rmtree(temp_dir)


if __name__ == "__main__":
    unittest.main()