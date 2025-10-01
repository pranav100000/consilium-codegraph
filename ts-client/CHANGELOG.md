# Changelog

All notable changes to the Consilium CodeGraph TypeScript Client will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-09-30

### Added
- Initial release of TypeScript client for Consilium CodeGraph
- `CodeGraphAPI` class - simplified API for common operations
- `CodeGraph` class - full-featured API with advanced capabilities
- Complete type definitions for all models and APIs
- Support for symbol queries (getSymbol, findSymbols, getFileSymbols)
- Support for relationship queries (getCallers, getCallees, getEdges)
- Support for analysis queries (findPaths, getImpactRadius, getDependencies)
- Graph statistics and cycle detection
- Convenience functions: `analyzeCodebase()` and `findRelatedCode()`
- Comprehensive test suite with Vitest
- Example usage files
- Complete API documentation in README.md
- Migration guide from Python API
- ESLint and Prettier configuration
- TypeScript declarations for all exports

### Features
- 100% feature parity with Python API (`agent_api/`)
- Direct SQLite database access using `better-sqlite3`
- Synchronous operations for predictable performance
- In-memory caching for frequently accessed symbols
- WAL mode support for better concurrency
- Full TypeScript type safety
- Support for all symbol kinds (function, class, method, etc.)
- Support for all edge types (calls, imports, uses, etc.)
- Graph traversal with configurable depth
- Cycle detection in call graphs
- Impact radius analysis
- Path finding between symbols
- Dependency graph extraction

### Documentation
- Comprehensive README with examples
- API reference documentation
- Migration guide for Python users
- Working code examples
- Test suite demonstrating usage patterns
