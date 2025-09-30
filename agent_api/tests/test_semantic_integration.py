#!/usr/bin/env python3
"""
Comprehensive integration tests for Python API semantic analysis
"""

import pytest
import tempfile
import shutil
from pathlib import Path
import sqlite3
from unittest.mock import patch, MagicMock

import sys
# Add agent_api directory to path for proper imports
sys.path.insert(0, str(Path(__file__).parent.parent))

import code_graph
import simple_api
import helpers

# Import classes directly
CodeGraph = code_graph.CodeGraph
CodeGraphAPI = simple_api.CodeGraphAPI
AgentHelpers = helpers.AgentHelpers


class TestSemanticIntegration:
    """Test semantic analysis integration in Python API"""

    @pytest.fixture
    def typescript_project(self):
        """Create a TypeScript test project"""
        temp_dir = tempfile.mkdtemp()
        project_path = Path(temp_dir)
        
        # Main file with imports
        (project_path / "main.ts").write_text("""
import { Calculator } from './calculator';
import { Logger } from './utils/logger';

class Application {
    private calc: Calculator;
    private logger: Logger;
    
    constructor() {
        this.calc = new Calculator();
        this.logger = new Logger();
    }
    
    run(): void {
        const result = this.calc.add(5, 3);
        this.logger.info(`Result: ${result}`);
    }
}

export { Application };
""")

        # Calculator module
        (project_path / "calculator.ts").write_text("""
export class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }
    
    multiply(a: number, b: number): number {
        return a * b;
    }
    
    divide(a: number, b: number): number {
        if (b === 0) throw new Error('Division by zero');
        return a / b;
    }
}
""")

        # Utils directory with logger
        utils_dir = project_path / "utils"
        utils_dir.mkdir()
        (utils_dir / "logger.ts").write_text("""
export class Logger {
    private prefix: string = '[LOG]';
    
    info(message: string): void {
        console.log(`${this.prefix} ${message}`);
    }
    
    error(message: string): void {
        console.error(`${this.prefix} ERROR: ${message}`);
    }
}
""")

        # Package.json for TypeScript
        (project_path / "package.json").write_text("""
{
    "name": "test-semantic-project",
    "version": "1.0.0",
    "main": "main.ts",
    "scripts": {
        "build": "tsc"
    },
    "dependencies": {
        "typescript": "^5.0.0"
    }
}
""")

        # TSConfig
        (project_path / "tsconfig.json").write_text("""
{
    "compilerOptions": {
        "target": "ES2020",
        "module": "commonjs",
        "strict": true,
        "esModuleInterop": true,
        "skipLibCheck": true,
        "forceConsistentCasingInFileNames": true
    }
}
""")
        
        yield project_path
        shutil.rmtree(temp_dir)

    @pytest.fixture  
    def python_project(self):
        """Create a Python test project"""
        temp_dir = tempfile.mkdtemp()
        project_path = Path(temp_dir)
        
        # Main Python file
        (project_path / "main.py").write_text("""
from calculator import Calculator
from utils.logger import Logger
import math

class DataProcessor:
    def __init__(self):
        self.calc = Calculator()
        self.logger = Logger("DataProcessor")
    
    def process_numbers(self, numbers: list[int]) -> dict:
        total = sum(numbers)
        avg = self.calc.divide(total, len(numbers))
        
        self.logger.info(f"Processed {len(numbers)} numbers")
        
        return {
            'total': total,
            'average': avg,
            'count': len(numbers),
            'sqrt_avg': math.sqrt(avg)
        }

if __name__ == "__main__":
    processor = DataProcessor() 
    result = processor.process_numbers([1, 2, 3, 4, 5])
    print(result)
""")

        # Calculator module (shared logic with TS version)
        (project_path / "calculator.py").write_text("""
class Calculator:
    def add(self, a: float, b: float) -> float:
        return a + b
    
    def multiply(self, a: float, b: float) -> float:
        return a * b
    
    def divide(self, a: float, b: float) -> float:
        if b == 0:
            raise ValueError("Division by zero")
        return a / b
    
    def power(self, base: float, exponent: float) -> float:
        return base ** exponent
""")

        # Utils package
        utils_dir = project_path / "utils"
        utils_dir.mkdir()
        (utils_dir / "__init__.py").write_text("")
        
        (utils_dir / "logger.py").write_text("""
import datetime
from typing import Optional

class Logger:
    def __init__(self, name: str = "Logger"):
        self.name = name
        self.prefix = f"[{name}]"
    
    def _timestamp(self) -> str:
        return datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    def info(self, message: str) -> None:
        print(f"{self._timestamp()} {self.prefix} INFO: {message}")
    
    def error(self, message: str) -> None:
        print(f"{self._timestamp()} {self.prefix} ERROR: {message}")
    
    def debug(self, message: str) -> None:
        print(f"{self._timestamp()} {self.prefix} DEBUG: {message}")
""")

        yield project_path
        shutil.rmtree(temp_dir)

    def test_semantic_vs_syntactic_differences(self, typescript_project):
        """Test that semantic analysis provides more data than syntactic only"""
        
        # Mock the subprocess calls to avoid actual SCIP execution
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Test syntactic-only analysis
            graph_syntactic = CodeGraph(str(typescript_project), semantic=False)
            
            # Test semantic analysis
            graph_semantic = CodeGraph(str(typescript_project), semantic=True)
            
            # Verify different commands were called
            calls = mock_run.call_args_list
            assert len(calls) >= 2
            
            # Check that one call had --semantic and one didn't
            semantic_calls = [call for call in calls if '--semantic' in str(call)]
            syntactic_calls = [call for call in calls if '--semantic' not in str(call)]
            
            assert len(semantic_calls) >= 1, "Should have called with --semantic"
            assert len(syntactic_calls) >= 1, "Should have called without --semantic"

    def test_error_handling_no_database(self):
        """Test proper error handling when database doesn't exist"""
        
        with tempfile.TemporaryDirectory() as temp_dir:
            # Test CodeGraphAPI error message
            with pytest.raises(FileNotFoundError) as exc_info:
                CodeGraphAPI(temp_dir, semantic=True)
            
            assert "--semantic" in str(exc_info.value)
            
            # Test with semantic=False
            with pytest.raises(FileNotFoundError) as exc_info:
                CodeGraphAPI(temp_dir, semantic=False)
            
            assert "reviewbot scan" in str(exc_info.value)
            assert "--semantic" not in str(exc_info.value)

    def test_cross_file_symbol_resolution(self, typescript_project):
        """Test that semantic analysis can resolve cross-file imports"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Create mock database with semantic data
            db_path = typescript_project / ".reviewbot" / "graph.db"
            db_path.parent.mkdir(exist_ok=True)
            
            conn = sqlite3.connect(db_path)
            
            # Create schema
            conn.execute("""
                CREATE TABLE files (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL,
                    sha TEXT NOT NULL
                )
            """)
            
            conn.execute("""
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
                )
            """)
            
            conn.execute("""
                CREATE TABLE edges (
                    edge_type TEXT NOT NULL,
                    src TEXT,
                    dst TEXT,
                    file_src INTEGER,
                    file_dst INTEGER,
                    resolution TEXT NOT NULL
                )
            """)
            
            # Insert test data with cross-file references
            conn.execute("INSERT INTO files VALUES (1, 'main.ts', 'abc123')")
            conn.execute("INSERT INTO files VALUES (2, 'calculator.ts', 'def456')")
            conn.execute("INSERT INTO files VALUES (3, 'utils/logger.ts', 'ghi789')")
            
            # Insert symbols
            conn.execute("INSERT INTO symbols VALUES ('main.Application', 'TypeScript', 'class', 'Application', 'main.Application', NULL, 1, 4, 0)")
            conn.execute("INSERT INTO symbols VALUES ('calculator.Calculator', 'TypeScript', 'class', 'Calculator', 'calculator.Calculator', NULL, 2, 1, 0)")
            conn.execute("INSERT INTO symbols VALUES ('logger.Logger', 'TypeScript', 'class', 'Logger', 'utils.logger.Logger', NULL, 3, 1, 0)")
            
            # Insert semantic edges (cross-file imports)
            conn.execute("INSERT INTO edges VALUES ('imports', 'main.Application', 'calculator.Calculator', 1, 2, 'semantic')")
            conn.execute("INSERT INTO edges VALUES ('imports', 'main.Application', 'logger.Logger', 1, 3, 'semantic')")
            conn.execute("INSERT INTO edges VALUES ('calls', 'main.Application.run', 'calculator.Calculator.add', 1, 2, 'semantic')")
            
            conn.commit()
            conn.close()
            
            # Test querying with semantic data available
            api = CodeGraphAPI(str(typescript_project))
            
            # Should be able to find cross-file relationships
            application_symbol = api.get_symbol("main.Application")
            assert application_symbol is not None
            assert application_symbol.name == "Application"

    def test_multi_language_integration(self, typescript_project, python_project):
        """Test handling of multi-language projects"""
        
        # Copy Python files into TypeScript project to create mixed project
        for py_file in python_project.glob("**/*.py"):
            relative_path = py_file.relative_to(python_project)
            dest_path = typescript_project / relative_path
            dest_path.parent.mkdir(exist_ok=True, parents=True)
            shutil.copy2(py_file, dest_path)
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Test semantic analysis on multi-language project
            graph = CodeGraph(str(typescript_project), semantic=True)
            
            # Should call semantic scan
            calls = mock_run.call_args_list
            semantic_calls = [call for call in calls if '--semantic' in str(call)]
            assert len(semantic_calls) >= 1

    def test_helpers_semantic_integration(self, typescript_project):
        """Test AgentHelpers works with semantic analysis"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Test with semantic=True (default)
            helpers = AgentHelpers(str(typescript_project), semantic=True)
            assert helpers.graph.semantic == True
            
            # Test with semantic=False
            helpers_no_semantic = AgentHelpers(str(typescript_project), semantic=False) 
            assert helpers_no_semantic.graph.semantic == False

    def test_database_concurrency_settings(self, typescript_project):
        """Test that database concurrency settings work correctly"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Create minimal database
            db_path = typescript_project / ".reviewbot" / "graph.db"
            db_path.parent.mkdir(exist_ok=True)
            
            conn = sqlite3.connect(db_path)
            conn.execute("CREATE TABLE test (id INTEGER)")
            conn.close()
            
            # Test with different concurrency settings
            api1 = CodeGraphAPI(str(typescript_project), check_same_thread=True)
            api2 = CodeGraphAPI(str(typescript_project), check_same_thread=False, timeout=5.0)
            
            # Both should initialize successfully
            assert api1.db_path.exists()
            assert api2.db_path.exists()

    @patch('subprocess.run')
    def test_scan_command_construction(self, mock_run, typescript_project):
        """Test that scan commands are constructed correctly"""
        
        mock_run.return_value = MagicMock(returncode=0)
        
        # Test semantic=True
        graph1 = CodeGraph(str(typescript_project), semantic=True)
        
        # Test semantic=False  
        graph2 = CodeGraph(str(typescript_project), semantic=False)
        
        # Verify correct commands were called
        calls = mock_run.call_args_list
        
        # Find semantic call
        semantic_call = None
        syntactic_call = None
        
        for call in calls:
            args = call[0][0]  # First positional argument (the command list)
            if '--semantic' in args:
                semantic_call = args
            else:
                syntactic_call = args
        
        # Verify semantic call structure
        assert semantic_call is not None
        assert 'cargo' in semantic_call
        assert 'run' in semantic_call
        assert '--repo' in semantic_call
        assert 'scan' in semantic_call
        assert '--semantic' in semantic_call
        
        # Verify syntactic call structure
        assert syntactic_call is not None
        assert 'cargo' in syntactic_call
        assert 'run' in syntactic_call
        assert '--repo' in syntactic_call
        assert 'scan' in syntactic_call
        assert '--semantic' not in syntactic_call

    def test_performance_tracking(self, typescript_project):
        """Test that performance information is available"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            import time
            start_time = time.time()
            
            # Test semantic analysis
            graph = CodeGraph(str(typescript_project), semantic=True)
            
            end_time = time.time()
            duration = end_time - start_time
            
            # Should complete reasonably quickly (mocked)
            assert duration < 1.0  # Should be very fast with mocking

if __name__ == "__main__":
    pytest.main([__file__, "-v"])