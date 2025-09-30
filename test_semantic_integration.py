#!/usr/bin/env python3
"""
Test script for Python API semantic integration
"""

import sys
import os
from pathlib import Path

# Add agent_api to Python path  
sys.path.insert(0, str(Path(__file__).parent))

from agent_api.code_graph import CodeGraph
from agent_api.simple_api import CodeGraphAPI
from agent_api.helpers import AgentHelpers

def test_semantic_integration():
    """Test that Python API now supports semantic analysis"""
    
    repo_path = "./test_ts_project"
    
    print("🧪 Testing Python API Semantic Integration")
    print("=" * 50)
    
    # Test 1: CodeGraph with semantic=True (default)
    print("\n1. Testing CodeGraph with semantic=True")
    try:
        graph = CodeGraph(repo_path, semantic=True)
        print("✅ CodeGraph initialized with semantic analysis")
    except Exception as e:
        print(f"❌ CodeGraph failed: {e}")
    
    # Test 2: CodeGraph with semantic=False  
    print("\n2. Testing CodeGraph with semantic=False")
    try:
        graph_no_semantic = CodeGraph(repo_path, semantic=False)
        print("✅ CodeGraph initialized without semantic analysis")
    except Exception as e:
        print(f"❌ CodeGraph failed: {e}")
    
    # Test 3: CodeGraphAPI with semantic parameter
    print("\n3. Testing CodeGraphAPI error message includes semantic flag")
    try:
        # This should fail with helpful error message
        api = CodeGraphAPI("./nonexistent_repo", semantic=True)
        print("❌ Should have failed with FileNotFoundError")
    except FileNotFoundError as e:
        if "--semantic" in str(e):
            print("✅ Error message includes semantic flag recommendation")
        else:
            print(f"❌ Error message doesn't mention semantic: {e}")
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
    
    # Test 4: AgentHelpers with semantic parameter
    print("\n4. Testing AgentHelpers with semantic parameter")
    try:
        helpers = AgentHelpers(repo_path, semantic=True)
        print("✅ AgentHelpers initialized with semantic analysis")
    except Exception as e:
        print(f"❌ AgentHelpers failed: {e}")
    
    print("\n🎉 Python API semantic integration test completed!")

if __name__ == "__main__":
    test_semantic_integration()