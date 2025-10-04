# Agent-Focused API

> **Design Philosophy**: Complement existing agentic tools (Read, Grep, Glob) by adding the **graph layer** — understanding relationships between code entities.

## Why This API?

AI coding agents already have powerful file-based tools:
- **Read**: Get file contents
- **Grep**: Search text in files
- **Glob**: Find files by pattern

But these can't answer:
- ❓ "What functions call this?"
- ❓ "Where is `authenticateUser` defined?"
- ❓ "What would break if I change this class?"
- ❓ "What does this function call?"

The **AgentCodeGraph API** fills this gap.

## The Two Core Tools

### 1. `getSymbol()` - Get Symbol with Relationships

Get detailed information about a code symbol (function, class, method, etc.) including its relationships.

```typescript
getSymbol(
  identifier: string,
  options?: {
    includeCallers?: boolean;
    includeCallees?: boolean;
    includeReferences?: boolean;
    includeImports?: boolean;
    includeImportedBy?: boolean;
    includeContains?: boolean;
  }
): SymbolInfo | null
```

**Returns**: Structured information about the symbol, not just grep matches.

#### Example 1: Basic Info

```typescript
import { AgentCodeGraph } from "@consilium/codegraph-client";

const agent = new AgentCodeGraph("./my-project");

// Find where a symbol is defined
const symbol = agent.getSymbol("authenticateUser");

console.log(symbol.location);
// { file: "src/auth.ts", line: 42, column: 0 }

console.log(symbol.kind);
// "Function"

console.log(symbol.signature);
// "function authenticateUser(email: string, password: string): Promise<User>"
```

#### Example 2: With Relationships

```typescript
// Get symbol with call graph
const symbol = agent.getSymbol("authenticateUser", {
  includeCallers: true,
  includeCallees: true,
});

console.log("This function calls:");
symbol.callees.forEach(fqn => console.log(`  - ${fqn}`));
// - validateCredentials
// - createSession
// - logAuthEvent

console.log("This function is called by:");
symbol.callers.forEach(fqn => console.log(`  - ${fqn}`));
// - handleLogin
// - handleOAuthCallback
```

#### Example 3: Find All References

```typescript
// See everywhere this symbol is used
const symbol = agent.getSymbol("User", {
  includeReferences: true,
});

console.log(`"User" is referenced in ${symbol.references.length} places:`);
symbol.references.forEach(ref => {
  console.log(`  ${ref.file}:${ref.line}`);
});
```

### 2. `queryRelationships()` - Navigate the Graph

Follow relationships between symbols, with support for deep traversal.

```typescript
queryRelationships(
  symbolFqn: string,
  relationship: RelationshipType,
  options?: {
    depth?: number;
    limit?: number;
  }
): RelationshipNode[]
```

**Relationship Types**:
- `"calls"` - Functions/methods this calls
- `"called-by"` - Functions/methods that call this
- `"imports"` - Symbols this imports
- `"imported-by"` - Symbols that import this
- `"implements"` - Interfaces this implements
- `"implemented-by"` - Classes that implement this interface
- `"contains"` - Symbols contained (methods in class, etc.)
- `"contained-in"` - Symbol that contains this

#### Example 1: Find Callers (Impact Analysis)

```typescript
// What would break if I change this?
const callers = agent.queryRelationships("authenticateUser", "called-by");

console.log(`${callers.length} functions depend on this`);

callers.forEach(caller => {
  console.log(`  - ${caller.symbol} at ${caller.location.file}:${caller.location.line}`);
});
```

#### Example 2: Deep Traversal

```typescript
// Find transitive dependencies (what this calls, and what those call)
const deps = agent.queryRelationships("processPayment", "calls", {
  depth: 3,  // Go 3 levels deep
  limit: 50  // Max 50 results
});

// Results include depth and path
deps.forEach(dep => {
  console.log(`Depth ${dep.depth}: ${dep.symbol}`);
  if (dep.path) {
    console.log(`  Path: ${dep.path.join(" → ")}`);
  }
});
```

#### Example 3: Find Implementations

```typescript
// Find all classes implementing an interface
const impls = agent.queryRelationships("IPaymentProvider", "implemented-by");

console.log("Implementations:");
impls.forEach(impl => {
  console.log(`  - ${impl.symbol} (${impl.kind})`);
});
// - StripeProvider (Class)
// - PayPalProvider (Class)
// - MockPaymentProvider (Class)
```

### 3. `findSymbols()` - Search for Symbols (Bonus)

Search the symbol table (more structured than Grep).

```typescript
findSymbols(
  query: string,
  filters?: {
    kind?: string[];
    inFile?: string;
    limit?: number;
  }
): SymbolSearchResult[]
```

#### Example

```typescript
// Find all functions with "validate" in the name
const validators = agent.findSymbols("validate", {
  kind: ["Function", "Method"],
});

validators.forEach(fn => {
  console.log(`${fn.name} in ${fn.location.file}:${fn.location.line}`);
});
```

## Real-World Agent Workflow

**Task**: "Understand the authentication flow"

```typescript
import { AgentCodeGraph } from "@consilium/codegraph-client";
import { readFileSync } from "fs"; // Standard Read tool

const agent = new AgentCodeGraph("./my-project");

// Step 1: Find auth-related symbols
const authSymbols = agent.findSymbols("auth", {
  kind: ["Function", "Class", "Method"]
});

console.log(`Found ${authSymbols.length} auth-related symbols`);

// Step 2: Get details about the main entry point
const mainAuth = authSymbols.find(s => s.name === "authenticateUser");
const authInfo = agent.getSymbol(mainAuth.fqn, {
  includeCallees: true,
  includeCallers: true
});

console.log(`authenticateUser calls ${authInfo.callees.length} functions`);

// Step 3: Read the actual source code (using standard file tools)
const sourceCode = readFileSync(authInfo.location.file, 'utf-8');
const lines = sourceCode.split('\n');
const relevantCode = lines.slice(
  authInfo.location.line - 1,
  authInfo.location.line + 20
).join('\n');

console.log("Source code:");
console.log(relevantCode);

// Step 4: Explore what it calls
for (const calleeFqn of authInfo.callees) {
  const callee = agent.getSymbol(calleeFqn);
  console.log(`  → Calls ${callee.name} at ${callee.location.file}:${callee.location.line}`);

  // Use Read to examine each dependency
  const calleeCode = readFileSync(callee.location.file, 'utf-8');
  // ... analyze calleeCode
}

// Step 5: Check impact before making changes
const impacted = agent.queryRelationships(mainAuth.fqn, "called-by", {
  depth: 2
});

console.log(`Changing this would impact ${impacted.length} functions`);

agent.close();
```

## Comparison: With vs. Without Graph API

### Scenario: "Find who calls the `deleteUser` function"

#### ❌ Without Graph API (using only Grep)

```typescript
// Use Grep to search for the function name
const grepResults = await Grep("deleteUser");

// Now you have to:
// 1. Parse grep results manually
// 2. Filter out the definition vs. calls
// 3. Filter out comments and strings
// 4. Guess which matches are actual function calls
// 5. Miss indirect calls through variables

// Result: Incomplete, error-prone
```

#### ✅ With Graph API

```typescript
const callers = agent.queryRelationships("deleteUser", "called-by");

// Result: Accurate list of all callers
callers.forEach(caller => {
  console.log(`${caller.symbol} at ${caller.location.file}:${caller.location.line}`);
});
```

### Scenario: "What would break if I rename this class?"

#### ❌ Without Graph API

```typescript
// Grep for the class name - might miss:
// - Type references
// - Imports
// - Inheritance
// - Method calls on instances
// - Constructor calls

// Result: Incomplete refactoring, broken code
```

#### ✅ With Graph API

```typescript
// Find all references
const symbol = agent.getSymbol("UserManager", {
  includeReferences: true,
  includeImportedBy: true,
});

console.log(`Found ${symbol.references.length} references`);
console.log(`Imported by ${symbol.importedBy.length} modules`);

// Result: Complete list of what needs updating
```

## Integration with Existing Tools

The graph API is designed to work **alongside** your existing tools:

| Tool | Use For | Example |
|------|---------|---------|
| **Grep** | Search text/comments | `grep("TODO")` |
| **Glob** | Find files by name | `glob("**/*.test.ts")` |
| **Read** | Get file contents | `read("src/auth.ts")` |
| **getSymbol()** | Understand relationships | `getSymbol("auth", { includeCallers: true })` |
| **queryRelationships()** | Navigate call graph | `queryRelationships("auth", "calls", { depth: 3 })` |
| **findSymbols()** | Search code entities | `findSymbols("validate", { kind: ["Function"] })` |

## Type Reference

### SymbolInfo

```typescript
interface SymbolInfo {
  fqn: string;              // Fully qualified name
  name: string;             // Short name
  kind: string;             // "Function", "Class", "Method", etc.
  location: Location;       // Where it's defined
  signature?: string;       // Type signature

  // Context
  containedIn?: string;     // Parent class/module
  contains?: string[];      // Child symbols

  // Relationships (populated based on options)
  callers?: string[];       // Who calls this
  callees?: string[];       // What this calls
  references?: Location[];  // All usages
  imports?: string[];       // What this imports
  importedBy?: string[];    // Who imports this
}
```

### RelationshipNode

```typescript
interface RelationshipNode {
  symbol: string;           // FQN of the symbol
  kind: string;             // Symbol kind
  location: Location;       // Where it's defined
  depth: number;            // Distance from start (1, 2, 3...)
  path?: string[];          // Call path (for deep queries)
}
```

### Location

```typescript
interface Location {
  file: string;             // Relative file path
  line: number;             // Line number
  column?: number;          // Column number
}
```

## Common Patterns

### Pattern 1: "Find entry points"

```typescript
const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 100 });

const entryPoints = allFunctions.filter(fn => {
  const info = agent.getSymbol(fn.fqn, { includeCallers: true });
  return !info.callers || info.callers.length === 0;
});

console.log("Entry points:", entryPoints.map(ep => ep.name));
```

### Pattern 2: "Find dead code"

```typescript
const allSymbols = agent.findSymbols("", { limit: 1000 });

const unused = allSymbols.filter(sym => {
  const info = agent.getSymbol(sym.fqn, {
    includeCallers: true,
    includeReferences: true
  });

  return (
    (!info.callers || info.callers.length === 0) &&
    (!info.references || info.references.length === 1) // Only definition
  );
});

console.log("Potentially unused:", unused.map(u => u.name));
```

### Pattern 3: "Trace execution path"

```typescript
// Start from an entry point, follow the calls
function traceExecution(startFn: string, maxDepth = 5) {
  const visited = new Set<string>();

  function trace(fqn: string, depth: number, indent = "") {
    if (depth >= maxDepth || visited.has(fqn)) return;
    visited.add(fqn);

    const symbol = agent.getSymbol(fqn, { includeCallees: true });
    console.log(`${indent}${symbol.name} (${symbol.location.file}:${symbol.location.line})`);

    symbol.callees?.forEach(callee => {
      trace(callee, depth + 1, indent + "  ");
    });
  }

  trace(startFn, 0);
}

traceExecution("main");
```

### Pattern 4: "Find similar functions"

```typescript
// Find functions with similar call patterns
const targetFn = agent.getSymbol("processPayment", { includeCallees: true });
const targetCallees = new Set(targetFn.callees);

const allFunctions = agent.findSymbols("", { kind: ["Function"] });

const similar = allFunctions
  .map(fn => {
    const info = agent.getSymbol(fn.fqn, { includeCallees: true });
    const overlap = info.callees?.filter(c => targetCallees.has(c)).length || 0;
    return { fn, overlap };
  })
  .filter(({ overlap }) => overlap > 0)
  .sort((a, b) => b.overlap - a.overlap);

console.log("Similar functions:");
similar.forEach(({ fn, overlap }) => {
  console.log(`  ${fn.name} (${overlap} common dependencies)`);
});
```

## Best Practices

1. **Start with findSymbols** - Get a list of relevant symbols first
2. **Use getSymbol for details** - Get relationships for specific symbols
3. **Combine with Read** - Use the graph to find code, then Read to analyze it
4. **Limit depth** - Deep traversals can be expensive, start with depth 1-2
5. **Cache results** - If analyzing multiple symbols, store results to avoid re-querying
6. **Close when done** - Call `agent.close()` to close the database connection

## Examples

See [agent-example.ts](./agent-example.ts) for a complete working example.

## API Summary

```typescript
import { AgentCodeGraph } from "@consilium/codegraph-client";

const agent = new AgentCodeGraph(repoPath);

// Search for symbols
const symbols = agent.findSymbols(query, filters);

// Get symbol details with relationships
const symbol = agent.getSymbol(fqn, options);

// Navigate the graph
const related = agent.queryRelationships(fqn, relationship, options);

agent.close();
```

---

**Next**: See [agent-example.ts](./agent-example.ts) for real-world usage patterns.
