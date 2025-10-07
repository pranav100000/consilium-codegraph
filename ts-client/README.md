# Consilium CodeGraph TypeScript Client

TypeScript client for querying code graphs with semantic enrichment. This package provides programmatic access to the Consilium CodeGraph database, allowing you to analyze codebases, traverse call graphs, and understand code dependencies.

## Features

- 📦 **Index from TypeScript**: Scan repositories without manual CLI commands
- 🔍 **Symbol Queries**: Find and inspect functions, classes, methods, and more
- 🕸️ **Graph Traversal**: Navigate call graphs, dependencies, and relationships
- 📊 **Statistics**: Get insights into codebase structure and complexity
- 🔄 **Cycle Detection**: Find circular dependencies automatically
- 💾 **SQLite Backend**: Direct database access for maximum performance
- 🎯 **Type Safety**: Full TypeScript definitions for all APIs
- 🤖 **Agent-Focused API**: Designed to complement Read/Grep/Glob tools for AI coding agents
- 🗺️ **Codebuff Integration**: Drop-in replacement for tree-sitter with 80% token savings
- ⚡ **Bun & Node.js Support**: Works with both runtimes - uses Bun's native SQLite for 3x faster performance

## Installation

### From GitHub (Recommended)

**Includes pre-built binaries - no Rust toolchain needed!**

```bash
npm install github:yourusername/consilium-codegraph#main
# or
yarn add github:yourusername/consilium-codegraph#main
```

The package includes pre-built binaries for macOS (ARM/Intel), Linux, and Windows. See [GITHUB_INSTALLATION.md](../GITHUB_INSTALLATION.md) for details.

**Works with Bun!** If you're using Bun (like Codebuff), no native compilation needed - it uses Bun's built-in SQLite. See [BUN_SUPPORT.md](./BUN_SUPPORT.md).

### From npm (Coming Soon)

```bash
npm install @consilium/codegraph-client
```

**Note**: For Node.js, this package optionally uses `better-sqlite3` (native module). If you're using Bun, it uses the built-in `bun:sqlite` instead - no compilation needed!

## Quick Start

### Agent API (For AI Coding Agents) 🤖

**Recommended for AI agents** - Complete TypeScript API for both indexing and querying:

```typescript
import { scanRepositorySync, isScanned, AgentCodeGraph } from "@consilium/codegraph-client";

// 1. Index the repository (if not already done)
if (!isScanned("/path/to/your/repo")) {
  const result = scanRepositorySync("/path/to/your/repo", {
    semantic: true  // Include type information
  });

  console.log(`Indexed in ${result.duration}ms`);
}

// 2. Query the code graph
const agent = new AgentCodeGraph("/path/to/your/repo");

// Find symbols (better than grep for structure)
const symbols = agent.findSymbols("auth", { kind: ["Function", "Method"] });

// Get symbol with relationships
const info = agent.getSymbol(symbols[0].fqn, {
  includeCallers: true,
  includeCallees: true
});

console.log(`Calls: ${info.callees?.length || 0} functions`);
console.log(`Called by: ${info.callers?.length || 0} functions`);

// Navigate the graph (deep traversal)
const deps = agent.queryRelationships(symbols[0].fqn, "calls", { depth: 3 });

agent.close();
```

**Key Features:**
- 📦 **Index from TypeScript**: `scanRepositorySync()` - no CLI needed
- 🔍 **Find symbols**: `findSymbols()` - structured search (better than grep)
- 🔗 **Get relationships**: `getSymbol()` - understand dependencies
- 🕸️ **Navigate graph**: `queryRelationships()` - deep traversal
- 🎯 **Codebuff Integration**: `getFileTokenData()` - drop-in replacement for tree-sitter

📚 **See [AGENT_API.md](./AGENT_API.md) for complete documentation**
📚 **See [CODEBUFF_INTEGRATION.md](../CODEBUFF_INTEGRATION.md) for Codebuff integration**

### Simple API (Recommended for most use cases)

```typescript
import { CodeGraphAPI } from "@consilium/codegraph-client";

// Initialize the API
const api = new CodeGraphAPI("/path/to/your/repo");

// Find symbols
const symbols = api.findSymbols("getUserData");
console.log(`Found ${symbols.length} symbols`);

// Get symbol details
const symbol = api.getSymbol("UserService.authenticate");
if (symbol) {
  console.log(`Symbol: ${symbol.name}`);
  console.log(`Location: ${symbol.location.file}:${symbol.location.line}`);
}

// Analyze relationships
const callers = api.getCallers("UserService.authenticate");
console.log(`Called by: ${callers.join(", ")}`);

const callees = api.getCallees("UserService.authenticate");
console.log(`Calls: ${callees.join(", ")}`);

// Get impact radius
const impacted = api.getImpactRadius("DataProcessor.process");
console.log(`Would affect ${impacted.size} symbols`);

// Get statistics
const stats = api.getStats();
console.log(`Total symbols: ${stats.totalSymbols}`);
console.log(`Total edges: ${stats.totalEdges}`);

// Close connection when done
api.close();
```

### Full-Featured API

```typescript
import { CodeGraph, SymbolKind } from "@consilium/codegraph-client";

// Initialize with semantic analysis support
const graph = new CodeGraph("/path/to/your/repo", undefined, true);

// Search for specific symbol types
const classes = graph.findSymbols("User", SymbolKind.CLASS);
const functions = graph.findSymbols("process", SymbolKind.FUNCTION);

// Get all symbols in a file
const fileSymbols = graph.getFileSymbols("src/services/user.ts");

// Traverse call chains
const callers = graph.getCallers("processData", 3); // 3 levels deep
for (const callPath of callers) {
  console.log(`Call path (depth ${callPath.depth}):`);
  for (const sym of callPath.path) {
    console.log(`  -> ${sym.fqn} (${sym.location.file}:${sym.location.line})`);
  }
}

// Find paths between symbols
const paths = graph.findPath("main", "DatabaseConnection.query", 10);
console.log(`Found ${paths.length} execution paths`);

// Get dependency graph
const deps = graph.getDependencies("DataService");
console.log(`Direct dependencies: ${deps.dependencies[deps.root.fqn].length}`);
console.log(`Direct dependents: ${deps.dependents[deps.root.fqn].length}`);
if (deps.cycles.length > 0) {
  console.log(`⚠️  Found ${deps.cycles.length} circular dependencies`);
}

// Get statistics
const stats = graph.getStatistics();
console.log(JSON.stringify(stats, null, 2));

// Close connection
graph.close();
```

### Convenience Functions

```typescript
import { analyzeCodebase, findRelatedCode } from "@consilium/codegraph-client";

// Quick codebase analysis
const analysis = analyzeCodebase("/path/to/your/repo");
console.log(`Files: ${analysis.stats.totalFiles}`);
console.log(`Symbols: ${analysis.stats.totalSymbols}`);
console.log(`Cycles found: ${analysis.cycles.length}`);
console.log(`Entry points: ${analysis.entryPoints.length}`);
console.log(`Complex functions: ${analysis.complexFunctions.length}`);

// Find all code related to a symbol
const related = findRelatedCode("/path/to/your/repo", "UserService.login");
console.log(`Callers: ${related.callers.length}`);
console.log(`Callees: ${related.callees.length}`);
console.log(`Impact radius: ${related.impact.length} symbols`);
```

## API Reference

### CodeGraphAPI

The simplified API for common operations.

#### Constructor

```typescript
constructor(repoPath: string, dbPath?: string, semantic?: boolean)
```

- `repoPath`: Path to the repository root
- `dbPath`: (Optional) Path to the graph database (default: `.reviewbot/graph.db`)
- `semantic`: (Optional) Whether to recommend semantic analysis (default: `true`)

#### Methods

- `getSymbol(fqn: string): Symbol | null` - Get a symbol by its fully qualified name
- `findSymbols(pattern: string, kind?: string): Symbol[]` - Search for symbols by name pattern
- `getFileSymbols(filePath: string): Symbol[]` - Get all symbols in a file
- `getCallers(symbol: string): string[]` - Get all functions that call this symbol
- `getCallees(symbol: string): string[]` - Get all functions called by this symbol
- `getEdges(source?: string, target?: string, edgeType?: string): Edge[]` - Get edges with filters
- `findPaths(start: string, end: string, maxDepth?: number): string[][]` - Find paths between symbols
- `getDependencies(symbol: string): Record<string, string[]>` - Get all dependencies of a symbol
- `getImpactRadius(symbol: string, maxDepth?: number): Set<string>` - Get impact radius of changes
- `getStats(): GraphStats` - Get overall graph statistics
- `findCycles(): string[][]` - Find all cycles in the call graph
- `close(): void` - Close the database connection

### Scanner API

Index repositories from TypeScript.

#### Functions

```typescript
// Synchronous scan
scanRepositorySync(repoPath: string, options?: ScanOptions): ScanResult

// Async scan
scanRepository(repoPath: string, options?: ScanOptions): Promise<ScanResult>

// Check if scanned
isScanned(repoPath: string): boolean

// Get CLI info
getCLIInfo(): { path: string; version?: string } | null
```

**Options:**
- `semantic?: boolean` - Include type information (default: false)
- `quiet?: boolean` - Suppress output (default: false)

**Example:**
```typescript
import { scanRepositorySync, isScanned } from "@consilium/codegraph-client";

if (!isScanned("./my-project")) {
  const result = scanRepositorySync("./my-project", { semantic: true });
  if (result.success) {
    console.log(`Scanned in ${result.duration}ms`);
  }
}
```

### CodeGraph

The full-featured API with advanced capabilities.

#### Constructor

```typescript
constructor(repoPath: string, dbPath?: string, semantic?: boolean)
```

#### Methods

- `getSymbol(fqn: string): Symbol | null`
- `findSymbols(pattern: string, kind?: SymbolKind, limit?: number): Symbol[]`
- `getFileSymbols(filepath: string): Symbol[]`
- `getCallers(symbol: string, maxDepth?: number): CallPath[]`
- `getCallees(symbol: string, maxDepth?: number): CallPath[]`
- `getDependencies(symbol: string): DependencyGraph`
- `findPath(fromSymbol: string, toSymbol: string, maxDepth?: number): Symbol[][]`
- `getStatistics(): GraphStats`
- `refreshCache(): void` - Clear all caches
- `close(): void`

## Types

All TypeScript types are fully documented with JSDoc comments. Key types include:

- `Symbol` - A code symbol (function, class, method, etc.)
- `Edge` - A relationship between symbols
- `Location` - Source code location
- `CallPath` - A path through function calls
- `DependencyGraph` - Dependency relationships with cycle detection
- `GraphStats` - Statistics about the code graph

See the source code for complete type definitions.

## Development

### Building

```bash
npm run build
```

### Testing

```bash
# Run all tests
npm test

# Run tests with coverage
npm run test:coverage

# Run standalone examples/demos
npx ts-node test-agent-api.ts
npx ts-node test-codebuff-integration.ts
```

**Test Structure:**
- `tests/*.test.ts` - Formal unit tests using vitest framework
- `test-*.ts` - Standalone example/demo scripts for manual testing

### Linting

```bash
npm run lint
```

## Comparison with Python API

This TypeScript client provides feature parity with the Python API (`agent_api/`). The main differences:

- **Database Access**: Uses `better-sqlite3` instead of Python's `sqlite3`
- **Async/Await**: All operations are synchronous (like the Python version)
- **Type Safety**: Full TypeScript types vs Python type hints
- **Performance**: Similar performance, both use direct SQLite access

### Migration from Python

```python
# Python
from agent_api.code_graph import CodeGraph
graph = CodeGraph("/path/to/repo")
symbol = graph.get_symbol("MyClass.method")
```

```typescript
// TypeScript
import { CodeGraph } from "@consilium/codegraph-client";
const graph = new CodeGraph("/path/to/repo");
const symbol = graph.getSymbol("MyClass.method");
```

Method names follow JavaScript conventions (camelCase instead of snake_case).

## License

MIT

## Contributing

Contributions are welcome! Please see the main [Consilium CodeGraph repository](https://github.com/yourusername/consilium-codegraph) for contribution guidelines.
