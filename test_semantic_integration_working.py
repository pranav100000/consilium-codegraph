#!/usr/bin/env python3
"""
Working semantic integration tests that properly handle imports
"""

import pytest
import tempfile
import shutil
from pathlib import Path
import sqlite3
from unittest.mock import patch, MagicMock
import threading
import time

# Add agent_api to path
import sys
sys.path.insert(0, str(Path(__file__).parent / "agent_api"))

# Import the classes
from agent_api.code_graph import CodeGraph
from agent_api.simple_api import CodeGraphAPI
from agent_api.helpers import AgentHelpers


def test_semantic_flag_integration():
    """Test that semantic flag is properly integrated"""
    print("🧪 Testing Semantic Flag Integration")
    
    with tempfile.TemporaryDirectory() as temp_dir:
        project_path = Path(temp_dir)
        
        # Create minimal database so CodeGraph doesn't try to scan
        db_path = project_path / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(parents=True, exist_ok=True)
        conn = sqlite3.connect(db_path)
        conn.execute("CREATE TABLE test (id INTEGER)")
        conn.close()
        
        # Test CodeGraph semantic parameter
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Test semantic=True
            graph = CodeGraph(temp_dir, semantic=True)
            assert graph.semantic == True, "Should store semantic=True"
            
            # Test semantic=False  
            graph_no_semantic = CodeGraph(temp_dir, semantic=False)
            assert graph_no_semantic.semantic == False, "Should store semantic=False"
            
            # If scan was called (when database was missing), check flags
            if mock_run.called:
                calls = mock_run.call_args_list
                call_strs = [str(call) for call in calls]
                
                # Check if semantic calls were made
                semantic_calls = [call for call in call_strs if '--semantic' in call]
                if semantic_calls:
                    print("✅ Found calls with --semantic flag")
                
                non_semantic_calls = [call for call in call_strs if 'scan' in call and '--semantic' not in call]
                if non_semantic_calls:
                    print("✅ Found calls without --semantic flag")
    
    print("✅ Semantic flag integration working")


def test_error_message_integration():
    """Test that error messages include correct semantic flags"""
    print("🧪 Testing Error Message Integration")
    
    with tempfile.TemporaryDirectory() as temp_dir:
        # Test semantic=True error message
        try:
            CodeGraphAPI(temp_dir, semantic=True)
            assert False, "Should have failed"
        except FileNotFoundError as e:
            assert "--semantic" in str(e), f"Error should mention --semantic: {e}"
        
        # Test semantic=False error message  
        try:
            CodeGraphAPI(temp_dir, semantic=False)
            assert False, "Should have failed"
        except FileNotFoundError as e:
            assert "--semantic" not in str(e), f"Error should not mention --semantic: {e}"
            assert "reviewbot scan" in str(e), f"Error should mention scan command: {e}"
    
    print("✅ Error message integration working")


def test_database_functionality():
    """Test that database functionality works with semantic data"""
    print("🧪 Testing Database Functionality")
    
    with tempfile.TemporaryDirectory() as temp_dir:
        project_path = Path(temp_dir)
        
        # Create database with semantic data
        db_path = project_path / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(parents=True, exist_ok=True)
        
        conn = sqlite3.connect(db_path)
        conn.executescript("""
            CREATE TABLE files (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL,
                sha TEXT NOT NULL
            );
            
            CREATE TABLE symbols (
                id TEXT PRIMARY KEY,
                lang TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                fqn TEXT NOT NULL,
                signature TEXT,
                file_id INTEGER,
                line INTEGER,
                col INTEGER,
                FOREIGN KEY(file_id) REFERENCES files(id)
            );
            
            CREATE TABLE edges (
                edge_type TEXT NOT NULL,
                src TEXT,
                dst TEXT,
                file_src INTEGER,
                file_dst INTEGER,
                resolution TEXT NOT NULL
            );
            
            -- Insert test data
            INSERT INTO files VALUES (1, 'main.ts', 'abc123');
            INSERT INTO symbols VALUES 
                ('main.Application', 'TypeScript', 'class', 'Application', 'main.Application', NULL, 1, 5, 0);
            INSERT INTO edges VALUES 
                ('imports', 'main.Application', 'utils.Logger', 1, 2, 'semantic');
        """)
        conn.commit()
        conn.close()
        
        # Test API functionality
        api = CodeGraphAPI(str(project_path))
        
        # Test symbol retrieval
        symbol = api.get_symbol("main.Application")
        assert symbol is not None, "Should find the test symbol"
        assert symbol.name == "Application", f"Symbol name should be Application, got {symbol.name}"
        assert symbol.kind == "class", f"Symbol kind should be class, got {symbol.kind}"
    
    print("✅ Database functionality working")


def test_concurrent_access():
    """Test concurrent database access works properly"""
    print("🧪 Testing Concurrent Access")
    
    with tempfile.TemporaryDirectory() as temp_dir:
        project_path = Path(temp_dir)
        
        # Create database
        db_path = project_path / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(parents=True, exist_ok=True)
        
        conn = sqlite3.connect(db_path)
        conn.executescript("""
            CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT, sha TEXT);
            CREATE TABLE symbols (id TEXT PRIMARY KEY, lang TEXT, kind TEXT, name TEXT, fqn TEXT, signature TEXT, file_id INTEGER, line INTEGER, col INTEGER);
            INSERT INTO files VALUES (1, 'test.ts', 'abc123');
            INSERT INTO symbols VALUES ('test.symbol', 'TypeScript', 'class', 'TestClass', 'test.TestClass', NULL, 1, 1, 0);
        """)
        conn.commit()
        conn.close()
        
        results = []
        errors = []
        
        def worker():
            try:
                api = CodeGraphAPI(str(project_path), check_same_thread=False, timeout=30.0)
                symbol = api.get_symbol("test.TestClass")
                results.append(symbol is not None)
            except Exception as e:
                errors.append(e)
        
        # Start multiple workers
        threads = []
        for i in range(3):
            thread = threading.Thread(target=worker)
            threads.append(thread)
            thread.start()
        
        # Wait for completion
        for thread in threads:
            thread.join(timeout=5)
        
        assert len(errors) == 0, f"Concurrent access errors: {errors}"
        assert len(results) == 3, f"Expected 3 results, got {len(results)}"
        assert all(results), "All workers should find the symbol"
    
    print("✅ Concurrent access working")


def test_helpers_integration():
    """Test that AgentHelpers works with semantic parameter"""
    print("🧪 Testing Helpers Integration")
    
    with tempfile.TemporaryDirectory() as temp_dir:
        project_path = Path(temp_dir)
        
        # Create minimal database so CodeGraph doesn't try to scan
        db_path = project_path / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(parents=True, exist_ok=True)
        conn = sqlite3.connect(db_path)
        conn.execute("CREATE TABLE test (id INTEGER)")
        conn.close()
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Test semantic=True (default)
            helpers = AgentHelpers(temp_dir, semantic=True)
            assert helpers.graph.semantic == True
            
            # Test semantic=False
            helpers_no_semantic = AgentHelpers(temp_dir, semantic=False)
            assert helpers_no_semantic.graph.semantic == False
    
    print("✅ Helpers integration working")


def run_all_tests():
    """Run all semantic integration tests"""
    print("🚀 Running Semantic Integration Test Suite")
    print("=" * 60)
    
    tests = [
        test_semantic_flag_integration,
        test_error_message_integration,
        test_database_functionality,
        test_concurrent_access,
        test_helpers_integration
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        try:
            test()
            passed += 1
        except Exception as e:
            print(f"❌ {test.__name__} failed: {e}")
            failed += 1
    
    print("\n" + "=" * 60)
    print(f"📊 Test Results: {passed} passed, {failed} failed")
    
    if failed == 0:
        print("🎉 All semantic integration tests passed!")
        return True
    else:
        print(f"⚠️  {failed} tests failed")
        return False


if __name__ == "__main__":
    success = run_all_tests()
    sys.exit(0 if success else 1)