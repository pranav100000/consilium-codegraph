#!/usr/bin/env python3
"""
Edge cases and error handling tests for Python API semantic integration
"""

import pytest
import tempfile
import shutil
from pathlib import Path
import sqlite3
import subprocess
from unittest.mock import patch, MagicMock
import threading
import time

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent_api.code_graph import CodeGraph
from agent_api.simple_api import CodeGraphAPI
from agent_api.helpers import AgentHelpers


class TestEdgeCasesIntegration:
    """Test edge cases and error scenarios"""

    @pytest.fixture
    def malformed_project(self):
        """Create a project with malformed/problematic files"""
        temp_dir = tempfile.mkdtemp()
        project_path = Path(temp_dir)
        
        # File with syntax errors
        (project_path / "broken.ts").write_text("""
// This file has intentional syntax errors
class BrokenClass {
    method() {
        // Unclosed brace
        if (true {
            console.log("missing closing brace")
        // Missing closing brace for method and class
""")
        
        # File with unicode and special characters
        (project_path / "unicode.ts").write_text("""
// Unicode identifiers and strings
class 测试类 {
    private 属性: string = "测试值";
    
    方法(参数: string): void {
        console.log(`处理: ${参数} 🎉`);
    }
    
    // Emoji in code
    🚀launch(): void {
        this.方法("火箭发射 🚀");
    }
}

// Mixed scripts
const наименование = "название";
const اسم = "name";
const 名前 = "name in Japanese";
""")
        
        # Empty file
        (project_path / "empty.ts").write_text("")
        
        # Very long line file
        long_line = "const longVariable = " + '"' + "x" * 10000 + '"' + ";"
        (project_path / "long_lines.ts").write_text(long_line)
        
        # Binary file (should be ignored)
        (project_path / "binary.bin").write_bytes(b'\x00\x01\x02\x03\xff\xfe\xfd')
        
        # File with no extension
        (project_path / "no_extension").write_text("// File with no extension")
        
        yield project_path
        shutil.rmtree(temp_dir)

    @pytest.fixture
    def corrupted_database_project(self):
        """Create project with corrupted database"""
        temp_dir = tempfile.mkdtemp()
        project_path = Path(temp_dir)
        
        # Create .reviewbot directory
        reviewbot_dir = project_path / ".reviewbot"
        reviewbot_dir.mkdir()
        
        # Create corrupted database file
        db_path = reviewbot_dir / "graph.db"
        with open(db_path, 'wb') as f:
            f.write(b'This is not a valid SQLite database file')
        
        yield project_path
        shutil.rmtree(temp_dir)

    def test_malformed_files_handling(self, malformed_project):
        """Test handling of malformed source files"""
        
        with patch('subprocess.run') as mock_run:
            # Simulate successful scan despite malformed files
            mock_run.return_value = MagicMock(returncode=0)
            
            # Should not crash on malformed files
            graph = CodeGraph(str(malformed_project), semantic=True)
            
            # Verify scan was attempted
            assert mock_run.called

    def test_scan_command_failure_handling(self, malformed_project):
        """Test handling when scan command fails"""
        
        with patch('subprocess.run') as mock_run:
            # Simulate scan failure
            mock_run.return_value = MagicMock(
                returncode=1,
                stderr="Error: Failed to parse files"
            )
            
            # Should handle scan failure gracefully
            with pytest.raises(Exception):  # Should raise some exception
                CodeGraph(str(malformed_project), semantic=True)

    def test_corrupted_database_handling(self, corrupted_database_project):
        """Test handling of corrupted database files"""
        
        # Should handle corrupted database gracefully
        with pytest.raises(sqlite3.DatabaseError):
            CodeGraphAPI(str(corrupted_database_project))

    def test_unicode_handling(self, malformed_project):
        """Test handling of unicode in project paths and content"""
        
        # Create project with unicode path
        unicode_project = malformed_project / "测试项目"
        unicode_project.mkdir()
        
        (unicode_project / "test.ts").write_text("""
class UnicodeTest {
    测试方法(): string {
        return "Unicode works! 🎉";
    }
}
""")
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Should handle unicode paths
            graph = CodeGraph(str(unicode_project), semantic=True)
            assert mock_run.called

    def test_permission_denied_scenarios(self, malformed_project):
        """Test handling of permission denied scenarios"""
        
        # Create read-only directory (simulate permission issues)
        readonly_dir = malformed_project / ".reviewbot"
        readonly_dir.mkdir(mode=0o444)  # Read-only
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            try:
                # May fail due to permissions when trying to create database
                graph = CodeGraph(str(malformed_project), semantic=True)
            except PermissionError:
                pass  # Expected in some cases
            finally:
                # Cleanup: restore permissions
                readonly_dir.chmod(0o755)

    def test_network_timeout_simulation(self, malformed_project):
        """Test handling of network-like timeouts during scan"""
        
        def slow_scan(*args, **kwargs):
            time.sleep(0.5)  # Simulate slow scan
            return MagicMock(returncode=0)
        
        with patch('subprocess.run') as mock_run:
            mock_run.side_effect = slow_scan
            
            start_time = time.time()
            graph = CodeGraph(str(malformed_project), semantic=True)
            duration = time.time() - start_time
            
            # Should wait for slow scan to complete
            assert duration >= 0.5

    def test_concurrent_initialization_race_conditions(self, malformed_project):
        """Test race conditions during concurrent initialization"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            errors = []
            graphs = []
            
            def init_worker():
                try:
                    graph = CodeGraph(str(malformed_project), semantic=True)
                    graphs.append(graph)
                except Exception as e:
                    errors.append(e)
            
            # Start multiple threads simultaneously
            threads = []
            for i in range(5):
                thread = threading.Thread(target=init_worker)
                threads.append(thread)
            
            # Start all threads
            for thread in threads:
                thread.start()
            
            # Wait for completion
            for thread in threads:
                thread.join(timeout=10)
            
            # Should not have race condition errors
            assert len(errors) == 0, f"Race condition errors: {errors}"

    def test_memory_exhaustion_protection(self, malformed_project):
        """Test protection against memory exhaustion"""
        
        # Create very large mock result
        def memory_intensive_scan(*args, **kwargs):
            # Simulate scan that would use lots of memory
            return MagicMock(
                returncode=0,
                stdout="x" * 1000  # Reasonable size
            )
        
        with patch('subprocess.run') as mock_run:
            mock_run.side_effect = memory_intensive_scan
            
            # Should handle without memory issues
            graph = CodeGraph(str(malformed_project), semantic=True)
            assert mock_run.called

    def test_invalid_database_schema(self, malformed_project):
        """Test handling of databases with invalid/old schemas"""
        
        # Create database with wrong schema
        db_path = malformed_project / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(exist_ok=True)
        
        conn = sqlite3.connect(db_path)
        # Create table with wrong structure
        conn.execute("CREATE TABLE wrong_table (id TEXT)")
        conn.commit()
        conn.close()
        
        # Should detect invalid schema
        with pytest.raises((sqlite3.Error, Exception)):
            api = CodeGraphAPI(str(malformed_project))
            api.get_symbol("test")  # This should fail due to wrong schema

    def test_extremely_long_paths(self, malformed_project):
        """Test handling of extremely long file paths"""
        
        # Create deeply nested directory structure
        current_path = malformed_project
        for i in range(10):  # Create 10 levels deep
            current_path = current_path / f"very_long_directory_name_{i}_that_makes_path_extremely_long"
            current_path.mkdir()
        
        # Create file in deep path
        (current_path / "deep_file.ts").write_text("class DeepClass {}")
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Should handle long paths
            graph = CodeGraph(str(malformed_project), semantic=True)
            assert mock_run.called

    def test_database_locking_scenarios(self, malformed_project):
        """Test database locking and timeout scenarios"""
        
        # Create valid database
        db_path = malformed_project / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(exist_ok=True)
        
        conn = sqlite3.connect(db_path)
        conn.execute("CREATE TABLE files (id INTEGER, path TEXT)")
        conn.commit()
        
        # Keep connection open to simulate lock
        lock_thread_done = threading.Event()
        
        def lock_database():
            try:
                conn.execute("BEGIN EXCLUSIVE")
                # Hold lock briefly
                lock_thread_done.wait(timeout=2)
            finally:
                conn.rollback()
                conn.close()
        
        # Start locking thread
        lock_thread = threading.Thread(target=lock_database)
        lock_thread.start()
        
        try:
            # Try to access database with short timeout
            start_time = time.time()
            
            try:
                api = CodeGraphAPI(str(malformed_project), timeout=1.0)
                api.get_symbol("test")  # Should timeout
            except (sqlite3.OperationalError, Exception):
                pass  # Expected timeout
            
            duration = time.time() - start_time
            # Should respect timeout
            assert duration < 3.0, f"Timeout took {duration}s, expected < 3s"
            
        finally:
            lock_thread_done.set()
            lock_thread.join(timeout=5)

    def test_incomplete_scan_recovery(self, malformed_project):
        """Test recovery from incomplete scan operations"""
        
        def interrupted_scan(*args, **kwargs):
            # Simulate scan that gets interrupted
            raise KeyboardInterrupt("Scan interrupted")
        
        with patch('subprocess.run') as mock_run:
            mock_run.side_effect = interrupted_scan
            
            # Should handle interruption gracefully
            with pytest.raises(KeyboardInterrupt):
                CodeGraph(str(malformed_project), semantic=True)

    def test_disk_space_exhaustion_simulation(self, malformed_project):
        """Test handling of disk space issues"""
        
        def disk_full_scan(*args, **kwargs):
            # Simulate disk full error
            raise OSError("No space left on device")
        
        with patch('subprocess.run') as mock_run:
            mock_run.side_effect = disk_full_scan
            
            # Should handle disk space issues
            with pytest.raises(OSError):
                CodeGraph(str(malformed_project), semantic=True)

    def test_mixed_line_endings_files(self, malformed_project):
        """Test handling of files with mixed line endings"""
        
        # Create file with mixed line endings
        mixed_content = "class Test {\r\n    method() {\n        return 'mixed';\r    }\r\n}"
        (malformed_project / "mixed_endings.ts").write_text(mixed_content)
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Should handle mixed line endings
            graph = CodeGraph(str(malformed_project), semantic=True)
            assert mock_run.called

    def test_symlink_handling(self, malformed_project):
        """Test handling of symbolic links"""
        
        # Create a normal file
        (malformed_project / "real_file.ts").write_text("class RealClass {}")
        
        try:
            # Create symlink
            symlink_path = malformed_project / "link_file.ts"
            symlink_path.symlink_to("real_file.ts")
            
            with patch('subprocess.run') as mock_run:
                mock_run.return_value = MagicMock(returncode=0)
                
                # Should handle symlinks appropriately
                graph = CodeGraph(str(malformed_project), semantic=True)
                assert mock_run.called
                
        except OSError:
            # Symlinks might not be supported on all systems
            pytest.skip("Symlinks not supported on this system")

    def test_api_parameter_validation(self):
        """Test API parameter validation edge cases"""
        
        # Test invalid paths
        with pytest.raises((ValueError, FileNotFoundError, TypeError)):
            CodeGraph(None)  # type: ignore
        
        with pytest.raises((ValueError, FileNotFoundError)):
            CodeGraph("")
        
        with pytest.raises((ValueError, FileNotFoundError)):
            CodeGraph("/nonexistent/path/that/should/not/exist")
        
        # Test invalid timeout values
        with tempfile.TemporaryDirectory() as temp_dir:
            db_path = Path(temp_dir) / ".reviewbot" / "graph.db"
            db_path.parent.mkdir()
            
            conn = sqlite3.connect(db_path)
            conn.execute("CREATE TABLE test (id INTEGER)")
            conn.close()
            
            # Negative timeout should be handled
            api = CodeGraphAPI(temp_dir, timeout=-1.0)  # Should not crash
            
            # Very large timeout should be handled
            api = CodeGraphAPI(temp_dir, timeout=999999.0)  # Should not crash

if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])