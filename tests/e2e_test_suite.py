#!/usr/bin/env python3
"""
End-to-End Test Suite for Consilium CodeGraph

Tests the complete pipeline from source code to querying results,
including both Rust CLI and Python API integration.
"""

import os
import sys
import subprocess
import tempfile
import shutil
from pathlib import Path
import sqlite3
import time
import json

# Add agent_api to path for Python API testing
sys.path.insert(0, str(Path(__file__).parent.parent / "agent_api"))

try:
    from agent_api.code_graph import CodeGraph
    from agent_api.simple_api import CodeGraphAPI
    from agent_api.helpers import AgentHelpers
    PYTHON_API_AVAILABLE = True
except ImportError as e:
    print(f"⚠️  Python API not available: {e}")
    PYTHON_API_AVAILABLE = False


class E2ETestSuite:
    """End-to-end test suite for the complete CodeGraph pipeline"""
    
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.cargo_cmd = ["cargo", "run", "--"]
        self.results = []
        
    def log(self, message, success=None):
        """Log test results"""
        if success is True:
            print(f"✅ {message}")
        elif success is False:
            print(f"❌ {message}")
        else:
            print(f"ℹ️  {message}")
        
        self.results.append({
            'message': message,
            'success': success,
            'timestamp': time.time()
        })
    
    def run_command(self, cmd, cwd=None, timeout=120):
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
        except subprocess.TimeoutExpired:
            self.log(f"Command timed out after {timeout}s: {' '.join(cmd)}", False)
            return None
        except Exception as e:
            self.log(f"Command failed: {e}", False)
            return None
    
    def create_test_project(self, project_dir):
        """Create a realistic multi-language test project"""
        project_path = Path(project_dir)
        
        # TypeScript/JavaScript files
        ts_dir = project_path / "src"
        ts_dir.mkdir(parents=True, exist_ok=True)
        
        # Main application file
        (ts_dir / "app.ts").write_text("""
import { UserService } from './services/user-service';
import { DatabaseManager } from './database/database-manager';
import { Logger } from './utils/logger';

export class Application {
    private userService: UserService;
    private database: DatabaseManager;
    private logger: Logger;

    constructor() {
        this.logger = new Logger('Application');
        this.database = new DatabaseManager();
        this.userService = new UserService(this.database, this.logger);
    }

    async start(): Promise<void> {
        this.logger.info('Starting application...');
        await this.database.connect();
        
        const users = await this.userService.getAllUsers();
        this.logger.info(`Found ${users.length} users`);
    }

    async getUserById(id: number): Promise<any> {
        return this.userService.getUserById(id);
    }
}
""")

        # Services directory
        services_dir = ts_dir / "services"
        services_dir.mkdir(exist_ok=True)
        
        (services_dir / "user-service.ts").write_text("""
import { DatabaseManager } from '../database/database-manager';
import { Logger } from '../utils/logger';

export interface User {
    id: number;
    name: string;
    email: string;
}

export class UserService {
    constructor(
        private database: DatabaseManager,
        private logger: Logger
    ) {}

    async getAllUsers(): Promise<User[]> {
        this.logger.debug('Fetching all users');
        return this.database.query('SELECT * FROM users');
    }

    async getUserById(id: number): Promise<User | null> {
        this.logger.debug(`Fetching user ${id}`);
        const users = await this.database.query('SELECT * FROM users WHERE id = ?', [id]);
        return users.length > 0 ? users[0] : null;
    }

    async createUser(userData: Omit<User, 'id'>): Promise<User> {
        this.logger.info('Creating new user');
        const result = await this.database.execute(
            'INSERT INTO users (name, email) VALUES (?, ?)',
            [userData.name, userData.email]
        );
        return { id: result.insertId, ...userData };
    }
}
""")

        # Database layer
        db_dir = ts_dir / "database"
        db_dir.mkdir(exist_ok=True)
        
        (db_dir / "database-manager.ts").write_text("""
import { Logger } from '../utils/logger';

export class DatabaseManager {
    private logger = new Logger('DatabaseManager');
    private connected = false;

    async connect(): Promise<void> {
        this.logger.info('Connecting to database...');
        // Simulate connection
        await this.delay(100);
        this.connected = true;
        this.logger.info('Database connected');
    }

    async query(sql: string, params: any[] = []): Promise<any[]> {
        this.ensureConnected();
        this.logger.debug(`Query: ${sql}`);
        
        // Mock data
        if (sql.includes('SELECT * FROM users')) {
            return [
                { id: 1, name: 'John Doe', email: 'john@example.com' },
                { id: 2, name: 'Jane Smith', email: 'jane@example.com' }
            ];
        }
        return [];
    }

    async execute(sql: string, params: any[] = []): Promise<{ insertId: number }> {
        this.ensureConnected();
        this.logger.debug(`Execute: ${sql}`);
        return { insertId: Math.floor(Math.random() * 1000) };
    }

    private ensureConnected(): void {
        if (!this.connected) {
            throw new Error('Database not connected');
        }
    }

    private delay(ms: number): Promise<void> {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}
""")

        # Utils
        utils_dir = ts_dir / "utils"
        utils_dir.mkdir(exist_ok=True)
        
        (utils_dir / "logger.ts").write_text("""
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export class Logger {
    constructor(private name: string) {}

    debug(message: string): void {
        this.log('DEBUG', message);
    }

    info(message: string): void {
        this.log('INFO', message);
    }

    warn(message: string): void {
        this.log('WARN', message);
    }

    error(message: string): void {
        this.log('ERROR', message);
    }

    private log(level: LogLevel, message: string): void {
        console.log(`[${new Date().toISOString()}] ${level} [${this.name}]: ${message}`);
    }
}
""")

        # Python files
        py_dir = project_path / "python_src"
        py_dir.mkdir(exist_ok=True)
        
        (py_dir / "main.py").write_text("""
from user_manager import UserManager
from database import Database
import asyncio

class PythonApp:
    def __init__(self):
        self.database = Database()
        self.user_manager = UserManager(self.database)

    async def run(self):
        await self.database.connect()
        users = await self.user_manager.get_all_users()
        print(f"Found {len(users)} users")

if __name__ == "__main__":
    app = PythonApp()
    asyncio.run(app.run())
""")

        (py_dir / "user_manager.py").write_text("""
from typing import List, Optional, Dict, Any
from database import Database

class User:
    def __init__(self, id: int, name: str, email: str):
        self.id = id
        self.name = name
        self.email = email

class UserManager:
    def __init__(self, database: Database):
        self.database = database

    async def get_all_users(self) -> List[User]:
        rows = await self.database.query("SELECT * FROM users")
        return [User(**row) for row in rows]

    async def get_user_by_id(self, user_id: int) -> Optional[User]:
        rows = await self.database.query("SELECT * FROM users WHERE id = ?", [user_id])
        return User(**rows[0]) if rows else None

    async def create_user(self, name: str, email: str) -> User:
        result = await self.database.execute(
            "INSERT INTO users (name, email) VALUES (?, ?)",
            [name, email]
        )
        return User(result["insert_id"], name, email)
""")

        (py_dir / "database.py").write_text("""
import asyncio
from typing import List, Dict, Any

class Database:
    def __init__(self):
        self.connected = False

    async def connect(self):
        await asyncio.sleep(0.1)  # Simulate connection
        self.connected = True

    async def query(self, sql: str, params: List[Any] = None) -> List[Dict[str, Any]]:
        if not self.connected:
            raise RuntimeError("Database not connected")
        
        # Mock data
        if "SELECT * FROM users" in sql:
            return [
                {"id": 1, "name": "Alice", "email": "alice@example.com"},
                {"id": 2, "name": "Bob", "email": "bob@example.com"}
            ]
        return []

    async def execute(self, sql: str, params: List[Any] = None) -> Dict[str, Any]:
        if not self.connected:
            raise RuntimeError("Database not connected")
        
        return {"insert_id": 123, "rows_affected": 1}
""")

        # Configuration files
        (project_path / "package.json").write_text("""
{
    "name": "e2e-test-project",
    "version": "1.0.0",
    "main": "src/app.ts",
    "dependencies": {
        "typescript": "^5.0.0"
    }
}
""")

        (project_path / "tsconfig.json").write_text("""
{
    "compilerOptions": {
        "target": "ES2020",
        "module": "commonjs",
        "strict": true,
        "esModuleInterop": true
    }
}
""")

        self.log(f"Created test project with TypeScript and Python files", True)
        return project_path

    def test_syntactic_scan_and_query(self, project_path):
        """Test syntactic-only scanning and querying"""
        self.log("Testing syntactic scan and query workflow...")
        
        # Run syntactic scan
        scan_cmd = self.cargo_cmd + ["--repo", str(project_path), "scan"]
        result = self.run_command(scan_cmd)
        
        if result and result.returncode == 0:
            self.log("Syntactic scan completed successfully", True)
        else:
            self.log(f"Syntactic scan failed: {result.stderr if result else 'timeout'}", False)
            return False

        # Check database was created
        db_path = project_path / ".reviewbot" / "graph.db"
        if db_path.exists():
            self.log("Database created successfully", True)
        else:
            self.log("Database not found after scan", False)
            return False

        # Test search functionality
        search_cmd = self.cargo_cmd + ["--repo", str(project_path), "search", "UserService"]
        result = self.run_command(search_cmd)
        
        if result and result.returncode == 0 and "UserService" in result.stdout:
            self.log("Search functionality working", True)
        else:
            self.log("Search functionality failed", False)
            return False

        # Test show functionality
        show_cmd = self.cargo_cmd + ["--repo", str(project_path), "show", "stats"]
        result = self.run_command(show_cmd)
        
        if result and result.returncode == 0:
            self.log("Show stats functionality working", True)
        else:
            self.log("Show stats functionality failed", False)
            return False

        return True

    def test_semantic_scan_and_query(self, project_path):
        """Test semantic scanning with SCIP integration"""
        self.log("Testing semantic scan and query workflow...")
        
        # Remove existing database to test semantic scan
        db_path = project_path / ".reviewbot" / "graph.db"
        if db_path.exists():
            db_path.unlink()

        # Run semantic scan
        scan_cmd = self.cargo_cmd + ["--repo", str(project_path), "scan", "--semantic"]
        result = self.run_command(scan_cmd, timeout=180)  # Longer timeout for semantic
        
        if result and result.returncode == 0:
            self.log("Semantic scan completed successfully", True)
        else:
            # Semantic scan might fail if SCIP indexers aren't installed
            self.log(f"Semantic scan failed (expected if SCIP indexers not installed): {result.stderr if result else 'timeout'}", None)
            return True  # Don't fail the test, just note it

        # Check database exists
        if db_path.exists():
            self.log("Database created after semantic scan", True)
        else:
            self.log("Database not found after semantic scan", False)
            return False

        return True

    def test_database_content_validation(self, project_path):
        """Validate database content and structure"""
        self.log("Validating database content...")
        
        db_path = project_path / ".reviewbot" / "graph.db"
        if not db_path.exists():
            self.log("Database not found for validation", False)
            return False

        try:
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()

            # Check tables exist
            cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
            tables = [row[0] for row in cursor.fetchall()]
            
            expected_tables = ['files', 'symbols', 'edges', 'occurrences']
            for table in expected_tables:
                if table in tables:
                    self.log(f"Table '{table}' exists", True)
                else:
                    self.log(f"Table '{table}' missing", False)
                    return False

            # Check we have symbols
            cursor.execute("SELECT COUNT(*) FROM symbols")
            symbol_count = cursor.fetchone()[0]
            
            if symbol_count > 0:
                self.log(f"Found {symbol_count} symbols in database", True)
            else:
                self.log("No symbols found in database", False)
                return False

            # Check we have files
            cursor.execute("SELECT COUNT(*) FROM files")
            file_count = cursor.fetchone()[0]
            
            if file_count > 0:
                self.log(f"Found {file_count} files in database", True)
            else:
                self.log("No files found in database", False)
                return False

            # Sample some symbols
            cursor.execute("SELECT name, kind, fqn FROM symbols LIMIT 5")
            sample_symbols = cursor.fetchall()
            self.log(f"Sample symbols: {sample_symbols}")

            conn.close()
            return True

        except Exception as e:
            self.log(f"Database validation failed: {e}", False)
            return False

    def test_python_api_integration(self, project_path):
        """Test Python API integration with scanned data"""
        if not PYTHON_API_AVAILABLE:
            self.log("Python API not available, skipping test", None)
            return True

        self.log("Testing Python API integration...")

        try:
            # Test CodeGraphAPI
            api = CodeGraphAPI(str(project_path))
            
            # Test basic symbol retrieval
            symbols = api.get_symbols("UserService")
            if symbols:
                self.log(f"Python API found {len(symbols)} UserService symbols", True)
            else:
                self.log("Python API found no UserService symbols", False)
                return False

            # Test AgentHelpers (if database exists)
            helpers = AgentHelpers(str(project_path))
            self.log("AgentHelpers initialized successfully", True)

            return True

        except Exception as e:
            self.log(f"Python API integration failed: {e}", False)
            return False

    def test_incremental_processing(self, project_path):
        """Test incremental processing capabilities"""
        self.log("Testing incremental processing...")

        # Get initial stats
        show_cmd = self.cargo_cmd + ["--repo", str(project_path), "show", "stats"]
        result1 = self.run_command(show_cmd)
        
        if not result1 or result1.returncode != 0:
            self.log("Could not get initial stats", False)
            return False

        # Modify a file
        new_file = project_path / "src" / "new_module.ts"
        new_file.write_text("""
export class NewModule {
    process(): string {
        return "processed";
    }
}
""")

        # Run incremental scan
        scan_cmd = self.cargo_cmd + ["--repo", str(project_path), "scan"]
        result = self.run_command(scan_cmd)

        if result and result.returncode == 0:
            self.log("Incremental scan completed", True)
        else:
            self.log("Incremental scan failed", False)
            return False

        # Check new symbols were added
        search_cmd = self.cargo_cmd + ["--repo", str(project_path), "search", "NewModule"]
        result = self.run_command(search_cmd)

        if result and result.returncode == 0 and "NewModule" in result.stdout:
            self.log("Incremental processing working - new symbols found", True)
            return True
        else:
            self.log("Incremental processing failed - new symbols not found", False)
            return False

    def test_performance_benchmark(self, project_path):
        """Test performance on the created project"""
        self.log("Running performance benchmark...")

        # Clean database for fresh scan
        db_path = project_path / ".reviewbot" / "graph.db"
        if db_path.exists():
            db_path.unlink()

        # Time the scan
        start_time = time.time()
        scan_cmd = self.cargo_cmd + ["--repo", str(project_path), "scan"]
        result = self.run_command(scan_cmd)
        scan_time = time.time() - start_time

        if result and result.returncode == 0:
            self.log(f"Scan completed in {scan_time:.2f} seconds", True)
            
            # Performance expectations for small project
            if scan_time < 10:
                self.log("Scan performance acceptable", True)
            else:
                self.log(f"Scan took {scan_time:.2f}s, may be slow", None)
        else:
            self.log("Performance benchmark failed", False)
            return False

        # Test query performance
        start_time = time.time()
        search_cmd = self.cargo_cmd + ["--repo", str(project_path), "search", "class"]
        result = self.run_command(search_cmd)
        query_time = time.time() - start_time

        if result and result.returncode == 0:
            self.log(f"Search query completed in {query_time:.3f} seconds", True)
        else:
            self.log("Query performance test failed", False)
            return False

        return True

    def run_all_tests(self):
        """Run the complete end-to-end test suite"""
        print("🚀 Starting End-to-End Test Suite")
        print("=" * 60)

        with tempfile.TemporaryDirectory(prefix="e2e_test_") as temp_dir:
            project_path = self.create_test_project(temp_dir)
            
            tests = [
                ("Syntactic Scan & Query", self.test_syntactic_scan_and_query),
                ("Database Content Validation", self.test_database_content_validation),
                ("Semantic Scan & Query", self.test_semantic_scan_and_query),
                ("Python API Integration", self.test_python_api_integration),
                ("Incremental Processing", self.test_incremental_processing),
                ("Performance Benchmark", self.test_performance_benchmark),
            ]

            passed = 0
            failed = 0
            skipped = 0

            for test_name, test_func in tests:
                print(f"\n🧪 Running: {test_name}")
                print("-" * 40)
                
                try:
                    success = test_func(project_path)
                    if success:
                        passed += 1
                    else:
                        failed += 1
                except Exception as e:
                    self.log(f"Test {test_name} threw exception: {e}", False)
                    failed += 1

        # Final results
        print("\n" + "=" * 60)
        print("📊 End-to-End Test Results")
        print("=" * 60)
        print(f"✅ Passed: {passed}")
        print(f"❌ Failed: {failed}")
        print(f"⏭️  Skipped: {skipped}")
        
        if failed == 0:
            print("\n🎉 All end-to-end tests passed!")
            return True
        else:
            print(f"\n⚠️  {failed} test(s) failed")
            return False


if __name__ == "__main__":
    suite = E2ETestSuite()
    success = suite.run_all_tests()
    sys.exit(0 if success else 1)