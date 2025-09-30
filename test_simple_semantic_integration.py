#!/usr/bin/env python3
"""
Simple test to verify semantic integration works
"""

import tempfile
import sqlite3
import shutil
from pathlib import Path
from unittest.mock import patch, MagicMock

# Fix imports by adding the right path
import sys
sys.path.insert(0, str(Path(__file__).parent))

# Import with absolute paths to avoid relative import issues
import importlib.util

def import_module_from_path(name, path):
    """Import a module from a specific file path"""
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

# Import the modules we need
agent_api_path = Path(__file__).parent / "agent_api"
code_graph_mod = import_module_from_path("code_graph", agent_api_path / "code_graph.py")
simple_api_mod = import_module_from_path("simple_api", agent_api_path / "simple_api.py")

def test_basic_semantic_integration():
    """Test basic semantic integration functionality"""
    
    print("🧪 Testing Basic Semantic Integration")
    print("=" * 50)
    
    # Create temporary project
    temp_dir = tempfile.mkdtemp(prefix="semantic_test_")
    project_path = Path(temp_dir)
    
    try:
        # Create a simple TypeScript file
        (project_path / "test.ts").write_text("""
export class TestClass {
    method(): string {
        return "test";
    }
}
""")
        
        print(f"✅ Created test project at: {project_path}")
        
        # Test 1: Mock scan to avoid actual SCIP execution
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Test semantic=True
            print("\n1. Testing CodeGraph with semantic=True")
            try:
                graph = code_graph_mod.CodeGraph(str(project_path), semantic=True)
                print("✅ CodeGraph initialized with semantic analysis")
                
                # Check that subprocess was called with --semantic
                calls = mock_run.call_args_list
                semantic_calls = [call for call in calls if '--semantic' in str(call)]
                if semantic_calls:
                    print("✅ Scan called with --semantic flag")
                else:
                    print("❌ Scan not called with --semantic flag")
                    
            except Exception as e:
                print(f"❌ CodeGraph semantic initialization failed: {e}")
            
            # Test semantic=False
            print("\n2. Testing CodeGraph with semantic=False")
            try:
                graph_no_semantic = code_graph_mod.CodeGraph(str(project_path), semantic=False)
                print("✅ CodeGraph initialized without semantic analysis")
                
                # Check that subprocess was called without --semantic
                calls = mock_run.call_args_list
                non_semantic_calls = [call for call in calls if '--semantic' not in str(call)]
                if non_semantic_calls:
                    print("✅ Scan called without --semantic flag")
                else:
                    print("❌ Scan not called without --semantic flag")
                    
            except Exception as e:
                print(f"❌ CodeGraph non-semantic initialization failed: {e}")
        
        # Test 2: Database creation and API functionality
        print("\n3. Testing database creation and API")
        
        # Create a proper database manually
        db_path = project_path / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(parents=True, exist_ok=True)
        
        conn = sqlite3.connect(db_path)
        
        # Create the expected schema
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
            
            -- Insert test data
            INSERT INTO files VALUES (1, 'test.ts', 'abc123');
            INSERT INTO symbols VALUES 
                ('test.TestClass', 'TypeScript', 'class', 'TestClass', 'test.TestClass', NULL, 1, 1, 0);
        """)
        conn.commit()
        conn.close()
        
        print(f"✅ Created database at: {db_path}")
        
        # Test API functionality
        try:
            api = simple_api_mod.CodeGraphAPI(str(project_path))
            symbol = api.get_symbol("test.TestClass")
            
            if symbol:
                print("✅ API successfully retrieved symbol")
                print(f"   Symbol: {symbol.name} ({symbol.kind})")
            else:
                print("❌ API could not retrieve symbol")
                
        except Exception as e:
            print(f"❌ API test failed: {e}")
        
        print("\n🎉 Basic semantic integration test completed!")
        
    finally:
        # Cleanup
        shutil.rmtree(temp_dir, ignore_errors=True)

if __name__ == "__main__":
    test_basic_semantic_integration()