#!/usr/bin/env python3
"""
Simple bug hunting test to isolate issues
"""

import subprocess
import tempfile
from pathlib import Path
import time

def test_empty_files():
    """Test handling of empty files"""
    print("Testing empty file handling...")
    
    with tempfile.TemporaryDirectory() as temp_dir:
        project_path = Path(temp_dir) / "empty_test"
        project_path.mkdir()
        
        # Create empty TypeScript file
        (project_path / "empty.ts").write_text("")
        
        # Test scanning
        cmd = ["cargo", "run", "--", "--repo", str(project_path), "scan"]
        print(f"Running: {' '.join(cmd)}")
        
        try:
            result = subprocess.run(
                cmd, 
                capture_output=True,
                text=True,
                timeout=30
            )
            
            if result.returncode == 0:
                print("✅ Empty files handled correctly")
                return True
            else:
                print(f"❌ Empty files caused scan failure: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            print("❌ Test timed out")
            return False

if __name__ == "__main__":
    test_empty_files()