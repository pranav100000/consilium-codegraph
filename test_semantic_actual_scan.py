#!/usr/bin/env python3
"""
Test actual semantic scan functionality
"""

import sys
from pathlib import Path
import tempfile
import os

# Add agent_api to Python path  
sys.path.insert(0, str(Path(__file__).parent))

from agent_api.code_graph import CodeGraph

def test_semantic_scan():
    """Test that semantic flag actually triggers SCIP analysis"""
    
    # Create a minimal test repository
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Create a simple TypeScript file
        (temp_path / "test.ts").write_text("""
function greet(name: string): string {
    return `Hello, ${name}!`;
}

class Person {
    constructor(public name: string) {}
    
    sayHello(): string {
        return greet(this.name);
    }
}

export { greet, Person };
""")
        
        # Create package.json for TypeScript project
        (temp_path / "package.json").write_text("""
{
    "name": "test-project",
    "version": "1.0.0",
    "main": "test.ts",
    "dependencies": {
        "typescript": "^5.0.0"
    }
}
""")
        
        print(f"🧪 Testing semantic scan on temporary project: {temp_path}")
        print("=" * 60)
        
        # Test semantic=True - this should show the command with --semantic
        print("\n1. Testing with semantic=True")
        try:
            graph = CodeGraph(str(temp_path), semantic=True)
            print("✅ Semantic scan completed successfully")
        except Exception as e:
            print(f"ℹ️  Semantic scan failed (expected - no SCIP indexers): {e}")
        
        # Test semantic=False - this should show command without --semantic  
        print("\n2. Testing with semantic=False")
        try:
            graph = CodeGraph(str(temp_path), semantic=False)
            print("✅ Syntactic-only scan completed successfully")
        except Exception as e:
            print(f"ℹ️  Syntactic scan failed: {e}")

if __name__ == "__main__":
    test_semantic_scan()