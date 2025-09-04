"""
Tests for CodeGraph class
"""

import pytest
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent_api.code_graph import CodeGraph
from agent_api.models import Symbol, SymbolKind, Location, CallPath, DependencyGraph


class TestCodeGraph:
    """Test CodeGraph class"""
    
    def test_init_with_existing_db(self, mock_repo, mock_db):
        """Test initialization with existing database"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        assert graph.repo_path == mock_repo
        assert graph.db_path == mock_db
        assert graph.conn is not None
    
    def test_get_symbol(self, mock_repo, mock_db):
        """Test getting a symbol by FQN"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Get existing symbol
        symbol = graph.get_symbol("AuthService::authenticate")
        assert symbol is not None
        assert symbol.fqn == "AuthService::authenticate"
        assert symbol.name == "authenticate"
        assert symbol.kind == SymbolKind.METHOD
        assert symbol.location.file == "src/auth.py"
        assert symbol.location.line == 10
        
        # Try non-existent symbol
        symbol = graph.get_symbol("NonExistent::function")
        assert symbol is None
    
    def test_get_symbol_caching(self, mock_repo, mock_db):
        """Test that symbols are cached"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # First call
        symbol1 = graph.get_symbol("Database::query")
        
        # Second call should use cache
        symbol2 = graph.get_symbol("Database::query")
        
        # Should be the same object
        assert symbol1 is symbol2
        assert "Database::query" in graph._symbol_cache
    
    def test_find_symbols(self, mock_repo, mock_db):
        """Test finding symbols by pattern"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Find all auth-related symbols
        symbols = graph.find_symbols("auth")
        assert len(symbols) == 2  # authenticate and AuthService
        assert any(s.name == "authenticate" for s in symbols)
        assert any(s.name == "AuthService" for s in symbols)
        
        # Find with kind filter
        functions = graph.find_symbols("", SymbolKind.FUNCTION)
        assert all(s.kind == SymbolKind.FUNCTION for s in functions)
        assert len(functions) == 4  # main, process_data, check_permission, complex_function
        
        # Find with limit
        limited = graph.find_symbols("", limit=3)
        assert len(limited) <= 3
    
    def test_get_file_symbols(self, mock_repo, mock_db):
        """Test getting all symbols in a file"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Get symbols from auth.py
        symbols = graph.get_file_symbols("src/auth.py")
        assert len(symbols) == 4  # AuthService, authenticate, validate_token, check_permission
        assert all(s.location.file == "src/auth.py" for s in symbols)
        
        # Should be sorted by line number
        lines = [s.location.line for s in symbols]
        assert lines == sorted(lines)
    
    def test_get_callers(self, mock_repo, mock_db):
        """Test finding functions that call a symbol"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Find callers of Database::query
        callers = graph.get_callers("Database::query", max_depth=1)
        
        # Should find process_data, authenticate, check_permission
        caller_names = [path.path[1].fqn for path in callers if len(path.path) > 1]
        assert "process_data" in caller_names
        assert "AuthService::authenticate" in caller_names
        assert "check_permission" in caller_names
    
    def test_get_callers_with_depth(self, mock_repo, mock_db):
        """Test finding callers with multiple depth levels"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Find callers of Database::query with depth 2
        callers = graph.get_callers("Database::query", max_depth=2)
        
        # Should find direct and indirect callers
        all_callers = set()
        for path in callers:
            for sym in path.path[1:]:
                all_callers.add(sym.fqn)
        
        # Direct callers
        assert "process_data" in all_callers
        # Indirect callers (main calls process_data which calls Database::query)
        assert "main" in all_callers
    
    def test_get_callees(self, mock_repo, mock_db):
        """Test finding functions called by a symbol"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Find what main calls
        callees = graph.get_callees("main", max_depth=1)
        
        callee_names = [path.path[-1].fqn for path in callees if len(path.path) > 1]
        assert "process_data" in callee_names
        assert "AuthService::authenticate" in callee_names
    
    def test_recursive_call_detection(self, mock_repo, mock_db):
        """Test detection of recursive calls"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Add a recursive edge for testing
        conn = graph.conn
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO edges (src, dst, edge_type, resolution)
            VALUES ('process_data', 'process_data', 'calls', 'syntactic')
        """)
        conn.commit()
        
        # Get callees should mark as recursive
        callees = graph.get_callees("process_data", max_depth=2)
        recursive_paths = [p for p in callees if p.is_recursive]
        assert len(recursive_paths) > 0
    
    def test_get_dependencies(self, mock_repo, mock_db):
        """Test getting symbol dependencies"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        deps = graph.get_dependencies("AuthService::authenticate")
        
        assert isinstance(deps, DependencyGraph)
        assert deps.root.fqn == "AuthService::authenticate"
        
        # Should have Database::query and validate_token as dependencies
        auth_deps = deps.dependencies.get("AuthService::authenticate", [])
        assert "Database::query" in auth_deps
        assert "AuthService::validate_token" in auth_deps
        
        # Should have main as dependent
        auth_dependents = deps.dependents.get("AuthService::authenticate", [])
        assert "main" in auth_dependents
    
    def test_find_path(self, mock_repo, mock_db):
        """Test finding paths between symbols"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Find path from main to Database::query
        paths = graph.find_path("main", "Database::query", max_depth=3)
        
        assert len(paths) > 0
        
        # Check that paths are valid
        for path in paths:
            assert path[0].fqn == "main"
            assert path[-1].fqn == "Database::query"
            
        # Should find at least the path: main -> process_data -> Database::query
        path_strings = [" -> ".join([s.fqn for s in path]) for path in paths]
        assert any("process_data" in p for p in path_strings)
    
    def test_find_path_no_connection(self, mock_repo, mock_db):
        """Test finding path when no connection exists"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Try to find path to a symbol with no connection
        paths = graph.find_path("Database::execute", "main", max_depth=5)
        
        # Should return empty list
        assert paths == []
    
    def test_get_statistics(self, mock_repo, mock_db):
        """Test getting graph statistics"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        stats = graph.get_statistics()
        
        assert "symbols_by_kind" in stats
        assert "edges_by_type" in stats
        assert "total_files" in stats
        assert "total_symbols" in stats
        assert "total_edges" in stats
        
        # Check counts
        assert stats["total_files"] == 3
        assert stats["total_symbols"] == 10
        assert stats["total_edges"] == 11
        
        # Check symbol types
        assert stats["symbols_by_kind"]["function"] == 4
        assert stats["symbols_by_kind"]["class"] == 2
        assert stats["symbols_by_kind"]["method"] == 4
        
        # Check edge types
        assert stats["edges_by_type"]["calls"] == 9
        assert stats["edges_by_type"]["imports"] == 2
    
    def test_refresh_cache(self, mock_repo, mock_db):
        """Test cache refresh"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Populate cache
        graph.get_symbol("main")
        graph.get_callees("main")
        
        assert len(graph._symbol_cache) > 0
        
        # Refresh cache
        graph.refresh_cache()
        
        assert len(graph._symbol_cache) == 0
        assert len(graph._callgraph_cache) == 0
    
    def test_close_connection(self, mock_repo, mock_db):
        """Test closing database connection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Connection should be open
        assert graph.conn is not None
        
        # Close connection
        graph.close()
        
        # Should not raise error on subsequent close
        graph.close()
    
    def test_find_cycles(self, mock_repo, mock_db):
        """Test cycle detection in dependencies"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        
        # Create a cycle for testing
        dependencies = {
            "A": ["B"],
            "B": ["C"],
            "C": ["A"]  # Cycle: A -> B -> C -> A
        }
        
        cycles = graph._find_cycles("A", dependencies)
        
        assert len(cycles) > 0
        assert ["A", "B", "C"] in cycles or ["B", "C", "A"] in cycles or ["C", "A", "B"] in cycles
    
    @patch('subprocess.run')
    def test_run_initial_scan(self, mock_run, temp_dir):
        """Test running initial scan when database doesn't exist"""
        # Mock successful subprocess run
        mock_result = MagicMock()
        mock_result.returncode = 0
        mock_result.stderr = ""
        mock_run.return_value = mock_result
        
        # Create empty database after "scan"
        db_dir = temp_dir / ".reviewbot"
        db_dir.mkdir()
        db_path = db_dir / "graph.db"
        
        import sqlite3
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT)")
        cursor.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY, fqn TEXT)")
        cursor.execute("CREATE TABLE edges (id INTEGER PRIMARY KEY, src TEXT, dst TEXT)")
        conn.commit()
        conn.close()
        
        # Try to create graph with non-existent DB
        graph = CodeGraph(str(temp_dir))
        
        # Should have called subprocess
        mock_run.assert_called_once()
        
        # Should have connection
        assert graph.conn is not None