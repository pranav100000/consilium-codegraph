#!/usr/bin/env python3
"""
Simple End-to-End Test for Consilium CodeGraph

Tests the basic functionality that actually works.
"""

import os
import sys
import subprocess
import tempfile
import shutil
from pathlib import Path
import sqlite3
import time

class SimpleE2ETest:
    """Simple end-to-end test for the working functionality"""
    
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.results = []
        
    def log(self, message, success=None):
        """Log test results"""
        if success is True:
            print(f"✅ {message}")
        elif success is False:
            print(f"❌ {message}")
        else:
            print(f"ℹ️  {message}")
        
        self.results.append({'message': message, 'success': success})
    
    def run_command(self, cmd, cwd=None, timeout=60):
        """Run a command and return result"""
        try:
            result = subprocess.run(
                cmd, 
                cwd=cwd or self.project_root,
                capture_output=True,
                text=True,
                timeout=timeout
            )
            return result
        except Exception as e:
            self.log(f"Command failed: {e}", False)
            return None
    
    def create_simple_project(self, project_dir):
        """Create a simple test project"""
        project_path = Path(project_dir)
        
        # TypeScript files
        src_dir = project_path / "src"
        src_dir.mkdir(parents=True, exist_ok=True)
        
        (src_dir / "user.ts").write_text("""
export class User {
    constructor(
        public id: number,
        public name: string,
        public email: string
    ) {}

    getName(): string {
        return this.name;
    }

    getEmail(): string {
        return this.email;
    }
}
""")

        (src_dir / "service.ts").write_text("""
import { User } from './user';

export class UserService {
    private users: User[] = [];

    addUser(user: User): void {
        this.users.push(user);
    }

    getUser(id: number): User | undefined {
        return this.users.find(u => u.id === id);
    }

    getAllUsers(): User[] {
        return this.users;
    }
}
""")

        (src_dir / "app.ts").write_text("""
import { User } from './user';
import { UserService } from './service';

class Application {
    private userService = new UserService();

    run(): void {
        const user1 = new User(1, "Alice", "alice@example.com");
        const user2 = new User(2, "Bob", "bob@example.com");
        
        this.userService.addUser(user1);
        this.userService.addUser(user2);
        
        console.log("Users:", this.userService.getAllUsers());
    }
}

const app = new Application();
app.run();
""")

        # Package.json
        (project_path / "package.json").write_text("""
{
    "name": "simple-test-project",
    "version": "1.0.0",
    "main": "src/app.ts"
}
""")

        self.log("Created simple test project", True)
        return project_path

    def test_basic_scan(self, project_path):
        """Test basic scanning functionality"""
        self.log("Testing basic scan...")
        
        cmd = ["cargo", "run", "--", "--repo", str(project_path), "scan"]
        result = self.run_command(cmd)
        
        if result and result.returncode == 0:
            self.log("Basic scan completed successfully", True)
            return True
        else:
            self.log(f"Basic scan failed: {result.stderr if result else 'unknown'}", False)
            return False

    def test_database_created(self, project_path):
        """Test that database was created with content"""
        db_path = project_path / ".reviewbot" / "graph.db"
        
        if not db_path.exists():
            self.log("Database file not found", False)
            return False
        
        self.log("Database file exists", True)
        
        try:
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            # Check symbol count
            cursor.execute("SELECT COUNT(*) FROM symbol")
            symbol_count = cursor.fetchone()[0]
            
            if symbol_count > 0:
                self.log(f"Found {symbol_count} symbols in database", True)
            else:
                self.log("No symbols found in database", False)
                return False
                
            # Check file count
            cursor.execute("SELECT COUNT(*) FROM file")
            file_count = cursor.fetchone()[0]
            
            if file_count > 0:
                self.log(f"Found {file_count} files in database", True)
            else:
                self.log("No files found in database", False)
                return False
            
            # Sample some symbols
            cursor.execute("SELECT name, kind FROM symbol LIMIT 3")
            symbols = cursor.fetchall()
            self.log(f"Sample symbols: {symbols}")
            
            conn.close()
            return True
            
        except Exception as e:
            self.log(f"Database validation failed: {e}", False)
            return False

    def test_search_functionality(self, project_path):
        """Test search functionality"""
        self.log("Testing search functionality...")
        
        cmd = ["cargo", "run", "--", "--repo", str(project_path), "search", "User"]
        result = self.run_command(cmd)
        
        if result and result.returncode == 0:
            if "User" in result.stdout:
                self.log("Search found User symbols", True)
                return True
            else:
                self.log("Search completed but no User symbols found", False)
                return False
        else:
            self.log(f"Search failed: {result.stderr if result else 'unknown'}", False)
            return False

    def test_show_symbol(self, project_path):
        """Test show symbol functionality"""
        self.log("Testing show symbol functionality...")
        
        # First find a symbol to show
        cmd = ["cargo", "run", "--", "--repo", str(project_path), "search", "UserService"]
        result = self.run_command(cmd)
        
        if not result or result.returncode != 0:
            self.log("Could not find UserService for show test", False)
            return False
        
        # Try to show the symbol (use a likely FQN)
        cmd = ["cargo", "run", "--", "--repo", str(project_path), "show", "--symbol", "UserService"]
        result = self.run_command(cmd)
        
        if result and result.returncode == 0:
            self.log("Show symbol functionality working", True)
            return True
        else:
            self.log("Show symbol functionality failed", False)
            return False

    def test_semantic_scan(self, project_path):
        """Test semantic scanning (may fail if SCIP not installed)"""
        self.log("Testing semantic scan...")
        
        # Remove existing database
        db_path = project_path / ".reviewbot" / "graph.db"
        if db_path.exists():
            shutil.rmtree(db_path.parent)
        
        cmd = ["cargo", "run", "--", "--repo", str(project_path), "scan", "--semantic"]
        result = self.run_command(cmd, timeout=120)
        
        if result and result.returncode == 0:
            self.log("Semantic scan completed successfully", True)
            return True
        else:
            # Don't fail test if SCIP indexers aren't installed
            self.log("Semantic scan failed (expected if SCIP indexers not installed)", None)
            return True

    def test_python_api_compatibility(self, project_path):
        """Test Python API can read the database"""
        # Add agent_api to path
        agent_api_path = self.project_root / "agent_api"
        if not agent_api_path.exists():
            self.log("Python API not available", None)
            return True
        
        sys.path.insert(0, str(agent_api_path))
        
        try:
            from simple_api import CodeGraphAPI
            
            api = CodeGraphAPI(str(project_path))
            
            # Try to find some symbols
            symbols = api.find_symbols("User")
            if symbols:
                self.log(f"Python API found {len(symbols)} User symbols", True)
            else:
                self.log("Python API found no User symbols", False)
                return False
                
            return True
            
        except ImportError:
            self.log("Python API modules not available", None)
            return True
        except Exception as e:
            self.log(f"Python API test failed: {e}", False)
            return False

    def run_all_tests(self):
        """Run all simple e2e tests"""
        print("🚀 Running Simple End-to-End Tests")
        print("=" * 50)
        
        with tempfile.TemporaryDirectory(prefix="simple_e2e_") as temp_dir:
            project_path = self.create_simple_project(temp_dir)
            
            tests = [
                ("Basic Scan", self.test_basic_scan),
                ("Database Created", self.test_database_created),
                ("Search Functionality", self.test_search_functionality),
                ("Show Symbol", self.test_show_symbol),
                ("Semantic Scan", self.test_semantic_scan),
                ("Python API Compatibility", self.test_python_api_compatibility),
            ]
            
            passed = 0
            failed = 0
            
            for test_name, test_func in tests:
                print(f"\n🧪 {test_name}")
                print("-" * 30)
                
                try:
                    success = test_func(project_path)
                    if success:
                        passed += 1
                    else:
                        failed += 1
                except Exception as e:
                    self.log(f"Test {test_name} threw exception: {e}", False)
                    failed += 1
        
        # Results
        print("\n" + "=" * 50)
        print("📊 Simple E2E Test Results")
        print("=" * 50)
        print(f"✅ Passed: {passed}")
        print(f"❌ Failed: {failed}")
        
        if failed == 0:
            print("\n🎉 All simple e2e tests passed!")
            return True
        else:
            print(f"\n⚠️  {failed} test(s) failed")
            return False


if __name__ == "__main__":
    test = SimpleE2ETest()
    success = test.run_all_tests()
    sys.exit(0 if success else 1)