#!/usr/bin/env python3
"""
Performance and stress tests for Python API semantic integration
"""

import pytest
import tempfile
import shutil
from pathlib import Path
import sqlite3
import time
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from unittest.mock import patch, MagicMock

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent_api.code_graph import CodeGraph
from agent_api.simple_api import CodeGraphAPI
from agent_api.helpers import AgentHelpers


class TestPerformanceIntegration:
    """Performance and stress tests for semantic integration"""

    @pytest.fixture
    def large_typescript_project(self):
        """Create a large TypeScript project for performance testing"""
        temp_dir = tempfile.mkdtemp()
        project_path = Path(temp_dir)
        
        # Create package.json
        (project_path / "package.json").write_text("""
{
    "name": "large-test-project",
    "version": "1.0.0",
    "dependencies": {
        "typescript": "^5.0.0"
    }
}
""")
        
        # Create multiple modules with interconnections
        for i in range(50):  # 50 modules
            module_dir = project_path / f"module_{i}"
            module_dir.mkdir()
            
            # Main class file
            (module_dir / f"service_{i}.ts").write_text(f"""
import {{ BaseService }} from '../common/base_service';
import {{ Logger }} from '../utils/logger';
import {{ Config }} from '../config/config';

export class Service{i} extends BaseService {{
    private logger: Logger;
    private config: Config;
    private data: Map<string, any> = new Map();
    
    constructor() {{
        super();
        this.logger = new Logger(`Service{i}`);
        this.config = new Config();
    }}
    
    async processData(input: string[]): Promise<{{[key: string]: number}}> {{
        this.logger.info(`Processing ${{input.length}} items`);
        
        const result: {{[key: string]: number}} = {{}};
        
        for (let j = 0; j < input.length; j++) {{
            const item = input[j];
            result[item] = this.computeHash(item) + {i};
        }}
        
        await this.saveToCache(result);
        return result;
    }}
    
    private computeHash(input: string): number {{
        let hash = 0;
        for (let k = 0; k < input.length; k++) {{
            const char = input.charCodeAt(k);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash; // Convert to 32bit integer
        }}
        return hash;
    }}
    
    private async saveToCache(data: {{[key: string]: number}}): Promise<void> {{
        // Simulate async operation
        await new Promise(resolve => setTimeout(resolve, 1));
        
        Object.entries(data).forEach(([key, value]) => {{
            this.data.set(key, value);
        }});
    }}
    
    getStats(): {{count: number, keys: string[]}} {{
        return {{
            count: this.data.size,
            keys: Array.from(this.data.keys()).slice(0, 10)
        }};
    }}
}}
""")
            
            # Interface file
            (module_dir / f"interface_{i}.ts").write_text(f"""
export interface IService{i} {{
    processData(input: string[]): Promise<{{[key: string]: number}}>;
    getStats(): {{count: number, keys: string[]}};
}}

export interface IService{i}Config {{
    maxItems: number;
    timeout: number;
    retries: number;
}}

export interface IService{i}Result {{
    success: boolean;
    data?: {{[key: string]: number}};
    error?: string;
    processingTime: number;
}}
""")
            
            # Types file
            (module_dir / f"types_{i}.ts").write_text(f"""
export type Service{i}Status = 'idle' | 'processing' | 'error' | 'complete';

export type Service{i}Event = {{
    type: 'start' | 'progress' | 'complete' | 'error';
    timestamp: number;
    data?: any;
}};

export type Service{i}Metrics = {{
    totalProcessed: number;
    averageTime: number;
    errorRate: number;
    lastRun: Date;
}};
""")
        
        # Common shared modules
        common_dir = project_path / "common"
        common_dir.mkdir()
        
        (common_dir / "base_service.ts").write_text("""
export abstract class BaseService {
    protected id: string;
    protected created: Date;
    
    constructor() {
        this.id = Math.random().toString(36).substring(7);
        this.created = new Date();
    }
    
    getId(): string {
        return this.id;
    }
    
    getAge(): number {
        return Date.now() - this.created.getTime();
    }
}
""")
        
        # Utils directory
        utils_dir = project_path / "utils"
        utils_dir.mkdir()
        
        (utils_dir / "logger.ts").write_text("""
export class Logger {
    constructor(private name: string) {}
    
    info(message: string): void {
        console.log(`[${this.name}] INFO: ${message}`);
    }
    
    error(message: string): void {
        console.error(`[${this.name}] ERROR: ${message}`);
    }
}
""")
        
        # Config directory
        config_dir = project_path / "config"
        config_dir.mkdir()
        
        (config_dir / "config.ts").write_text("""
export class Config {
    private settings: Map<string, any> = new Map();
    
    constructor() {
        this.settings.set('maxConnections', 100);
        this.settings.set('timeout', 5000);
        this.settings.set('retries', 3);
    }
    
    get(key: string): any {
        return this.settings.get(key);
    }
    
    set(key: string, value: any): void {
        this.settings.set(key, value);
    }
}
""")
        
        yield project_path
        shutil.rmtree(temp_dir)

    @pytest.fixture
    def mock_large_database(self, large_typescript_project):
        """Create a large mock database with realistic data"""
        db_path = large_typescript_project / ".reviewbot" / "graph.db"
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
        
        conn.execute("""
            CREATE TABLE occurrences (
                file_path TEXT NOT NULL,
                symbol_id TEXT,
                role TEXT NOT NULL,
                line INTEGER NOT NULL,
                col INTEGER NOT NULL,
                token TEXT NOT NULL
            )
        """)
        
        # Insert large amounts of test data
        file_id = 1
        symbol_id = 1
        
        for i in range(50):  # 50 modules
            # Insert files
            files = [
                f"module_{i}/service_{i}.ts",
                f"module_{i}/interface_{i}.ts", 
                f"module_{i}/types_{i}.ts"
            ]
            
            for file_path in files:
                conn.execute("INSERT INTO files VALUES (?, ?, ?)", 
                           (file_id, file_path, f"sha{file_id:06d}"))
                
                # Insert symbols for this file
                for j in range(10):  # 10 symbols per file
                    conn.execute("""
                        INSERT INTO symbols VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """, (
                        f"symbol_{symbol_id}",
                        "TypeScript",
                        "class" if j % 3 == 0 else "function" if j % 3 == 1 else "variable",
                        f"Symbol{j}",
                        f"module_{i}.Symbol{j}",
                        f"signature_{j}",
                        file_id,
                        j + 1,
                        0
                    ))
                    
                    # Insert occurrences
                    for k in range(3):  # 3 occurrences per symbol
                        conn.execute("""
                            INSERT INTO occurrences VALUES (?, ?, ?, ?, ?, ?)
                        """, (
                            file_path,
                            f"symbol_{symbol_id}",
                            "definition" if k == 0 else "reference",
                            j + k + 1,
                            k * 10,
                            f"token_{symbol_id}_{k}"
                        ))
                    
                    symbol_id += 1
                
                file_id += 1
        
        # Insert cross-file edges (dependencies)
        for i in range(49):  # Dependencies between consecutive modules
            conn.execute("""
                INSERT INTO edges VALUES (?, ?, ?, ?, ?, ?)
            """, (
                "imports",
                f"symbol_{i * 30 + 1}",  # From module i
                f"symbol_{(i + 1) * 30 + 1}",  # To module i+1
                i * 3 + 1,
                (i + 1) * 3 + 1,
                "semantic"
            ))
        
        conn.commit()
        conn.close()
        
        return db_path

    def test_large_project_scan_performance(self, large_typescript_project):
        """Test scanning performance on large projects"""
        
        with patch('subprocess.run') as mock_run:
            # Simulate realistic scan time
            def slow_run(*args, **kwargs):
                time.sleep(0.1)  # Simulate some processing time
                return MagicMock(returncode=0)
            
            mock_run.side_effect = slow_run
            
            start_time = time.time()
            
            # Test semantic scan
            graph = CodeGraph(str(large_typescript_project), semantic=True)
            
            semantic_time = time.time() - start_time
            
            # Test syntactic scan
            start_time = time.time()
            graph_syntactic = CodeGraph(str(large_typescript_project), semantic=False)
            syntactic_time = time.time() - start_time
            
            # Both should complete within reasonable time
            assert semantic_time < 5.0, f"Semantic scan took {semantic_time}s, expected < 5s"
            assert syntactic_time < 5.0, f"Syntactic scan took {syntactic_time}s, expected < 5s"

    def test_concurrent_database_access(self, mock_large_database):
        """Test concurrent access to the database"""
        
        project_path = mock_large_database.parent.parent
        
        def worker_query(worker_id):
            """Worker function for concurrent testing"""
            api = CodeGraphAPI(str(project_path), check_same_thread=False, timeout=30.0)
            
            results = []
            for i in range(10):  # 10 queries per worker
                symbol = api.get_symbol(f"symbol_{worker_id * 10 + i + 1}")
                results.append(symbol is not None)
            
            return sum(results)  # Count successful queries
        
        # Test with multiple concurrent workers
        num_workers = 5
        with ThreadPoolExecutor(max_workers=num_workers) as executor:
            futures = [executor.submit(worker_query, i) for i in range(num_workers)]
            
            results = []
            for future in as_completed(futures, timeout=60):
                result = future.result()
                results.append(result)
        
        # All workers should have found some symbols
        assert len(results) == num_workers
        assert all(r > 0 for r in results), "All workers should find symbols"

    def test_memory_usage_large_dataset(self, mock_large_database):
        """Test memory usage doesn't grow excessively with large datasets"""
        
        project_path = mock_large_database.parent.parent
        
        import psutil
        import os
        
        process = psutil.Process(os.getpid())
        initial_memory = process.memory_info().rss / 1024 / 1024  # MB
        
        # Create multiple API instances
        apis = []
        for i in range(10):
            api = CodeGraphAPI(str(project_path), check_same_thread=False)
            apis.append(api)
        
        # Perform many queries
        for api in apis:
            for j in range(100):
                symbol = api.get_symbol(f"symbol_{j + 1}")
        
        final_memory = process.memory_info().rss / 1024 / 1024  # MB
        memory_increase = final_memory - initial_memory
        
        # Memory increase should be reasonable (less than 100MB for this test)
        assert memory_increase < 100, f"Memory increased by {memory_increase:.1f}MB, expected < 100MB"

    def test_query_performance_large_database(self, mock_large_database):
        """Test query performance on large database"""
        
        project_path = mock_large_database.parent.parent
        api = CodeGraphAPI(str(project_path))
        
        # Test symbol lookup performance
        start_time = time.time()
        
        found_symbols = 0
        for i in range(100):
            symbol = api.get_symbol(f"symbol_{i + 1}")
            if symbol:
                found_symbols += 1
        
        lookup_time = time.time() - start_time
        avg_time_per_lookup = lookup_time / 100
        
        assert avg_time_per_lookup < 0.01, f"Average lookup time {avg_time_per_lookup:.4f}s, expected < 0.01s"
        assert found_symbols > 90, f"Found {found_symbols} symbols, expected > 90"

    def test_helpers_performance_large_project(self, large_typescript_project):
        """Test AgentHelpers performance on large projects"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            start_time = time.time()
            
            # Test helpers initialization
            helpers = AgentHelpers(str(large_typescript_project), semantic=True)
            
            init_time = time.time() - start_time
            
            # Should initialize quickly
            assert init_time < 2.0, f"Helpers initialization took {init_time:.2f}s, expected < 2s"

    def test_database_cleanup_after_errors(self, large_typescript_project):
        """Test that database connections are properly cleaned up after errors"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Create database
            db_path = large_typescript_project / ".reviewbot" / "graph.db"
            db_path.parent.mkdir(exist_ok=True)
            
            conn = sqlite3.connect(db_path)
            conn.execute("CREATE TABLE test (id INTEGER)")
            conn.close()
            
            # Test that exceptions don't leave connections open
            for i in range(10):
                try:
                    api = CodeGraphAPI(str(large_typescript_project))
                    # Simulate an error during query
                    with patch.object(api.conn, 'cursor', side_effect=sqlite3.Error("Simulated error")):
                        api.get_symbol("nonexistent")
                except sqlite3.Error:
                    pass  # Expected
            
            # Should still be able to create new connections
            final_api = CodeGraphAPI(str(large_typescript_project))
            assert final_api.conn is not None

    def test_stress_test_rapid_initialization(self, large_typescript_project):
        """Stress test rapid initialization and cleanup"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Create minimal database
            db_path = large_typescript_project / ".reviewbot" / "graph.db"
            db_path.parent.mkdir(exist_ok=True)
            
            conn = sqlite3.connect(db_path)
            conn.execute("CREATE TABLE test (id INTEGER)")
            conn.close()
            
            # Rapidly create and destroy many API instances
            start_time = time.time()
            
            for i in range(50):
                api = CodeGraphAPI(str(large_typescript_project), timeout=1.0)
                # Do minimal work
                api.conn.execute("SELECT COUNT(*) FROM test")
                # Let it go out of scope (should auto-cleanup)
            
            total_time = time.time() - start_time
            avg_time = total_time / 50
            
            assert avg_time < 0.1, f"Average init time {avg_time:.3f}s, expected < 0.1s"
            assert total_time < 10, f"Total time {total_time:.1f}s, expected < 10s"

if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])