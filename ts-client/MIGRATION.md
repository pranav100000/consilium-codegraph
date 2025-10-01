# Migration Guide: Python to TypeScript

This guide helps you migrate from the Python API (`agent_api/`) to the TypeScript client.

## Overview

The TypeScript client provides 100% feature parity with the Python API while offering better type safety and native JavaScript/TypeScript integration.

## Installation

### Python (Old)
```bash
# Python dependencies were in requirements.txt
pip install -r requirements.txt
```

### TypeScript (New)
```bash
npm install @consilium/codegraph-client
# or
yarn add @consilium/codegraph-client
```

## API Comparison

### Initialization

#### Python
```python
from agent_api.simple_api import CodeGraphAPI
from agent_api.code_graph import CodeGraph

# Simple API
api = CodeGraphAPI("/path/to/repo")

# Full API
graph = CodeGraph("/path/to/repo", semantic=True)
```

#### TypeScript
```typescript
import { CodeGraphAPI, CodeGraph } from "@consilium/codegraph-client";

// Simple API
const api = new CodeGraphAPI("/path/to/repo");

// Full API
const graph = new CodeGraph("/path/to/repo", undefined, true);
```

### Symbol Queries

#### Python
```python
# Get symbol by FQN
symbol = api.get_symbol("UserService.authenticate")

# Find symbols by pattern
symbols = api.find_symbols("User", kind="class")

# Get file symbols
file_symbols = api.get_file_symbols("src/user.py")
```

#### TypeScript
```typescript
// Get symbol by FQN
const symbol = api.getSymbol("UserService.authenticate");

// Find symbols by pattern
const symbols = api.findSymbols("User", "class");

// Get file symbols
const fileSymbols = api.getFileSymbols("src/user.ts");
```

### Relationship Queries

#### Python
```python
# Get callers
callers = api.get_callers("processData")

# Get callees
callees = api.get_callees("processData")

# Get edges
edges = api.get_edges(source="MyClass.method")

# Get dependencies
deps = api.get_dependencies("DataService")
```

#### TypeScript
```typescript
// Get callers
const callers = api.getCallers("processData");

// Get callees
const callees = api.getCallees("processData");

// Get edges
const edges = api.getEdges("MyClass.method");

// Get dependencies
const deps = api.getDependencies("DataService");
```

### Analysis Queries

#### Python
```python
# Find paths
paths = api.find_paths("main", "query", max_depth=5)

# Impact radius
impact = api.get_impact_radius("DataProcessor", max_depth=3)

# Statistics
stats = api.get_stats()

# Find cycles
cycles = api.find_cycles()
```

#### TypeScript
```typescript
// Find paths
const paths = api.findPaths("main", "query", 5);

// Impact radius
const impact = api.getImpactRadius("DataProcessor", 3);

// Statistics
const stats = api.getStats();

// Find cycles
const cycles = api.findCycles();
```

### Convenience Functions

#### Python
```python
from agent_api.simple_api import analyze_codebase, find_related_code

# Analyze codebase
analysis = analyze_codebase("/path/to/repo")
print(f"Files: {analysis['stats']['total_files']}")

# Find related code
related = find_related_code("/path/to/repo", "UserService.login")
print(f"Callers: {len(related['callers'])}")
```

#### TypeScript
```typescript
import { analyzeCodebase, findRelatedCode } from "@consilium/codegraph-client";

// Analyze codebase
const analysis = analyzeCodebase("/path/to/repo");
console.log(`Files: ${analysis.stats.totalFiles}`);

// Find related code
const related = findRelatedCode("/path/to/repo", "UserService.login");
console.log(`Callers: ${related.callers.length}`);
```

### Resource Management

#### Python
```python
# Context manager (recommended)
with CodeGraphAPI("/path/to/repo") as api:
    symbols = api.find_symbols("User")
    # api.close() called automatically

# Manual cleanup
api = CodeGraphAPI("/path/to/repo")
try:
    symbols = api.find_symbols("User")
finally:
    api.close()
```

#### TypeScript
```typescript
// Manual cleanup (required)
const api = new CodeGraphAPI("/path/to/repo");
try {
  const symbols = api.findSymbols("User");
  // ... use api
} finally {
  api.close();
}

// Or without try-finally if no errors expected
const api = new CodeGraphAPI("/path/to/repo");
const symbols = api.findSymbols("User");
api.close();
```

## Naming Conventions

Python uses `snake_case` while TypeScript uses `camelCase`:

| Python | TypeScript |
|--------|------------|
| `get_symbol()` | `getSymbol()` |
| `find_symbols()` | `findSymbols()` |
| `get_file_symbols()` | `getFileSymbols()` |
| `get_callers()` | `getCallers()` |
| `get_callees()` | `getCallees()` |
| `get_edges()` | `getEdges()` |
| `find_paths()` | `findPaths()` |
| `get_dependencies()` | `getDependencies()` |
| `get_impact_radius()` | `getImpactRadius()` |
| `get_stats()` | `getStats()` |
| `find_cycles()` | `findCycles()` |
| `refresh_cache()` | `refreshCache()` |

## Property Name Changes

Object properties also follow camelCase:

| Python | TypeScript |
|--------|------------|
| `symbol.file_path` | `symbol.location.file` |
| `edge.edge_type` | `edge.edgeType` |
| `stats.total_files` | `stats.totalFiles` |
| `stats.total_symbols` | `stats.totalSymbols` |
| `stats.total_edges` | `stats.totalEdges` |
| `stats.symbols_by_kind` | `stats.symbolsByKind` |
| `stats.edges_by_type` | `stats.edgesByType` |
| `call_path.is_recursive` | `callPath.isRecursive` |

## Type Differences

### Return Types

#### Python
```python
# Returns list
symbols: List[Symbol] = api.find_symbols("User")

# Returns set
impact: Set[str] = api.get_impact_radius("func")

# Returns dict
stats: Dict[str, Any] = api.get_stats()
```

#### TypeScript
```typescript
// Returns array
const symbols: Symbol[] = api.findSymbols("User");

// Returns Set (JavaScript Set)
const impact: Set<string> = api.getImpactRadius("func");

// Returns object with types
const stats: GraphStats = api.getStats();
```

### Optional Values

#### Python
```python
symbol: Optional[Symbol] = api.get_symbol("MyClass")
if symbol is not None:
    print(symbol.name)
```

#### TypeScript
```typescript
const symbol: Symbol | null = api.getSymbol("MyClass");
if (symbol !== null) {
  console.log(symbol.name);
}

// Or with optional chaining
console.log(api.getSymbol("MyClass")?.name);
```

## Error Handling

#### Python
```python
try:
    api = CodeGraphAPI("/path/to/repo")
    symbols = api.find_symbols("User")
except FileNotFoundError as e:
    print(f"Database not found: {e}")
except Exception as e:
    print(f"Error: {e}")
finally:
    if api:
        api.close()
```

#### TypeScript
```typescript
try {
  const api = new CodeGraphAPI("/path/to/repo");
  const symbols = api.findSymbols("User");
  api.close();
} catch (error) {
  if (error instanceof Error) {
    console.error(`Error: ${error.message}`);
  }
}
```

## Complete Example Migration

### Python Original
```python
from agent_api.simple_api import CodeGraphAPI

def analyze_function(repo_path: str, function_name: str):
    with CodeGraphAPI(repo_path) as api:
        # Find the function
        functions = api.find_symbols(function_name, kind="function")
        if not functions:
            print(f"Function {function_name} not found")
            return

        func = functions[0]
        print(f"Analyzing: {func.fqn}")

        # Get callers and callees
        callers = api.get_callers(func.fqn)
        callees = api.get_callees(func.fqn)

        print(f"Called by: {len(callers)} functions")
        print(f"Calls: {len(callees)} functions")

        # Impact analysis
        impact = api.get_impact_radius(func.fqn, max_depth=2)
        print(f"Impact radius: {len(impact)} symbols")

analyze_function("./my-repo", "processData")
```

### TypeScript Migrated
```typescript
import { CodeGraphAPI } from "@consilium/codegraph-client";

function analyzeFunction(repoPath: string, functionName: string): void {
  const api = new CodeGraphAPI(repoPath);

  try {
    // Find the function
    const functions = api.findSymbols(functionName, "function");
    if (functions.length === 0) {
      console.log(`Function ${functionName} not found`);
      return;
    }

    const func = functions[0];
    console.log(`Analyzing: ${func.fqn}`);

    // Get callers and callees
    const callers = api.getCallers(func.fqn);
    const callees = api.getCallees(func.fqn);

    console.log(`Called by: ${callers.length} functions`);
    console.log(`Calls: ${callees.length} functions`);

    // Impact analysis
    const impact = api.getImpactRadius(func.fqn, 2);
    console.log(`Impact radius: ${impact.size} symbols`);
  } finally {
    api.close();
  }
}

analyzeFunction("./my-repo", "processData");
```

## Key Differences Summary

1. **Import style**: Python uses `from/import`, TypeScript uses `import { } from`
2. **Naming**: Python uses `snake_case`, TypeScript uses `camelCase`
3. **Type hints**: Python uses type comments, TypeScript has built-in types
4. **Context managers**: Python has `with` statement, TypeScript uses try/finally
5. **None vs null**: Python uses `None`, TypeScript uses `null` or `undefined`
6. **Collections**: Python `set` → TypeScript `Set`, Python `dict` → TypeScript object/`Map`

## Testing

### Python (pytest)
```python
def test_get_symbol():
    api = CodeGraphAPI("./test_repo")
    symbol = api.get_symbol("MyClass.method")
    assert symbol is not None
    api.close()
```

### TypeScript (vitest)
```typescript
import { describe, it, expect } from "vitest";
import { CodeGraphAPI } from "../src";

describe("CodeGraphAPI", () => {
  it("should get symbol", () => {
    const api = new CodeGraphAPI("./test_repo");
    const symbol = api.getSymbol("MyClass.method");
    expect(symbol).not.toBeNull();
    api.close();
  });
});
```

## Performance Considerations

Both Python and TypeScript clients have similar performance characteristics:

- Direct SQLite database access (no HTTP overhead)
- Synchronous operations
- In-memory caching for frequently accessed symbols
- Batch operations for improved throughput

The TypeScript client uses `better-sqlite3`, which is as fast or faster than Python's `sqlite3` module for most operations.

## Need Help?

- Check the [README.md](./README.md) for detailed API documentation
- See [examples/basic-usage.ts](./examples/basic-usage.ts) for working examples
- Review the test files in `tests/` for usage patterns
