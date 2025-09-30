#!/usr/bin/env python3
"""
End-to-end integration tests for Python API with real SCIP analysis
"""

import pytest
import tempfile
import shutil
from pathlib import Path
import sqlite3
import subprocess
import json
from unittest.mock import patch, MagicMock

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent_api.code_graph import CodeGraph
from agent_api.simple_api import CodeGraphAPI
from agent_api.helpers import AgentHelpers


class TestEndToEndIntegration:
    """End-to-end tests that exercise the full pipeline"""

    @pytest.fixture
    def real_typescript_project(self):
        """Create a realistic TypeScript project for end-to-end testing"""
        temp_dir = tempfile.mkdtemp()
        project_path = Path(temp_dir)
        
        # Package.json with TypeScript dependencies
        (project_path / "package.json").write_text("""
{
    "name": "e2e-test-project",
    "version": "1.0.0",
    "main": "src/index.ts",
    "scripts": {
        "build": "tsc",
        "test": "jest"
    },
    "dependencies": {
        "express": "^4.18.0",
        "lodash": "^4.17.0"
    },
    "devDependencies": {
        "typescript": "^5.0.0",
        "@types/node": "^18.0.0",
        "@types/express": "^4.17.0",
        "jest": "^29.0.0"
    }
}
""")

        # TypeScript config
        (project_path / "tsconfig.json").write_text("""
{
    "compilerOptions": {
        "target": "ES2020",
        "module": "commonjs",
        "lib": ["ES2020"],
        "outDir": "./dist",
        "rootDir": "./src",
        "strict": true,
        "esModuleInterop": true,
        "skipLibCheck": true,
        "forceConsistentCasingInFileNames": true,
        "declaration": true,
        "sourceMap": true
    },
    "include": ["src/**/*"],
    "exclude": ["node_modules", "dist", "**/*.test.ts"]
}
""")

        # Source directory structure
        src_dir = project_path / "src"
        src_dir.mkdir()

        # Main application entry point
        (src_dir / "index.ts").write_text("""
import express from 'express';
import { UserService } from './services/user-service';
import { DatabaseManager } from './database/database-manager';
import { Logger } from './utils/logger';
import { Config } from './config';

class Application {
    private app: express.Application;
    private userService: UserService;
    private database: DatabaseManager;
    private logger: Logger;
    private config: Config;

    constructor() {
        this.app = express();
        this.config = new Config();
        this.logger = new Logger(this.config.logLevel);
        this.database = new DatabaseManager(this.config.databaseUrl);
        this.userService = new UserService(this.database, this.logger);
        
        this.setupRoutes();
        this.setupMiddleware();
    }

    private setupRoutes(): void {
        this.app.get('/api/users', async (req, res) => {
            try {
                const users = await this.userService.getAllUsers();
                res.json(users);
            } catch (error) {
                this.logger.error('Failed to get users', error);
                res.status(500).json({ error: 'Internal server error' });
            }
        });

        this.app.get('/api/users/:id', async (req, res) => {
            try {
                const userId = parseInt(req.params.id);
                const user = await this.userService.getUserById(userId);
                
                if (!user) {
                    res.status(404).json({ error: 'User not found' });
                    return;
                }
                
                res.json(user);
            } catch (error) {
                this.logger.error('Failed to get user', error);
                res.status(500).json({ error: 'Internal server error' });
            }
        });

        this.app.post('/api/users', async (req, res) => {
            try {
                const userData = req.body;
                const newUser = await this.userService.createUser(userData);
                res.status(201).json(newUser);
            } catch (error) {
                this.logger.error('Failed to create user', error);
                res.status(500).json({ error: 'Internal server error' });
            }
        });
    }

    private setupMiddleware(): void {
        this.app.use(express.json());
        this.app.use(express.urlencoded({ extended: true }));
    }

    public async start(): Promise<void> {
        await this.database.connect();
        
        const port = this.config.port;
        this.app.listen(port, () => {
            this.logger.info(`Server started on port ${port}`);
        });
    }
}

// Bootstrap application
async function bootstrap() {
    const app = new Application();
    await app.start();
}

if (require.main === module) {
    bootstrap().catch(console.error);
}

export { Application };
""")

        # Services directory
        services_dir = src_dir / "services"
        services_dir.mkdir()

        (services_dir / "user-service.ts").write_text("""
import { DatabaseManager } from '../database/database-manager';
import { Logger } from '../utils/logger';
import { User, CreateUserRequest } from '../types/user';
import { ValidationError, NotFoundError } from '../errors/custom-errors';

export class UserService {
    constructor(
        private database: DatabaseManager,
        private logger: Logger
    ) {}

    async getAllUsers(): Promise<User[]> {
        this.logger.debug('Fetching all users');
        
        const query = 'SELECT * FROM users ORDER BY created_at DESC';
        const rows = await this.database.query(query);
        
        return rows.map(row => this.mapRowToUser(row));
    }

    async getUserById(id: number): Promise<User | null> {
        if (!id || id <= 0) {
            throw new ValidationError('Invalid user ID');
        }

        this.logger.debug(`Fetching user with ID: ${id}`);
        
        const query = 'SELECT * FROM users WHERE id = ?';
        const rows = await this.database.query(query, [id]);
        
        if (rows.length === 0) {
            return null;
        }
        
        return this.mapRowToUser(rows[0]);
    }

    async createUser(userData: CreateUserRequest): Promise<User> {
        this.validateUserData(userData);
        
        this.logger.debug('Creating new user', { email: userData.email });
        
        const query = `
            INSERT INTO users (email, name, age, created_at) 
            VALUES (?, ?, ?, datetime('now'))
        `;
        
        const result = await this.database.execute(query, [
            userData.email,
            userData.name,
            userData.age
        ]);
        
        const newUserId = result.lastInsertRowId;
        const createdUser = await this.getUserById(newUserId);
        
        if (!createdUser) {
            throw new Error('Failed to retrieve created user');
        }
        
        this.logger.info(`User created successfully`, { userId: newUserId });
        return createdUser;
    }

    private validateUserData(userData: CreateUserRequest): void {
        if (!userData.email || !userData.email.includes('@')) {
            throw new ValidationError('Invalid email address');
        }
        
        if (!userData.name || userData.name.trim().length === 0) {
            throw new ValidationError('Name is required');
        }
        
        if (userData.age !== undefined && userData.age < 0) {
            throw new ValidationError('Age must be positive');
        }
    }

    private mapRowToUser(row: any): User {
        return {
            id: row.id,
            email: row.email,
            name: row.name,
            age: row.age,
            createdAt: new Date(row.created_at)
        };
    }
}
""")

        # Database layer
        database_dir = src_dir / "database"
        database_dir.mkdir()

        (database_dir / "database-manager.ts").write_text("""
import { Logger } from '../utils/logger';

export interface QueryResult {
    lastInsertRowId: number;
    changes: number;
}

export class DatabaseManager {
    private isConnected: boolean = false;
    private logger: Logger = new Logger();

    constructor(private connectionString: string) {}

    async connect(): Promise<void> {
        this.logger.info('Connecting to database', { connectionString: this.connectionString });
        
        // Simulate connection logic
        await this.delay(100);
        this.isConnected = true;
        
        this.logger.info('Database connected successfully');
        await this.createTables();
    }

    async query(sql: string, params: any[] = []): Promise<any[]> {
        this.ensureConnected();
        
        this.logger.debug('Executing query', { sql, params });
        
        // Simulate query execution
        await this.delay(50);
        
        // Return mock data based on query
        if (sql.includes('SELECT * FROM users')) {
            return this.getMockUsers(params);
        }
        
        return [];
    }

    async execute(sql: string, params: any[] = []): Promise<QueryResult> {
        this.ensureConnected();
        
        this.logger.debug('Executing command', { sql, params });
        
        // Simulate execution
        await this.delay(30);
        
        return {
            lastInsertRowId: Math.floor(Math.random() * 1000) + 1,
            changes: 1
        };
    }

    private ensureConnected(): void {
        if (!this.isConnected) {
            throw new Error('Database not connected');
        }
    }

    private async createTables(): Promise<void> {
        const createUsersTable = `
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                age INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        `;
        
        await this.execute(createUsersTable);
    }

    private getMockUsers(params: any[]): any[] {
        const allUsers = [
            { id: 1, email: 'john@example.com', name: 'John Doe', age: 30, created_at: '2024-01-01 10:00:00' },
            { id: 2, email: 'jane@example.com', name: 'Jane Smith', age: 25, created_at: '2024-01-02 10:00:00' }
        ];
        
        if (params.length > 0) {
            // Filter by ID
            return allUsers.filter(user => user.id === params[0]);
        }
        
        return allUsers;
    }

    private delay(ms: number): Promise<void> {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}
""")

        # Types directory
        types_dir = src_dir / "types"
        types_dir.mkdir()

        (types_dir / "user.ts").write_text("""
export interface User {
    id: number;
    email: string;
    name: string;
    age?: number;
    createdAt: Date;
}

export interface CreateUserRequest {
    email: string;
    name: string;
    age?: number;
}

export interface UpdateUserRequest {
    email?: string;
    name?: string;
    age?: number;
}
""")

        # Utils directory
        utils_dir = src_dir / "utils"
        utils_dir.mkdir()

        (utils_dir / "logger.ts").write_text("""
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export class Logger {
    constructor(private level: LogLevel = 'info') {}

    debug(message: string, meta?: any): void {
        if (this.shouldLog('debug')) {
            this.log('DEBUG', message, meta);
        }
    }

    info(message: string, meta?: any): void {
        if (this.shouldLog('info')) {
            this.log('INFO', message, meta);
        }
    }

    warn(message: string, meta?: any): void {
        if (this.shouldLog('warn')) {
            this.log('WARN', message, meta);
        }
    }

    error(message: string, error?: any): void {
        if (this.shouldLog('error')) {
            this.log('ERROR', message, error);
        }
    }

    private shouldLog(level: LogLevel): boolean {
        const levels = ['debug', 'info', 'warn', 'error'];
        return levels.indexOf(level) >= levels.indexOf(this.level);
    }

    private log(level: string, message: string, meta?: any): void {
        const timestamp = new Date().toISOString();
        const logEntry = {
            timestamp,
            level,
            message,
            ...(meta && { meta })
        };
        
        console.log(JSON.stringify(logEntry));
    }
}
""")

        # Errors directory
        errors_dir = src_dir / "errors"
        errors_dir.mkdir()

        (errors_dir / "custom-errors.ts").write_text("""
export class ValidationError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'ValidationError';
    }
}

export class NotFoundError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'NotFoundError';
    }
}

export class DatabaseError extends Error {
    constructor(message: string, public cause?: Error) {
        super(message);
        this.name = 'DatabaseError';
    }
}
""")

        # Config file
        (src_dir / "config.ts").write_text("""
export class Config {
    public readonly port: number;
    public readonly databaseUrl: string;
    public readonly logLevel: 'debug' | 'info' | 'warn' | 'error';

    constructor() {
        this.port = parseInt(process.env.PORT || '3000');
        this.databaseUrl = process.env.DATABASE_URL || 'sqlite://./database.db';
        this.logLevel = (process.env.LOG_LEVEL as any) || 'info';
    }
}
""")

        # Test files
        tests_dir = src_dir / "__tests__"
        tests_dir.mkdir()

        (tests_dir / "user-service.test.ts").write_text("""
import { UserService } from '../services/user-service';
import { DatabaseManager } from '../database/database-manager';
import { Logger } from '../utils/logger';

describe('UserService', () => {
    let userService: UserService;
    let mockDatabase: jest.Mocked<DatabaseManager>;
    let mockLogger: jest.Mocked<Logger>;

    beforeEach(() => {
        mockDatabase = {
            query: jest.fn(),
            execute: jest.fn(),
            connect: jest.fn()
        } as any;

        mockLogger = {
            debug: jest.fn(),
            info: jest.fn(),
            warn: jest.fn(),
            error: jest.fn()
        } as any;

        userService = new UserService(mockDatabase, mockLogger);
    });

    test('should get all users', async () => {
        const mockUsers = [
            { id: 1, email: 'test@example.com', name: 'Test User', age: 30, created_at: '2024-01-01' }
        ];
        
        mockDatabase.query.mockResolvedValue(mockUsers);

        const result = await userService.getAllUsers();
        
        expect(result).toHaveLength(1);
        expect(result[0].email).toBe('test@example.com');
        expect(mockDatabase.query).toHaveBeenCalledWith('SELECT * FROM users ORDER BY created_at DESC');
    });

    test('should throw validation error for invalid user data', async () => {
        const invalidUserData = { email: 'invalid', name: '' };

        await expect(userService.createUser(invalidUserData)).rejects.toThrow('Invalid email address');
    });
});
""")

        yield project_path
        shutil.rmtree(temp_dir)

    def test_full_semantic_pipeline_mock(self, real_typescript_project):
        """Test the full semantic analysis pipeline with mocked SCIP"""
        
        with patch('subprocess.run') as mock_run:
            # Mock successful scan
            mock_run.return_value = MagicMock(returncode=0)
            
            # Test semantic analysis enabled
            graph = CodeGraph(str(real_typescript_project), semantic=True)
            
            # Verify scan command was called with semantic flag
            calls = mock_run.call_args_list
            semantic_calls = [call for call in calls if '--semantic' in str(call)]
            assert len(semantic_calls) > 0, "Should call scan with --semantic flag"
            
            # Verify database would be accessible
            db_path = real_typescript_project / ".reviewbot" / "graph.db"
            assert db_path.parent.exists(), "Should create .reviewbot directory"

    def test_api_integration_with_realistic_data(self, real_typescript_project):
        """Test API integration with realistic semantic data"""
        
        # Create realistic database with semantic relationships
        db_path = real_typescript_project / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(exist_ok=True)
        
        conn = sqlite3.connect(db_path)
        
        # Create schema
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
        """)
        
        # Insert realistic data based on the TypeScript project
        files_data = [
            (1, 'src/index.ts', 'abc123'),
            (2, 'src/services/user-service.ts', 'def456'),
            (3, 'src/database/database-manager.ts', 'ghi789'),
            (4, 'src/utils/logger.ts', 'jkl012'),
            (5, 'src/config.ts', 'mno345')
        ]
        
        for file_data in files_data:
            conn.execute("INSERT INTO files VALUES (?, ?, ?)", file_data)
        
        # Insert symbols
        symbols_data = [
            ('app.Application', 'TypeScript', 'class', 'Application', 'src.index.Application', None, 1, 9, 0),
            ('user-service.UserService', 'TypeScript', 'class', 'UserService', 'src.services.UserService', None, 2, 6, 0),
            ('database.DatabaseManager', 'TypeScript', 'class', 'DatabaseManager', 'src.database.DatabaseManager', None, 3, 8, 0),
            ('logger.Logger', 'TypeScript', 'class', 'Logger', 'src.utils.Logger', None, 4, 3, 0),
            ('config.Config', 'TypeScript', 'class', 'Config', 'src.Config', None, 5, 1, 0)
        ]
        
        for symbol_data in symbols_data:
            conn.execute("INSERT INTO symbols VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)", symbol_data)
        
        # Insert semantic edges (imports and calls)
        edges_data = [
            ('imports', 'app.Application', 'user-service.UserService', 1, 2, 'semantic'),
            ('imports', 'app.Application', 'database.DatabaseManager', 1, 3, 'semantic'),
            ('imports', 'app.Application', 'logger.Logger', 1, 4, 'semantic'),
            ('imports', 'app.Application', 'config.Config', 1, 5, 'semantic'),
            ('imports', 'user-service.UserService', 'database.DatabaseManager', 2, 3, 'semantic'),
            ('imports', 'user-service.UserService', 'logger.Logger', 2, 4, 'semantic'),
            ('calls', 'app.Application.setupRoutes', 'user-service.UserService.getAllUsers', 1, 2, 'semantic'),
            ('calls', 'app.Application.setupRoutes', 'user-service.UserService.getUserById', 1, 2, 'semantic'),
            ('calls', 'app.Application.setupRoutes', 'user-service.UserService.createUser', 1, 2, 'semantic')
        ]
        
        for edge_data in edges_data:
            conn.execute("INSERT INTO edges VALUES (?, ?, ?, ?, ?, ?)", edge_data)
        
        conn.commit()
        conn.close()
        
        # Test API queries
        api = CodeGraphAPI(str(real_typescript_project))
        
        # Test symbol lookup
        app_symbol = api.get_symbol("src.index.Application")
        assert app_symbol is not None
        assert app_symbol.name == "Application"
        assert app_symbol.kind == "class"
        
        user_service_symbol = api.get_symbol("src.services.UserService")
        assert user_service_symbol is not None
        assert user_service_symbol.name == "UserService"

    def test_helpers_with_realistic_project(self, real_typescript_project):
        """Test AgentHelpers with realistic project structure"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            # Create minimal database for helpers to work
            db_path = real_typescript_project / ".reviewbot" / "graph.db"
            db_path.parent.mkdir(exist_ok=True)
            
            conn = sqlite3.connect(db_path)
            conn.execute("CREATE TABLE test (id INTEGER)")
            conn.close()
            
            # Test helpers initialization
            helpers = AgentHelpers(str(real_typescript_project), semantic=True)
            
            # Should initialize without errors
            assert helpers.graph is not None
            assert helpers.graph.semantic == True

    def test_concurrent_api_access_realistic(self, real_typescript_project):
        """Test concurrent API access with realistic project"""
        
        # Create database with realistic data
        db_path = real_typescript_project / ".reviewbot" / "graph.db"
        db_path.parent.mkdir(exist_ok=True)
        
        conn = sqlite3.connect(db_path)
        conn.executescript("""
            CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT, sha TEXT);
            CREATE TABLE symbols (id TEXT PRIMARY KEY, lang TEXT, kind TEXT, name TEXT, fqn TEXT, signature TEXT, file_id INTEGER, line INTEGER, col INTEGER);
            INSERT INTO files VALUES (1, 'src/index.ts', 'abc123');
            INSERT INTO symbols VALUES ('test.symbol', 'TypeScript', 'class', 'TestClass', 'test.TestClass', NULL, 1, 1, 0);
        """)
        conn.commit()
        conn.close()
        
        import threading
        results = []
        errors = []
        
        def worker():
            try:
                api = CodeGraphAPI(str(real_typescript_project), check_same_thread=False)
                symbol = api.get_symbol("test.TestClass")
                results.append(symbol is not None)
            except Exception as e:
                errors.append(e)
        
        # Start multiple workers
        threads = []
        for i in range(5):
            thread = threading.Thread(target=worker)
            threads.append(thread)
            thread.start()
        
        # Wait for completion
        for thread in threads:
            thread.join(timeout=10)
        
        # All should succeed
        assert len(errors) == 0, f"Concurrent access errors: {errors}"
        assert len(results) == 5
        assert all(results), "All workers should find the symbol"

    def test_project_structure_analysis(self, real_typescript_project):
        """Test analyzing project structure and dependencies"""
        
        with patch('subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            
            graph = CodeGraph(str(real_typescript_project), semantic=True)
            
            # Verify project structure was analyzed
            calls = mock_run.call_args_list
            
            # Should have at least one call with the project path
            project_calls = [call for call in calls if str(real_typescript_project) in str(call)]
            assert len(project_calls) > 0, "Should analyze the project path"
            
            # Should use semantic analysis
            semantic_calls = [call for call in calls if '--semantic' in str(call)]
            assert len(semantic_calls) > 0, "Should use semantic analysis"

if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])