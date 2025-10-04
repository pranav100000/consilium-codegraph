# TypeScript Client - Complete Summary

## Overview

The TypeScript client provides a complete API for AI coding agents to both **index** and **query** code graphs - all from TypeScript, no manual CLI commands needed.

## Installation

```bash
# In your project
npm install /path/to/consilium-codegraph/ts-client

# Or use npm link for development
cd /path/to/consilium-codegraph/ts-client && npm link
cd /path/to/your/project && npm link @consilium/codegraph-client
```

## Complete API (4 Functions)

### 1. Index - `scanRepositorySync()`

Index a repository to create the code graph database.

```typescript
import { scanRepositorySync } from "@consilium/codegraph-client";

const result = scanRepositorySync("./my-project", {
  semantic: true,  // Include type info (recommended)
  quiet: false     // Show progress
});

if (result.success) {
  console.log(`Indexed in ${result.duration}ms`);
}
```

### 2. Check - `isScanned()`

Check if a repository has been indexed.

```typescript
import { isScanned } from "@consilium/codegraph-client";

if (!isScanned("./my-project")) {
  // Need to scan first
}
```

### 3. Find - `findSymbols()`

Search for code entities by name.

```typescript
const agent = new AgentCodeGraph("./my-project");

// Find all functions with "auth" in name
const functions = agent.findSymbols("auth", {
  kind: ["Function", "Method"],
  limit: 50
});
```

### 4. Get - `getSymbol()`

Get detailed information about a symbol including relationships.

```typescript
const info = agent.getSymbol("authenticateUser", {
  includeCallers: true,    // Who calls this
  includeCallees: true,    // What this calls
  includeReferences: true  // All usages
});

console.log(`Called by ${info.callers?.length || 0} functions`);
```

### 5. Navigate - `queryRelationships()`

Traverse the code graph deeply.

```typescript
// Find all dependencies (depth=3)
const deps = agent.queryRelationships("processPayment", "calls", {
  depth: 3,
  limit: 100
});

// Find who calls this (impact analysis)
const callers = agent.queryRelationships("deleteUser", "called-by", {
  depth: 2
});
```

## Complete Example

```typescript
import {
  scanRepositorySync,
  isScanned,
  AgentCodeGraph
} from "@consilium/codegraph-client";

// 1. Index if needed
if (!isScanned("./my-project")) {
  console.log("Indexing...");
  const result = scanRepositorySync("./my-project", { semantic: true });

  if (!result.success) {
    throw new Error(`Failed: ${result.error}`);
  }

  console.log(`✅ Indexed in ${result.duration}ms`);
}

// 2. Query
const agent = new AgentCodeGraph("./my-project");

// Find entry points
const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 100 });

const entryPoints = allFunctions.filter(fn => {
  const info = agent.getSymbol(fn.fqn, { includeCallers: true });
  return !info.callers || info.callers.length === 0;
});

console.log(`Entry points: ${entryPoints.length}`);

// Analyze most-called functions
const withCallers = allFunctions
  .map(fn => {
    const info = agent.getSymbol(fn.fqn, { includeCallers: true });
    return { name: fn.name, callers: info.callers?.length || 0 };
  })
  .sort((a, b) => b.callers - a.callers);

console.log("\nMost-called functions:");
withCallers.slice(0, 5).forEach(fn => {
  console.log(`  ${fn.name}: ${fn.callers} callers`);
});

agent.close();
```

## Prerequisites

Build the Rust CLI once:

```bash
cd /path/to/consilium-codegraph
cargo build --release
```

The TypeScript scanner will automatically find and use it.

## Documentation

| Document | Purpose |
|----------|---------|
| **[COMPLETE_EXAMPLE.md](COMPLETE_EXAMPLE.md)** | Full workflow examples |
| **[ts-client/README.md](ts-client/README.md)** | Complete API reference |
| **[ts-client/AGENT_API.md](ts-client/AGENT_API.md)** | Agent API guide |
| **[ts-client/AGENT_TOOLS_REFERENCE.md](ts-client/AGENT_TOOLS_REFERENCE.md)** | In-depth tool docs |
| **[ts-client/AGENT_API_QUICK_REF.md](ts-client/AGENT_API_QUICK_REF.md)** | Quick reference |
| **[ts-client/agent-example.ts](ts-client/agent-example.ts)** | Working examples |

## Key Features

✅ **Index from TypeScript** - No manual CLI commands
✅ **3 Core Methods** - `findSymbols()`, `getSymbol()`, `queryRelationships()`
✅ **Complements Read/Grep/Glob** - Designed for AI agents
✅ **Deep Graph Traversal** - Navigate call graphs with depth control
✅ **Type-Safe** - Full TypeScript definitions
✅ **Fast** - Direct SQLite queries, Rust indexing

## Use Cases

### Find Dead Code
```typescript
const allSymbols = agent.findSymbols("", { limit: 500 });

const unused = allSymbols.filter(sym => {
  const info = agent.getSymbol(sym.fqn, {
    includeCallers: true,
    includeReferences: true
  });

  return (
    (!info.callers || info.callers.length === 0) &&
    (!info.references || info.references.length <= 1)
  );
});

console.log(`Found ${unused.length} potentially unused symbols`);
```

### Impact Analysis
```typescript
const impacted = agent.queryRelationships("deleteUser", "called-by", {
  depth: 2
});

console.log(`Changing deleteUser would affect ${impacted.length} symbols`);
```

### Security Audit
```typescript
const authFunctions = agent.findSymbols("auth", { kind: ["Function", "Method"] });
const handlers = agent.findSymbols("handle", { kind: ["Function", "Method"] });

const unprotected = handlers.filter(handler => {
  const info = agent.getSymbol(handler.fqn, { includeCallees: true });
  const callsAuth = info.callees?.some(callee =>
    authFunctions.some(auth => callee.includes(auth.name))
  );
  return !callsAuth;
});

console.log(`⚠️ ${unprotected.length} potentially unprotected routes`);
```

## Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Index 100k LOC | ~30s | One-time per codebase |
| `findSymbols()` | ~5-20ms | Fast, indexed |
| `getSymbol()` basic | ~1-5ms | Very fast |
| `getSymbol()` with relationships | ~10-30ms | Medium |
| `queryRelationships()` depth=1 | ~10-30ms | Fast |
| `queryRelationships()` depth=3 | ~100-500ms | Slower |

## Next Steps

1. Read [COMPLETE_EXAMPLE.md](COMPLETE_EXAMPLE.md) for full workflow
2. See [ts-client/AGENT_TOOLS_REFERENCE.md](ts-client/AGENT_TOOLS_REFERENCE.md) for in-depth docs
3. Try [ts-client/agent-example.ts](ts-client/agent-example.ts) examples
4. Integrate into your AI agent!
