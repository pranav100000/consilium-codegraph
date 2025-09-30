#!/usr/bin/env python3
"""
Shared pytest configuration and fixtures for integration tests
"""

import pytest
import tempfile
import shutil
from pathlib import Path
import sqlite3
import os
import sys

# Add the parent directory to the path so we can import the API modules
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


@pytest.fixture(scope="session")
def test_repo_path():
    """Path to the test repository used for integration tests"""
    return Path(__file__).parent.parent.parent / "test_ts_project"


@pytest.fixture
def temp_project():
    """Create a temporary project directory for testing"""
    temp_dir = tempfile.mkdtemp(prefix="consilium_test_")
    project_path = Path(temp_dir)
    
    yield project_path
    
    # Cleanup
    shutil.rmtree(temp_dir, ignore_errors=True)


@pytest.fixture
def minimal_database(temp_project):
    """Create a minimal test database"""
    db_path = temp_project / ".reviewbot" / "graph.db"
    db_path.parent.mkdir(parents=True, exist_ok=True)
    
    conn = sqlite3.connect(db_path)
    
    # Create minimal schema
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
        
        CREATE TABLE occurrences (
            file_path TEXT NOT NULL,
            symbol_id TEXT,
            role TEXT NOT NULL,
            line INTEGER NOT NULL,
            col INTEGER NOT NULL,
            token TEXT NOT NULL
        );
        
        -- Insert test data
        INSERT INTO files VALUES (1, 'test.ts', 'abc123');
        INSERT INTO symbols VALUES 
            ('test.TestClass', 'TypeScript', 'class', 'TestClass', 'test.TestClass', NULL, 1, 1, 0),
            ('test.testFunction', 'TypeScript', 'function', 'testFunction', 'test.testFunction', 'function testFunction(): void', 1, 5, 0);
        INSERT INTO occurrences VALUES 
            ('test.ts', 'test.TestClass', 'definition', 1, 0, 'TestClass'),
            ('test.ts', 'test.testFunction', 'definition', 5, 0, 'testFunction');
    """)
    
    conn.commit()
    conn.close()
    
    return db_path


@pytest.fixture
def sample_typescript_files(temp_project):
    """Create sample TypeScript files for testing"""
    files = {}
    
    # Main file
    main_content = """
import { Helper } from './helper';

export class MainClass {
    private helper: Helper;
    
    constructor() {
        this.helper = new Helper();
    }
    
    public doSomething(): string {
        return this.helper.process('test');
    }
}
"""
    main_file = temp_project / "main.ts"
    main_file.write_text(main_content)
    files['main.ts'] = main_file
    
    # Helper file
    helper_content = """
export class Helper {
    public process(input: string): string {
        return `Processed: ${input}`;
    }
    
    public validate(input: string): boolean {
        return input.length > 0;
    }
}
"""
    helper_file = temp_project / "helper.ts"
    helper_file.write_text(helper_content)
    files['helper.ts'] = helper_file
    
    # Package.json
    package_content = """
{
    "name": "test-project",
    "version": "1.0.0",
    "dependencies": {
        "typescript": "^5.0.0"
    }
}
"""
    package_file = temp_project / "package.json"
    package_file.write_text(package_content)
    files['package.json'] = package_file
    
    return files


@pytest.fixture
def sample_python_files(temp_project):
    """Create sample Python files for testing"""
    files = {}
    
    # Main file
    main_content = """
from helper import Helper

class MainClass:
    def __init__(self):
        self.helper = Helper()
    
    def do_something(self) -> str:
        return self.helper.process('test')
"""
    main_file = temp_project / "main.py"
    main_file.write_text(main_content)
    files['main.py'] = main_file
    
    # Helper file
    helper_content = """
class Helper:
    def process(self, input_str: str) -> str:
        return f"Processed: {input_str}"
    
    def validate(self, input_str: str) -> bool:
        return len(input_str) > 0
"""
    helper_file = temp_project / "helper.py"
    helper_file.write_text(helper_content)
    files['helper.py'] = helper_file
    
    return files


@pytest.fixture(autouse=True)
def cleanup_environment():
    """Ensure clean environment for each test"""
    # Store original environment
    original_env = os.environ.copy()
    
    yield
    
    # Restore original environment
    os.environ.clear()
    os.environ.update(original_env)


def pytest_configure(config):
    """Configure pytest"""
    config.addinivalue_line(
        "markers", "slow: marks tests as slow (deselect with '-m \"not slow\"')"
    )
    config.addinivalue_line(
        "markers", "integration: marks tests as integration tests"
    )
    config.addinivalue_line(
        "markers", "unit: marks tests as unit tests"
    )


def pytest_collection_modifyitems(config, items):
    """Modify test collection to add markers"""
    for item in items:
        # Mark integration tests
        if "integration" in item.nodeid:
            item.add_marker(pytest.mark.integration)
        
        # Mark slow tests
        if any(keyword in item.name.lower() for keyword in ['performance', 'stress', 'large', 'concurrent']):
            item.add_marker(pytest.mark.slow)


@pytest.fixture
def mock_scip_output():
    """Mock SCIP output for testing"""
    return {
        "metadata": {
            "tool_info": {
                "name": "scip-typescript",
                "version": "0.3.16"
            },
            "project_root": "/test/project",
            "text_document_encoding": 1
        },
        "documents": [
            {
                "relative_path": "main.ts",
                "symbols": [
                    {
                        "symbol": "scip-typescript npm . . `main.ts`/MainClass#",
                        "documentation": ["Main application class"],
                        "relationships": []
                    }
                ],
                "occurrences": [
                    {
                        "range": [5, 13, 22],
                        "symbol": "scip-typescript npm . . `main.ts`/MainClass#",
                        "symbol_roles": 1
                    }
                ]
            }
        ]
    }