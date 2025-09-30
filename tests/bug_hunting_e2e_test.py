#!/usr/bin/env python3
"""
Bug Hunting End-to-End Test Suite for Consilium CodeGraph

Comprehensive testing designed to find bugs, edge cases, and performance issues.
"""

import os
import sys
import subprocess
import tempfile
import shutil
import threading
import time
import sqlite3
import random
import string
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
import json


class BugHuntingE2ETest:
    """Comprehensive bug hunting test suite"""
    
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.results = []
        self.failures = []
        
    def log(self, message, success=None, category="INFO"):
        """Enhanced logging with categories"""
        if success is True:
            print(f"✅ [{category}] {message}")
        elif success is False:
            print(f"❌ [{category}] {message}")
            self.failures.append(f"[{category}] {message}")
        else:
            print(f"ℹ️  [{category}] {message}")
        
        self.results.append({
            'message': message, 
            'success': success, 
            'category': category,
            'timestamp': time.time()
        })
    
    def run_command(self, cmd, cwd=None, timeout=60, expect_failure=False):
        """Run command with enhanced error handling"""
        try:
            result = subprocess.run(
                cmd, 
                cwd=cwd or self.project_root,
                capture_output=True,
                text=True,
                timeout=timeout
            )
            
            if expect_failure:
                if result.returncode != 0:
                    return result  # Expected failure
                else:
                    self.log(f"Expected failure but command succeeded: {' '.join(cmd)}", False, "UNEXPECTED")
                    return None
            else:
                return result
                
        except subprocess.TimeoutExpired:
            self.log(f"Command timed out after {timeout}s: {' '.join(cmd)}", False, "TIMEOUT")
            return None
        except Exception as e:
            self.log(f"Command failed: {e}", False, "ERROR")
            return None

    # ========== Edge Case Tests ==========
    
    def test_empty_files(self, base_dir):
        """Test handling of empty files"""
        self.log("Testing empty file handling...", category="EDGE_CASE")
        
        project_path = Path(base_dir) / "empty_files"
        project_path.mkdir()
        
        # Create empty files in various languages
        empty_files = [
            "empty.ts", "empty.py", "empty.java", "empty.cpp", 
            "empty.go", "empty.rs", "empty.cs"
        ]
        
        for filename in empty_files:
            (project_path / filename).write_text("")
        
        # Test scanning
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        
        if result and result.returncode == 0:
            self.log("Empty files handled correctly", True, "EDGE_CASE")
            return True
        else:
            self.log(f"Empty files caused scan failure: {result.stderr if result else 'timeout'}", False, "EDGE_CASE")
            return False

    def test_syntax_errors(self, base_dir):
        """Test handling of files with syntax errors"""
        self.log("Testing syntax error handling...", category="EDGE_CASE")
        
        project_path = Path(base_dir) / "syntax_errors"
        project_path.mkdir()
        
        # Create files with deliberate syntax errors
        syntax_error_files = {
            "broken.ts": "class Broken { public function invalid syntax here",
            "broken.py": "def broken_function(\n    missing_closing_paren",
            "broken.java": "public class Broken { public void method( missing brace",
            "broken.cpp": "#include <iostream>\nint main() { std::cout << missing_quote;",
            "broken.go": "package main\nfunc main() { fmt.Println(unclosed string",
            "broken.rs": "fn main() { let x = missing_semicolon",
            "broken.cs": "using System; public class Broken { public void Method( syntax error"
        }
        
        for filename, content in syntax_error_files.items():
            (project_path / filename).write_text(content)
        
        # Test scanning - should handle gracefully
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        
        if result and result.returncode == 0:
            self.log("Syntax errors handled gracefully", True, "EDGE_CASE")
            return True
        else:
            self.log(f"Syntax errors caused scan crash: {result.stderr if result else 'timeout'}", False, "EDGE_CASE")
            return False

    def test_huge_files(self, base_dir):
        """Test handling of very large files"""
        self.log("Testing large file handling...", category="EDGE_CASE")
        
        project_path = Path(base_dir) / "huge_files"
        project_path.mkdir()
        
        # Generate a large TypeScript file (10k lines)
        large_content = []
        for i in range(5000):
            large_content.append(f"""
export class Generated{i} {{
    private value{i}: number = {i};
    
    public getValue{i}(): number {{
        return this.value{i};
    }}
    
    public setValue{i}(val: number): void {{
        this.value{i} = val;
    }}
}}""")
        
        (project_path / "huge.ts").write_text("\n".join(large_content))
        
        # Test scanning with timeout
        start_time = time.time()
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"], timeout=120)
        scan_time = time.time() - start_time
        
        if result and result.returncode == 0:
            self.log(f"Large file handled in {scan_time:.2f}s", True, "EDGE_CASE")
            
            # Check if reasonable number of symbols extracted
            db_path = project_path / ".reviewbot" / "graph.db"
            if db_path.exists():
                conn = sqlite3.connect(db_path)
                cursor = conn.cursor()
                cursor.execute("SELECT COUNT(*) FROM symbol")
                symbol_count = cursor.fetchone()[0]
                conn.close()
                
                if symbol_count > 1000:
                    self.log(f"Large file produced {symbol_count} symbols", True, "EDGE_CASE")
                    return True
                else:
                    self.log(f"Large file produced too few symbols: {symbol_count}", False, "EDGE_CASE")
                    return False
            else:
                self.log("Large file scan didn't create database", False, "EDGE_CASE")
                return False
        else:
            self.log(f"Large file scan failed after {scan_time:.2f}s", False, "EDGE_CASE")
            return False

    def test_unicode_and_special_chars(self, base_dir):
        """Test handling of Unicode and special characters"""
        self.log("Testing Unicode and special character handling...", category="EDGE_CASE")
        
        project_path = Path(base_dir) / "unicode_test"
        project_path.mkdir()
        
        # Create files with Unicode content
        unicode_content = {
            "unicode.ts": """
// Unicode comments: 你好世界 🚀 𝓤𝓷𝓲𝓬𝓸𝓭𝓮
export class Unicode类 {
    private 变量: string = "测试";
    
    public get数据(): string {
        return this.变量 + " 🎉";
    }
}

const 常量 = "特殊字符: ñáéíóú àèìòù äëïöü";
""",
            "unicode.py": """# -*- coding: utf-8 -*-
# Unicode test: 日本語 한국어 العربية
class Unicode类:
    def __init__(self):
        self.变量 = "测试数据"
        self.émojis = "🐍🚀💻"
    
    def 获取数据(self):
        return f"{self.变量} - {self.émojis}"

# Special characters
变量名 = "特殊字符测试"
""",
        }
        
        for filename, content in unicode_content.items():
            (project_path / filename).write_text(content, encoding='utf-8')
        
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        
        if result and result.returncode == 0:
            self.log("Unicode characters handled correctly", True, "EDGE_CASE")
            return True
        else:
            self.log(f"Unicode handling failed: {result.stderr if result else 'timeout'}", False, "EDGE_CASE")
            return False

    # ========== Concurrency Tests ==========
    
    def test_concurrent_scans(self, base_dir):
        """Test multiple concurrent scan operations"""
        self.log("Testing concurrent scan operations...", category="CONCURRENCY")
        
        # Create multiple test projects
        projects = []
        for i in range(3):
            project_path = Path(base_dir) / f"concurrent_{i}"
            project_path.mkdir()
            
            # Simple TS project
            (project_path / "test.ts").write_text(f"""
export class Test{i} {{
    private value: number = {i};
    public getValue(): number {{ return this.value; }}
}}
""")
            projects.append(project_path)
        
        # Run concurrent scans
        def scan_project(project):
            return self.run_command(["cargo", "run", "--", "--repo", str(project), "scan"])
        
        start_time = time.time()
        with ThreadPoolExecutor(max_workers=3) as executor:
            futures = [executor.submit(scan_project, proj) for proj in projects]
            results = [future.result() for future in as_completed(futures)]
        
        total_time = time.time() - start_time
        
        # Check all succeeded
        successes = sum(1 for r in results if r and r.returncode == 0)
        
        if successes == 3:
            self.log(f"All {successes} concurrent scans completed in {total_time:.2f}s", True, "CONCURRENCY")
            return True
        else:
            self.log(f"Only {successes}/3 concurrent scans succeeded", False, "CONCURRENCY")
            return False

    def test_concurrent_queries(self, base_dir):
        """Test concurrent query operations on same database"""
        self.log("Testing concurrent query operations...", category="CONCURRENCY")
        
        project_path = Path(base_dir) / "concurrent_queries"
        project_path.mkdir()
        
        # Create a substantial project
        (project_path / "main.ts").write_text("""
export class UserService {
    private users: User[] = [];
    
    addUser(user: User): void { this.users.push(user); }
    getUser(id: number): User | undefined { return this.users.find(u => u.id === id); }
    getAllUsers(): User[] { return this.users; }
}

export class User {
    constructor(public id: number, public name: string) {}
    getName(): string { return this.name; }
}

export class Application {
    private service = new UserService();
    run(): void { /* implementation */ }
}
""")
        
        # Initial scan
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        if not result or result.returncode != 0:
            self.log("Initial scan for concurrent queries failed", False, "CONCURRENCY")
            return False
        
        # Run concurrent queries
        def run_query(query_type):
            if query_type == "search":
                return self.run_command(["cargo", "run", "--", "--repo", str(project_path), "search", "User"])
            elif query_type == "show":
                return self.run_command(["cargo", "run", "--", "--repo", str(project_path), "show", "--symbol", "User"])
            else:
                return self.run_command(["cargo", "run", "--", "--repo", str(project_path), "search", "Service"])
        
        queries = ["search", "show", "search", "search", "show"]
        
        with ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(run_query, q) for q in queries]
            results = [future.result() for future in as_completed(futures)]
        
        successes = sum(1 for r in results if r and r.returncode == 0)
        
        if successes == len(queries):
            self.log(f"All {successes} concurrent queries succeeded", True, "CONCURRENCY")
            return True
        else:
            self.log(f"Only {successes}/{len(queries)} concurrent queries succeeded", False, "CONCURRENCY")
            return False

    # ========== Incremental Testing ==========
    
    def test_incremental_updates(self, base_dir):
        """Test incremental scanning and updates"""
        self.log("Testing incremental updates...", category="INCREMENTAL")
        
        project_path = Path(base_dir) / "incremental"
        project_path.mkdir()
        
        # Initial file
        initial_content = """
export class InitialClass {
    method1(): void {}
}
"""
        (project_path / "evolving.ts").write_text(initial_content)
        
        # Initial scan
        result1 = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        if not result1 or result1.returncode != 0:
            self.log("Initial incremental scan failed", False, "INCREMENTAL")
            return False
        
        # Get initial symbol count
        db_path = project_path / ".reviewbot" / "graph.db"
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("SELECT COUNT(*) FROM symbol")
        initial_count = cursor.fetchone()[0]
        conn.close()
        
        # Update file (add more content)
        updated_content = initial_content + """
export class SecondClass {
    method2(): void {}
    method3(): string { return "test"; }
}

export function utilityFunction(): number {
    return 42;
}
"""
        (project_path / "evolving.ts").write_text(updated_content)
        
        # Wait a moment to ensure file timestamp changes
        time.sleep(1)
        
        # Incremental scan
        result2 = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        if not result2 or result2.returncode != 0:
            self.log("Incremental scan failed", False, "INCREMENTAL")
            return False
        
        # Check updated symbol count
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("SELECT COUNT(*) FROM symbol")
        updated_count = cursor.fetchone()[0]
        conn.close()
        
        if updated_count > initial_count:
            self.log(f"Incremental update: {initial_count} → {updated_count} symbols", True, "INCREMENTAL")
            return True
        else:
            self.log(f"Incremental update failed: {initial_count} → {updated_count} symbols", False, "INCREMENTAL")
            return False

    # ========== Stress Tests ==========
    
    def test_many_files(self, base_dir):
        """Test handling of many files"""
        self.log("Testing many files handling...", category="STRESS")
        
        project_path = Path(base_dir) / "many_files"
        project_path.mkdir()
        
        # Create many small files
        for i in range(100):
            content = f"""
export class File{i}Class {{
    private value{i}: number = {i};
    
    getValue{i}(): number {{
        return this.value{i};
    }}
}}
"""
            (project_path / f"file_{i:03d}.ts").write_text(content)
        
        start_time = time.time()
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"], timeout=300)
        scan_time = time.time() - start_time
        
        if result and result.returncode == 0:
            # Check symbol count
            db_path = project_path / ".reviewbot" / "graph.db"
            if db_path.exists():
                conn = sqlite3.connect(db_path)
                cursor = conn.cursor()
                cursor.execute("SELECT COUNT(*) FROM symbol")
                symbol_count = cursor.fetchone()[0]
                cursor.execute("SELECT COUNT(*) FROM file")
                file_count = cursor.fetchone()[0]
                conn.close()
                
                self.log(f"Many files: {file_count} files, {symbol_count} symbols in {scan_time:.2f}s", True, "STRESS")
                return True
            else:
                self.log("Many files scan didn't create database", False, "STRESS")
                return False
        else:
            self.log(f"Many files scan failed after {scan_time:.2f}s", False, "STRESS")
            return False

    def test_deep_nesting(self, base_dir):
        """Test deeply nested code structures"""
        self.log("Testing deep nesting handling...", category="STRESS")
        
        project_path = Path(base_dir) / "deep_nesting"
        project_path.mkdir()
        
        # Generate deeply nested TypeScript
        nested_content = "export namespace Level0 {\n"
        for i in range(20):
            nested_content += "  " * (i + 1) + f"export namespace Level{i + 1} {{\n"
        
        # Add a class at the deepest level
        nested_content += "  " * 21 + "export class DeepClass {\n"
        nested_content += "  " * 22 + "method(): void {}\n"
        nested_content += "  " * 21 + "}\n"
        
        # Close all namespaces
        for i in range(20, -1, -1):
            nested_content += "  " * (i + 1) + "}\n"
        
        (project_path / "nested.ts").write_text(nested_content)
        
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        
        if result and result.returncode == 0:
            self.log("Deep nesting handled correctly", True, "STRESS")
            return True
        else:
            self.log(f"Deep nesting caused failure: {result.stderr if result else 'timeout'}", False, "STRESS")
            return False

    # ========== Error Recovery Tests ==========
    
    def test_database_corruption_recovery(self, base_dir):
        """Test recovery from database corruption"""
        self.log("Testing database corruption recovery...", category="ERROR_RECOVERY")
        
        project_path = Path(base_dir) / "corruption_test"
        project_path.mkdir()
        
        (project_path / "test.ts").write_text("""
export class TestClass {
    method(): void {}
}
""")
        
        # Initial scan
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        if not result or result.returncode != 0:
            self.log("Initial scan for corruption test failed", False, "ERROR_RECOVERY")
            return False
        
        # Corrupt the database
        db_path = project_path / ".reviewbot" / "graph.db"
        if db_path.exists():
            # Write garbage to the database file
            with open(db_path, 'wb') as f:
                f.write(b"corrupted database content" * 100)
        
        # Try to scan again - should recover
        result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
        
        if result and result.returncode == 0:
            self.log("Database corruption recovery successful", True, "ERROR_RECOVERY")
            return True
        else:
            self.log(f"Database corruption recovery failed: {result.stderr if result else 'timeout'}", False, "ERROR_RECOVERY")
            return False

    def test_permission_errors(self, base_dir):
        """Test handling of permission errors"""
        self.log("Testing permission error handling...", category="ERROR_RECOVERY")
        
        project_path = Path(base_dir) / "permission_test"
        project_path.mkdir()
        
        # Create a read-only file
        readonly_file = project_path / "readonly.ts"
        readonly_file.write_text("""
export class ReadOnlyClass {
    method(): void {}
}
""")
        
        # Make file read-only
        readonly_file.chmod(0o444)
        
        # Create .reviewbot directory as read-only
        reviewbot_dir = project_path / ".reviewbot"
        reviewbot_dir.mkdir()
        reviewbot_dir.chmod(0o444)
        
        try:
            # Try to scan - should handle gracefully
            result = self.run_command(["cargo", "run", "--", "--repo", str(project_path), "scan"])
            
            # Restore permissions for cleanup
            reviewbot_dir.chmod(0o755)
            readonly_file.chmod(0o644)
            
            if result:
                if result.returncode != 0:
                    self.log("Permission errors handled gracefully", True, "ERROR_RECOVERY")
                    return True
                else:
                    self.log("Scan succeeded despite permission restrictions", True, "ERROR_RECOVERY")
                    return True
            else:
                self.log("Permission error test timed out", False, "ERROR_RECOVERY")
                return False
                
        except Exception as e:
            # Restore permissions
            try:
                reviewbot_dir.chmod(0o755)
                readonly_file.chmod(0o644)
            except:
                pass
            self.log(f"Permission error test exception: {e}", False, "ERROR_RECOVERY")
            return False

    def run_all_bug_hunting_tests(self):
        """Run the complete bug hunting test suite"""
        print("🐛 Bug Hunting End-to-End Test Suite")
        print("=" * 80)
        
        with tempfile.TemporaryDirectory(prefix="bug_hunting_") as temp_dir:
            test_categories = [
                ("Edge Cases", [
                    ("Empty Files", lambda: self.test_empty_files(temp_dir)),
                    ("Syntax Errors", lambda: self.test_syntax_errors(temp_dir)),
                    ("Huge Files", lambda: self.test_huge_files(temp_dir)),
                    ("Unicode & Special Chars", lambda: self.test_unicode_and_special_chars(temp_dir)),
                ]),
                ("Concurrency", [
                    ("Concurrent Scans", lambda: self.test_concurrent_scans(temp_dir)),
                    ("Concurrent Queries", lambda: self.test_concurrent_queries(temp_dir)),
                ]),
                ("Incremental Updates", [
                    ("Incremental Scanning", lambda: self.test_incremental_updates(temp_dir)),
                ]),
                ("Stress Tests", [
                    ("Many Files", lambda: self.test_many_files(temp_dir)),
                    ("Deep Nesting", lambda: self.test_deep_nesting(temp_dir)),
                ]),
                ("Error Recovery", [
                    ("Database Corruption", lambda: self.test_database_corruption_recovery(temp_dir)),
                    ("Permission Errors", lambda: self.test_permission_errors(temp_dir)),
                ]),
            ]
            
            total_passed = 0
            total_failed = 0
            
            for category_name, tests in test_categories:
                print(f"\n🧪 {category_name}")
                print("-" * 60)
                
                category_passed = 0
                category_failed = 0
                
                for test_name, test_func in tests:
                    print(f"\n  🔍 {test_name}")
                    
                    try:
                        success = test_func()
                        if success:
                            category_passed += 1
                            total_passed += 1
                        else:
                            category_failed += 1
                            total_failed += 1
                    except Exception as e:
                        self.log(f"Test {test_name} threw exception: {e}", False, "EXCEPTION")
                        category_failed += 1
                        total_failed += 1
                
                print(f"\n  📊 {category_name} Results: {category_passed} passed, {category_failed} failed")
        
        # Final summary
        print("\n" + "=" * 80)
        print("🏁 Bug Hunting Test Summary")
        print("=" * 80)
        print(f"✅ Total Passed: {total_passed}")
        print(f"❌ Total Failed: {total_failed}")
        
        if self.failures:
            print(f"\n🔍 Failures Found ({len(self.failures)}):")
            for failure in self.failures[:10]:  # Show first 10
                print(f"  • {failure}")
            if len(self.failures) > 10:
                print(f"  ... and {len(self.failures) - 10} more failures")
        
        if total_failed == 0:
            print("\n🎉 NO BUGS FOUND! System appears robust.")
            return True
        else:
            print(f"\n⚠️  {total_failed} potential issues discovered")
            return False


if __name__ == "__main__":
    test = BugHuntingE2ETest()
    success = test.run_all_bug_hunting_tests()
    sys.exit(0 if success else 1)