# Agent-Focused API Summary

## What We Built

A minimal, focused API for AI coding agents that **complements** existing file tools (Read, Grep, Glob) by adding the **graph layer** - understanding relationships between code entities.

## The Problem We Solved

AI agents already have:
- ✅ **Read** - Get file contents
- ✅ **Grep** - Search text
- ✅ **Glob** - Find files

But they can't:
- ❌ "What functions call this?"
- ❌ "Where is X defined?"
- ❌ "What would break if I change this?"
- ❌ "What does this function call?"

## The Solution: 2 Core Tools

### 1. `getSymbol()` - Get symbol with relationships

```typescript
const info = agent.getSymbol("authenticateUser", {
  includeCallers: true,
  includeCallees: true
});

// Returns structured info:
// - Where it's defined (file:line)
// - What it calls
// - What calls it
// - Type signature
// - Parent/container
```

**Replaces**: Trying to grep for function calls and manually parsing results

### 2. `queryRelationships()` - Navigate the graph

```typescript
// Find all callers (with depth)
const callers = agent.queryRelationships("deleteUser", "called-by", {
  depth: 2
});

// Each result includes:
// - Symbol FQN
// - Location (file:line)
// - Depth from start
// - Call path (for deep queries)
```

**Replaces**: Impossible with just grep/read

### Bonus: `findSymbols()` - Structured search

```typescript
// Find all functions with "auth" in name
const funcs = agent.findSymbols("auth", {
  kind: ["Function", "Method"]
});
```

**Better than grep**: Returns typed results with locations

## Design Principles

1. **Minimal** - Only 3 methods, clear purpose for each
2. **Complementary** - Works alongside Read/Grep/Glob, doesn't replace them
3. **Focused** - Only graph operations, not file operations
4. **Typed** - Returns structured data, not text to parse
5. **Efficient** - Direct SQLite queries, no extra processing

## Integration Example

```typescript
import { AgentCodeGraph } from "@consilium/codegraph-client";
import { readFileSync } from "fs";

const agent = new AgentCodeGraph("./my-repo");

// Step 1: Find relevant symbols (graph tool)
const symbols = agent.findSymbols("auth", { kind: ["Function"] });

// Step 2: Get relationships (graph tool)
const info = agent.getSymbol(symbols[0].fqn, {
  includeCallees: true
});

// Step 3: Read actual code (existing tool)
const code = readFileSync(info.location.file, 'utf-8');

// Step 4: Navigate dependencies (graph tool)
for (const calleeFqn of info.callees) {
  const callee = agent.getSymbol(calleeFqn);

  // Step 5: Read dependency code (existing tool)
  const calleeCode = readFileSync(callee.location.file, 'utf-8');
}

agent.close();
```

## Common Use Cases

### 1. Impact Analysis
```typescript
const affected = agent.queryRelationships("deleteUser", "called-by", {
  depth: 2
});
console.log(`Changing this affects ${affected.length} symbols`);
```

### 2. Find Entry Points
```typescript
const allFuncs = agent.findSymbols("", { kind: ["Function"], limit: 100 });
const entryPoints = allFuncs.filter(fn => {
  const info = agent.getSymbol(fn.fqn, { includeCallers: true });
  return !info.callers || info.callers.length === 0;
});
```

### 3. Trace Execution
```typescript
const flow = agent.queryRelationships("main", "calls", { depth: 3 });
// Returns full call graph from main, 3 levels deep
```

### 4. Find Dead Code
```typescript
const unused = allSymbols.filter(sym => {
  const info = agent.getSymbol(sym.fqn, {
    includeCallers: true,
    includeReferences: true
  });
  return !info.callers?.length && info.references?.length === 1;
});
```

## API Surface

```typescript
class AgentCodeGraph {
  constructor(repoPath: string, dbPath?: string)

  // Search for symbols by name
  findSymbols(query: string, filters?: {
    kind?: string[];
    inFile?: string;
    limit?: number;
  }): SymbolSearchResult[]

  // Get symbol with optional relationships
  getSymbol(identifier: string, options?: {
    includeCallers?: boolean;
    includeCallees?: boolean;
    includeReferences?: boolean;
    includeImports?: boolean;
    includeImportedBy?: boolean;
    includeContains?: boolean;
  }): SymbolInfo | null

  // Navigate graph relationships
  queryRelationships(
    symbolFqn: string,
    relationship: "calls" | "called-by" | "imports" | "imported-by" |
                  "implements" | "implemented-by" | "contains" | "contained-in",
    options?: {
      depth?: number;
      limit?: number;
    }
  ): RelationshipNode[]

  close(): void
}
```

## Files Created

1. **Implementation**: `ts-client/src/agent-api.ts`
2. **Full Documentation**: `ts-client/AGENT_API.md`
3. **Quick Reference**: `ts-client/AGENT_API_QUICK_REF.md`
4. **Example**: `ts-client/agent-example.ts`
5. **Tests**: `ts-client/test-agent-api.ts`

## How to Use

```bash
cd ts-client
npm install
npm run build

# Run the example
npx tsx agent-example.ts

# Run tests
npx tsx test-agent-api.ts
```

## Import

```typescript
import { AgentCodeGraph } from "@consilium/codegraph-client";
```

Or if using locally:

```typescript
import { AgentCodeGraph } from "./ts-client/src/agent-api";
```

## Key Benefits vs. Full APIs

| Feature | Full API (CodeGraphAPI) | Agent API (AgentCodeGraph) |
|---------|------------------------|---------------------------|
| Methods | 20+ methods | 3 core methods |
| Focus | Complete feature set | Graph relationships only |
| Complexity | High - many options | Low - minimal, focused |
| Token overhead | Large (many tools) | Small (3 tools) |
| Integration | Standalone | Complements Read/Grep/Glob |
| Use case | Full applications | AI coding agents |

## Next Steps

See the full documentation:
- **Quick Start**: `ts-client/AGENT_API_QUICK_REF.md`
- **Complete Guide**: `ts-client/AGENT_API.md`
- **Working Example**: `ts-client/agent-example.ts`
