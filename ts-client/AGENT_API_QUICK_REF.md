# Agent API Quick Reference

## Setup

```typescript
import { AgentCodeGraph } from "@consilium/codegraph-client";

const agent = new AgentCodeGraph("./my-repo");
// ... use the API
agent.close();
```

## The 3 Core Methods

### 1. `findSymbols()` - Search for code entities

```typescript
findSymbols(query: string, filters?: {
  kind?: string[];        // ["Function", "Class", "Method"]
  inFile?: string;        // "src/auth.ts"
  limit?: number;         // default: 100
}): SymbolSearchResult[]
```

**Use when**: You need to find code by name

```typescript
// Find all functions with "auth" in name
const authFuncs = agent.findSymbols("auth", { kind: ["Function"] });

// Find all symbols in a specific file
const fileSymbols = agent.findSymbols("", { inFile: "src/auth.ts" });

// Find all classes
const classes = agent.findSymbols("", { kind: ["Class"], limit: 50 });
```

---

### 2. `getSymbol()` - Get details + relationships

```typescript
getSymbol(identifier: string, options?: {
  includeCallers?: boolean;      // Who calls this
  includeCallees?: boolean;      // What this calls
  includeReferences?: boolean;   // All usages
  includeImports?: boolean;      // What this imports
  includeImportedBy?: boolean;   // Who imports this
  includeContains?: boolean;     // Child symbols (methods, etc.)
}): SymbolInfo | null
```

**Use when**: You need to understand a specific symbol and its relationships

```typescript
// Basic info (location, kind, signature)
const info = agent.getSymbol("authenticateUser");

// With call graph
const info = agent.getSymbol("authenticateUser", {
  includeCallers: true,
  includeCallees: true
});

console.log(`Calls ${info.callees.length} functions`);
console.log(`Called by ${info.callers.length} functions`);

// All references (like "Find All References" in IDE)
const info = agent.getSymbol("User", {
  includeReferences: true
});

console.log(`Used in ${info.references.length} places`);
```

---

### 3. `queryRelationships()` - Navigate the graph

```typescript
queryRelationships(
  symbolFqn: string,
  relationship: "calls" | "called-by" | "imports" | "imported-by" |
                "implements" | "implemented-by" | "contains" | "contained-in",
  options?: {
    depth?: number;       // How deep to traverse (default: 1)
    limit?: number;       // Max results (default: 100)
  }
): RelationshipNode[]
```

**Use when**: You need to traverse relationships deeply or in specific directions

```typescript
// Find direct callers
const callers = agent.queryRelationships("deleteUser", "called-by");

// Find transitive dependencies (depth=3)
const deps = agent.queryRelationships("processPayment", "calls", {
  depth: 3,
  limit: 50
});

// Results include depth and path
deps.forEach(dep => {
  console.log(`${dep.symbol} at depth ${dep.depth}`);
  if (dep.path) {
    console.log(`  Path: ${dep.path.join(" → ")}`);
  }
});

// Find implementations
const impls = agent.queryRelationships("IPaymentProvider", "implemented-by");
```

---

## Common Patterns

### Find where something is defined

```typescript
const symbol = agent.getSymbol("authenticateUser");
console.log(`Defined at ${symbol.location.file}:${symbol.location.line}`);

// Then use Read tool to get the code
```

### Impact analysis ("What breaks if I change this?")

```typescript
const impacted = agent.queryRelationships("deleteUser", "called-by", {
  depth: 2  // Include indirect callers
});

console.log(`Changing this affects ${impacted.length} functions`);
```

### Find entry points

```typescript
const allFuncs = agent.findSymbols("", { kind: ["Function"], limit: 100 });

const entryPoints = allFuncs.filter(fn => {
  const info = agent.getSymbol(fn.fqn, { includeCallers: true });
  return !info.callers || info.callers.length === 0;
});
```

### Find dead code

```typescript
const allSymbols = agent.findSymbols("", { limit: 500 });

const unused = allSymbols.filter(sym => {
  const info = agent.getSymbol(sym.fqn, {
    includeCallers: true,
    includeReferences: true
  });

  return (
    (!info.callers || info.callers.length === 0) &&
    (!info.references || info.references.length === 1)
  );
});
```

### Trace execution flow

```typescript
function traceFrom(startFn: string, maxDepth = 3) {
  const calls = agent.queryRelationships(startFn, "calls", {
    depth: maxDepth
  });

  // Group by depth
  const byDepth = {};
  calls.forEach(call => {
    byDepth[call.depth] = byDepth[call.depth] || [];
    byDepth[call.depth].push(call.symbol);
  });

  return byDepth;
}

const flow = traceFrom("main", 3);
console.log("Depth 1:", flow[1]);
console.log("Depth 2:", flow[2]);
console.log("Depth 3:", flow[3]);
```

### Explore a class

```typescript
const cls = agent.getSymbol("UserManager", {
  includeContains: true
});

console.log(`Class "${cls.name}" has ${cls.contains.length} members:`);
cls.contains.forEach(member => {
  const memberInfo = agent.getSymbol(member);
  console.log(`  - ${memberInfo.name} (${memberInfo.kind})`);
});
```

---

## Combining with Read/Grep/Glob

The agent API works best when combined with existing tools:

```typescript
// 1. Use findSymbols to locate code entities
const authSymbols = agent.findSymbols("auth", { kind: ["Function"] });

// 2. Use getSymbol to understand relationships
const mainAuth = agent.getSymbol(authSymbols[0].fqn, {
  includeCallees: true
});

// 3. Use Read to get the actual code
const code = await Read(mainAuth.location.file, {
  offset: mainAuth.location.line,
  limit: 20
});

// 4. Use queryRelationships for deep analysis
const dependencies = agent.queryRelationships(mainAuth.fqn, "calls", {
  depth: 2
});
```

---

## Return Types

### SymbolSearchResult
```typescript
{
  fqn: string;              // Fully qualified name
  name: string;             // Short name
  kind: string;             // "Function", "Class", etc.
  location: {
    file: string;
    line: number;
  };
  signature?: string;
}
```

### SymbolInfo
```typescript
{
  fqn: string;
  name: string;
  kind: string;
  location: { file, line, column? };
  signature?: string;
  containedIn?: string;         // Parent FQN
  contains?: string[];          // Child FQNs

  // Optional (based on request)
  callers?: string[];
  callees?: string[];
  references?: Location[];
  imports?: string[];
  importedBy?: string[];
}
```

### RelationshipNode
```typescript
{
  symbol: string;           // FQN
  kind: string;             // Symbol kind
  location: {
    file: string;
    line: number;
  };
  depth: number;            // 1, 2, 3...
  path?: string[];          // For deep queries
}
```

---

## Symbol Kinds

Common values for `kind`:
- `"Function"`
- `"Method"`
- `"Class"`
- `"Interface"`
- `"Variable"`
- `"Constant"`
- `"Type"`
- `"Module"`
- `"Enum"`
- `"Field"`
- `"Struct"`

---

## Tips

1. **Start simple** - Use `findSymbols` to explore, then `getSymbol` for details
2. **Limit depth** - Deep traversals can be slow, start with depth 1-2
3. **Use filters** - Narrow down by `kind` to reduce noise
4. **Cache results** - Store symbol info if you'll query it multiple times
5. **Combine tools** - Use graph API to find, Read to analyze
6. **Close connections** - Always call `agent.close()` when done

---

**Full Documentation**: See [AGENT_API.md](./AGENT_API.md)

**Examples**: See [agent-example.ts](./agent-example.ts)
