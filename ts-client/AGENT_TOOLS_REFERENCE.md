# Agent Tools - In-Depth Reference

Complete reference documentation for the `AgentCodeGraph` API tools.

---

## Table of Contents

1. [getSymbol()](#getsymbol)
2. [queryRelationships()](#queryrelationships)
3. [findSymbols()](#findsymbols)
4. [Type Definitions](#type-definitions)
5. [Error Handling](#error-handling)
6. [Performance Considerations](#performance-considerations)

---

## `getSymbol()`

Get detailed information about a code symbol including its location, type, and optionally its relationships to other symbols.

### Signature

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

### Parameters

#### `identifier` (required)
- **Type**: `string`
- **Description**: The symbol to look up. Can be either:
  - **Fully Qualified Name (FQN)**: `"my_module.MyClass.myMethod"`
  - **Simple name**: `"myMethod"` (will match first occurrence)
- **Examples**:
  ```typescript
  "authenticateUser"                    // Simple name
  "src/auth.ts#UserService"            // FQN with file
  "UserService.authenticate"            // Class.method
  ```

#### `options.includeCallers`
- **Type**: `boolean`
- **Default**: `false`
- **Description**: Include list of symbols that call this one
- **Populates**: `SymbolInfo.callers` (array of FQNs)
- **Performance**: Medium - requires edge traversal
- **Example**:
  ```typescript
  const info = agent.getSymbol("deleteUser", { includeCallers: true });
  console.log(`Called by: ${info.callers.length} symbols`);
  info.callers.forEach(caller => console.log(`  - ${caller}`));
  ```

#### `options.includeCallees`
- **Type**: `boolean`
- **Default**: `false`
- **Description**: Include list of symbols this one calls
- **Populates**: `SymbolInfo.callees` (array of FQNs)
- **Performance**: Medium - requires edge traversal
- **Use case**: Understanding dependencies, tracing execution flow
- **Example**:
  ```typescript
  const info = agent.getSymbol("processPayment", { includeCallees: true });
  console.log(`Calls: ${info.callees.length} functions`);

  // Then inspect each callee
  info.callees.forEach(calleeFqn => {
    const callee = agent.getSymbol(calleeFqn);
    console.log(`  → ${callee.name} at ${callee.location.file}`);
  });
  ```

#### `options.includeReferences`
- **Type**: `boolean`
- **Default**: `false`
- **Description**: Include all locations where this symbol is referenced/used
- **Populates**: `SymbolInfo.references` (array of `Location` objects)
- **Performance**: Expensive - scans occurrence table
- **Use case**: Find all usages (like "Find All References" in IDE)
- **Example**:
  ```typescript
  const info = agent.getSymbol("User", { includeReferences: true });
  console.log(`"User" is referenced in ${info.references.length} places:`);

  info.references.forEach(ref => {
    console.log(`  ${ref.file}:${ref.line}`);
    // Then use Read tool to see context
  });
  ```

#### `options.includeImports`
- **Type**: `boolean`
- **Default**: `false`
- **Description**: Include symbols that this symbol imports
- **Populates**: `SymbolInfo.imports` (array of FQNs)
- **Performance**: Medium - requires edge traversal
- **Use case**: Understanding module dependencies
- **Example**:
  ```typescript
  const info = agent.getSymbol("AuthService", { includeImports: true });
  console.log("Imports:", info.imports);
  ```

#### `options.includeImportedBy`
- **Type**: `boolean`
- **Default**: `false`
- **Description**: Include symbols that import this one
- **Populates**: `SymbolInfo.importedBy` (array of FQNs)
- **Performance**: Medium - requires edge traversal
- **Use case**: Impact analysis, finding dependents
- **Example**:
  ```typescript
  const info = agent.getSymbol("constants", { includeImportedBy: true });
  console.log(`Used by ${info.importedBy.length} modules`);
  ```

#### `options.includeContains`
- **Type**: `boolean`
- **Default**: `false`
- **Description**: Include child symbols (methods in a class, functions in a module, etc.)
- **Populates**: `SymbolInfo.contains` (array of FQNs)
- **Performance**: Medium - requires edge traversal
- **Use case**: Exploring class structure, module contents
- **Example**:
  ```typescript
  const cls = agent.getSymbol("UserManager", { includeContains: true });
  console.log(`Class has ${cls.contains.length} members:`);

  cls.contains.forEach(memberFqn => {
    const member = agent.getSymbol(memberFqn);
    console.log(`  - ${member.name} (${member.kind})`);
  });
  ```

### Return Value

Returns `SymbolInfo | null`

- **`null`**: Symbol not found
- **`SymbolInfo`**: Object with symbol details (see [Type Definitions](#symbolinfo))

### Return Value Fields

Always present:
- `fqn` - Fully qualified name
- `name` - Short name
- `kind` - Symbol type ("Function", "Class", "Method", etc.)
- `location` - Where it's defined (`{ file, line, column? }`)
- `signature` - Type signature (if available)
- `containedIn` - Parent symbol FQN (if any)

Conditionally present (based on options):
- `callers` - Array of FQNs (if `includeCallers: true`)
- `callees` - Array of FQNs (if `includeCallees: true`)
- `references` - Array of Locations (if `includeReferences: true`)
- `imports` - Array of FQNs (if `includeImports: true`)
- `importedBy` - Array of FQNs (if `includeImportedBy: true`)
- `contains` - Array of FQNs (if `includeContains: true`)

### Examples

#### Example 1: Basic symbol lookup

```typescript
const symbol = agent.getSymbol("authenticateUser");

if (!symbol) {
  console.log("Symbol not found");
} else {
  console.log(`Found: ${symbol.name}`);
  console.log(`Kind: ${symbol.kind}`);
  console.log(`Location: ${symbol.location.file}:${symbol.location.line}`);
  console.log(`Signature: ${symbol.signature || 'N/A'}`);
}
```

**Output**:
```
Found: authenticateUser
Kind: Function
Location: src/auth/service.ts:42
Signature: function(email: string, password: string): Promise<User>
```

#### Example 2: Call graph analysis

```typescript
const info = agent.getSymbol("validateCredentials", {
  includeCallers: true,
  includeCallees: true
});

console.log(`\n${info.name} (${info.kind})`);
console.log(`  Defined: ${info.location.file}:${info.location.line}`);
console.log(`\n  Calls ${info.callees?.length || 0} functions:`);
info.callees?.forEach(fqn => console.log(`    → ${fqn}`));

console.log(`\n  Called by ${info.callers?.length || 0} functions:`);
info.callers?.forEach(fqn => console.log(`    ← ${fqn}`));
```

**Output**:
```
validateCredentials (Function)
  Defined: src/auth/validators.ts:15

  Calls 3 functions:
    → hashPassword
    → compareHash
    → logAttempt

  Called by 2 functions:
    ← authenticateUser
    ← verifyToken
```

#### Example 3: Finding all usages

```typescript
const info = agent.getSymbol("API_KEY", { includeReferences: true });

console.log(`"${info.name}" is used in ${info.references.length} places:\n`);

// Group by file
const byFile = info.references.reduce((acc, ref) => {
  acc[ref.file] = acc[ref.file] || [];
  acc[ref.file].push(ref.line);
  return acc;
}, {});

Object.entries(byFile).forEach(([file, lines]) => {
  console.log(`  ${file}:`);
  lines.forEach(line => console.log(`    Line ${line}`));
});
```

**Output**:
```
"API_KEY" is used in 5 places:

  src/config.ts:
    Line 10
  src/services/api.ts:
    Line 23
    Line 45
  tests/config.test.ts:
    Line 8
    Line 12
```

#### Example 4: Exploring class structure

```typescript
const cls = agent.getSymbol("UserService", { includeContains: true });

console.log(`Class: ${cls.name}\n`);

// Categorize members by kind
const members = cls.contains.map(fqn => agent.getSymbol(fqn));
const methods = members.filter(m => m.kind === "Method");
const fields = members.filter(m => m.kind === "Field");

console.log(`Methods (${methods.length}):`);
methods.forEach(m => console.log(`  - ${m.name}${m.signature || ''}`));

console.log(`\nFields (${fields.length}):`);
fields.forEach(f => console.log(`  - ${f.name}: ${f.signature || 'unknown'}`));
```

**Output**:
```
Class: UserService

Methods (4):
  - authenticate(email: string, password: string)
  - createUser(data: UserData)
  - updateProfile(id: string, updates: Partial<User>)
  - deleteUser(id: string)

Fields (2):
  - db: Database
  - logger: Logger
```

#### Example 5: Combined analysis

```typescript
function analyzeFunction(fnName: string) {
  const info = agent.getSymbol(fnName, {
    includeCallers: true,
    includeCallees: true,
    includeReferences: true
  });

  if (!info) {
    console.log(`Function "${fnName}" not found`);
    return;
  }

  console.log(`\n📊 Analysis of ${info.name}\n`);
  console.log(`Location: ${info.location.file}:${info.location.line}`);
  console.log(`Signature: ${info.signature || 'N/A'}`);

  console.log(`\nDependencies: ${info.callees?.length || 0} functions`);
  console.log(`Dependents: ${info.callers?.length || 0} functions`);
  console.log(`Total usages: ${info.references?.length || 0} locations`);

  // Risk assessment
  const isEntryPoint = !info.callers || info.callers.length === 0;
  const isHighlyDepended = (info.callers?.length || 0) > 5;
  const isComplex = (info.callees?.length || 0) > 10;

  console.log(`\n🔍 Insights:`);
  if (isEntryPoint) console.log(`  ⚠️  Entry point (no callers)`);
  if (isHighlyDepended) console.log(`  ⚠️  Highly depended upon`);
  if (isComplex) console.log(`  ⚠️  Complex (many dependencies)`);

  if (!isEntryPoint && !isHighlyDepended && !isComplex) {
    console.log(`  ✅ Healthy complexity`);
  }
}

analyzeFunction("processPayment");
```

**Output**:
```
📊 Analysis of processPayment

Location: src/payments/processor.ts:56
Signature: async function(amount: number, method: PaymentMethod)

Dependencies: 8 functions
Dependents: 3 functions
Total usages: 12 locations

🔍 Insights:
  ✅ Healthy complexity
```

### Edge Cases

#### Symbol not found
```typescript
const info = agent.getSymbol("nonExistentFunction");
// Returns: null

if (!info) {
  console.log("Symbol not found - try findSymbols() to search");
}
```

#### Ambiguous name (multiple matches)
```typescript
// If multiple symbols have the same name, returns the first match
const info = agent.getSymbol("User");
// Matches the first "User" found (could be class, interface, type, etc.)

// Better: Use FQN for precision
const info = agent.getSymbol("src/models/user.ts#User");
```

#### No relationships
```typescript
const info = agent.getSymbol("utilFunction", {
  includeCallers: true,
  includeCallees: true
});

// If no callers/callees exist:
console.log(info.callers);   // []
console.log(info.callees);   // []
```

#### Empty signature
```typescript
const info = agent.getSymbol("myFunction");
console.log(info.signature);  // undefined (if type info not available)
```

### Performance Tips

1. **Don't include everything** - Only request relationships you need:
   ```typescript
   // ❌ Slow - fetches everything
   const info = agent.getSymbol(fqn, {
     includeCallers: true,
     includeCallees: true,
     includeReferences: true,
     includeImports: true,
     includeImportedBy: true,
     includeContains: true
   });

   // ✅ Fast - only what you need
   const info = agent.getSymbol(fqn, {
     includeCallees: true  // Just this
   });
   ```

2. **Cache results** if analyzing multiple times:
   ```typescript
   const cache = new Map();

   function getCachedSymbol(fqn, options) {
     const key = `${fqn}:${JSON.stringify(options)}`;
     if (!cache.has(key)) {
       cache.set(key, agent.getSymbol(fqn, options));
     }
     return cache.get(key);
   }
   ```

3. **References are expensive** - Use sparingly:
   ```typescript
   // ❌ Slow if called many times
   symbols.forEach(s => {
     const info = agent.getSymbol(s.fqn, { includeReferences: true });
   });

   // ✅ Faster - only for specific symbols
   const mainSymbol = agent.getSymbol("main", { includeReferences: true });
   ```

### Common Patterns

#### Pattern: "Is this dead code?"
```typescript
function isPotentiallyUnused(symbolName) {
  const info = agent.getSymbol(symbolName, {
    includeCallers: true,
    includeReferences: true
  });

  if (!info) return null;

  return {
    isUnused: (
      (!info.callers || info.callers.length === 0) &&
      (!info.references || info.references.length <= 1) // Only definition
    ),
    callerCount: info.callers?.length || 0,
    referenceCount: info.references?.length || 0
  };
}
```

#### Pattern: "What's the impact radius?"
```typescript
function getImpactRadius(symbolName, depth = 2) {
  const visited = new Set();
  const queue = [symbolName];

  while (queue.length > 0) {
    const current = queue.shift();
    if (visited.has(current)) continue;
    visited.add(current);

    const info = agent.getSymbol(current, { includeCallers: true });
    if (info?.callers) {
      queue.push(...info.callers);
    }
  }

  return Array.from(visited);
}
```

#### Pattern: "Build execution trace"
```typescript
function traceExecution(entryPoint, maxDepth = 5) {
  const trace = [];

  function visit(fqn, depth, path) {
    if (depth >= maxDepth) return;

    const info = agent.getSymbol(fqn, { includeCallees: true });
    if (!info) return;

    trace.push({
      symbol: info.name,
      location: info.location,
      depth,
      path: [...path, info.name]
    });

    info.callees?.forEach(callee => {
      visit(callee, depth + 1, [...path, info.name]);
    });
  }

  visit(entryPoint, 0, []);
  return trace;
}
```

---

## `queryRelationships()`

Navigate relationships in the code graph with support for deep traversal.

### Signature

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

### Parameters

#### `symbolFqn` (required)
- **Type**: `string`
- **Description**: Fully qualified name of the starting symbol
- **Note**: Must be exact FQN, not a simple name
- **Example**:
  ```typescript
  "authenticateUser"
  "UserService.validateCredentials"
  "src/utils/helpers.ts#formatDate"
  ```

#### `relationship` (required)
- **Type**: `RelationshipType`
- **Description**: Type of relationship to follow
- **Values**:
  - `"calls"` - Functions/methods this symbol calls
  - `"called-by"` - Functions/methods that call this symbol
  - `"imports"` - Symbols this symbol imports
  - `"imported-by"` - Symbols that import this symbol
  - `"implements"` - Interfaces this class implements
  - `"implemented-by"` - Classes that implement this interface
  - `"contains"` - Symbols contained by this (methods in class)
  - `"contained-in"` - Symbol that contains this

#### `options.depth`
- **Type**: `number`
- **Default**: `1`
- **Range**: `1` to `∞` (practically limited by graph size)
- **Description**: How many levels deep to traverse
- **Performance**: Grows exponentially with depth
- **Examples**:
  - `depth: 1` - Direct relationships only
  - `depth: 2` - Relationships + their relationships
  - `depth: 3` - Three levels deep
- **Typical usage**:
  ```typescript
  // Depth 1: Direct callers
  const direct = agent.queryRelationships(fqn, "called-by", { depth: 1 });

  // Depth 2: Include indirect callers
  const indirect = agent.queryRelationships(fqn, "called-by", { depth: 2 });

  // Depth 3+: Full transitive closure (use carefully)
  const all = agent.queryRelationships(fqn, "called-by", { depth: 5 });
  ```

#### `options.limit`
- **Type**: `number`
- **Default**: `100`
- **Description**: Maximum number of results to return
- **Purpose**: Prevent overwhelming results, protect performance
- **Note**: Results are truncated if limit is reached
- **Example**:
  ```typescript
  // Get at most 10 callers
  const topCallers = agent.queryRelationships(fqn, "called-by", {
    depth: 1,
    limit: 10
  });
  ```

### Return Value

Returns `RelationshipNode[]` - Array of symbols related to the starting symbol.

Each node contains:
- `symbol` - FQN of the related symbol
- `kind` - Symbol kind ("Function", "Class", etc.)
- `location` - Where it's defined (`{ file, line }`)
- `depth` - Distance from start (1, 2, 3, ...)
- `path` - (Optional) Full path from start to this node

### Relationship Types Explained

#### `"calls"` - Outgoing calls
```typescript
// Find what a function calls
const callees = agent.queryRelationships("processOrder", "calls");

// callees = [
//   { symbol: "validateOrder", depth: 1, ... },
//   { symbol: "chargePayment", depth: 1, ... },
//   { symbol: "sendEmail", depth: 1, ... }
// ]
```

Use when:
- Understanding dependencies
- Tracing execution flow
- Building call graphs

#### `"called-by"` - Incoming calls
```typescript
// Find what calls this function
const callers = agent.queryRelationships("sendEmail", "called-by");

// callers = [
//   { symbol: "processOrder", depth: 1, ... },
//   { symbol: "notifyUser", depth: 1, ... }
// ]
```

Use when:
- Impact analysis
- Finding usages
- Identifying entry points

#### `"imports"` - Outgoing imports
```typescript
// Find what this module imports
const imports = agent.queryRelationships("UserService", "imports");

// imports = [
//   { symbol: "Database", depth: 1, ... },
//   { symbol: "Logger", depth: 1, ... }
// ]
```

Use when:
- Understanding module dependencies
- Analyzing coupling

#### `"imported-by"` - Incoming imports
```typescript
// Find what imports this module
const importers = agent.queryRelationships("constants", "imported-by");

// importers = [
//   { symbol: "UserService", depth: 1, ... },
//   { symbol: "AuthService", depth: 1, ... },
//   { symbol: "AdminService", depth: 1, ... }
// ]
```

Use when:
- Impact analysis for module changes
- Finding dependents

#### `"implements"` - Interface implementation
```typescript
// Find what interfaces a class implements
const interfaces = agent.queryRelationships("UserService", "implements");

// interfaces = [
//   { symbol: "IService", depth: 1, ... },
//   { symbol: "ILoggable", depth: 1, ... }
// ]
```

Use when:
- Understanding class contracts
- Finding polymorphic relationships

#### `"implemented-by"` - Implementation lookup
```typescript
// Find classes that implement an interface
const impls = agent.queryRelationships("IPaymentProvider", "implemented-by");

// impls = [
//   { symbol: "StripeProvider", depth: 1, ... },
//   { symbol: "PayPalProvider", depth: 1, ... }
// ]
```

Use when:
- Finding all implementations
- Understanding polymorphism

#### `"contains"` - Container contents
```typescript
// Find members of a class
const members = agent.queryRelationships("UserService", "contains");

// members = [
//   { symbol: "UserService.authenticate", depth: 1, kind: "Method", ... },
//   { symbol: "UserService.createUser", depth: 1, kind: "Method", ... },
//   { symbol: "UserService.db", depth: 1, kind: "Field", ... }
// ]
```

Use when:
- Exploring class structure
- Finding all methods/fields

#### `"contained-in"` - Parent container
```typescript
// Find what contains this method
const parent = agent.queryRelationships("authenticate", "contained-in");

// parent = [
//   { symbol: "UserService", depth: 1, kind: "Class", ... }
// ]
```

Use when:
- Finding parent class/module
- Understanding scope

### Examples

#### Example 1: Direct callers

```typescript
const callers = agent.queryRelationships("deleteUser", "called-by", {
  depth: 1
});

console.log(`${callers.length} functions call deleteUser:\n`);
callers.forEach(caller => {
  console.log(`  - ${caller.symbol}`);
  console.log(`    at ${caller.location.file}:${caller.location.line}`);
});
```

**Output**:
```
3 functions call deleteUser:

  - AdminService.removeUser
    at src/admin/service.ts:45
  - UserManager.cleanup
    at src/users/manager.ts:89
  - deleteAccount
    at src/api/routes.ts:123
```

#### Example 2: Deep traversal with path

```typescript
const deps = agent.queryRelationships("main", "calls", {
  depth: 3,
  limit: 20
});

console.log("Execution flow from main:\n");

// Group by depth
const byDepth = deps.reduce((acc, node) => {
  acc[node.depth] = acc[node.depth] || [];
  acc[node.depth].push(node);
  return acc;
}, {});

Object.entries(byDepth).forEach(([depth, nodes]) => {
  console.log(`Depth ${depth}: ${nodes.length} functions`);
  nodes.slice(0, 3).forEach(node => {
    console.log(`  - ${node.symbol}`);
    if (node.path) {
      console.log(`    Path: ${node.path.join(" → ")}`);
    }
  });
  if (nodes.length > 3) {
    console.log(`  ... and ${nodes.length - 3} more`);
  }
});
```

**Output**:
```
Execution flow from main:

Depth 1: 4 functions
  - initializeApp
  - startServer
  - setupRoutes
  - connectDatabase

Depth 2: 12 functions
  - loadConfig
    Path: main → initializeApp → loadConfig
  - setupLogger
    Path: main → initializeApp → setupLogger
  - createExpressApp
    Path: main → startServer → createExpressApp

Depth 3: 8 functions
  - readEnvFile
    Path: main → initializeApp → loadConfig → readEnvFile
  - validateConfig
    Path: main → initializeApp → loadConfig → validateConfig
  ... and 6 more
```

#### Example 3: Impact analysis

```typescript
function analyzeChangeImpact(symbolName, maxDepth = 3) {
  const impacted = agent.queryRelationships(symbolName, "called-by", {
    depth: maxDepth,
    limit: 100
  });

  console.log(`\n🔍 Impact Analysis: ${symbolName}\n`);
  console.log(`Total impacted symbols: ${impacted.length}`);

  // Calculate risk
  const directImpact = impacted.filter(n => n.depth === 1).length;
  const indirectImpact = impacted.filter(n => n.depth > 1).length;

  console.log(`  Direct dependents: ${directImpact}`);
  console.log(`  Indirect dependents: ${indirectImpact}`);

  // Risk level
  const riskLevel =
    impacted.length > 20 ? "HIGH" :
    impacted.length > 10 ? "MEDIUM" :
    "LOW";

  console.log(`  Risk level: ${riskLevel}`);

  // Show critical paths (entry points to this symbol)
  const entryPoints = impacted.filter(n => {
    // Check if this node is an entry point
    const nodeInfo = agent.getSymbol(n.symbol, { includeCallers: true });
    return !nodeInfo.callers || nodeInfo.callers.length === 0;
  });

  if (entryPoints.length > 0) {
    console.log(`\n  Entry points affected: ${entryPoints.length}`);
    entryPoints.slice(0, 3).forEach(ep => {
      console.log(`    - ${ep.symbol} (depth: ${ep.depth})`);
    });
  }

  return { total: impacted.length, directImpact, indirectImpact, riskLevel };
}

analyzeChangeImpact("validateCredentials", 3);
```

**Output**:
```
🔍 Impact Analysis: validateCredentials

Total impacted symbols: 15
  Direct dependents: 3
  Indirect dependents: 12
  Risk level: MEDIUM

  Entry points affected: 2
    - handleLogin (depth: 2)
    - processOAuthCallback (depth: 3)
```

#### Example 4: Finding all implementations

```typescript
const interfaceName = "IPaymentProvider";
const impls = agent.queryRelationships(interfaceName, "implemented-by");

console.log(`\nImplementations of ${interfaceName}:\n`);

impls.forEach(impl => {
  const details = agent.getSymbol(impl.symbol, { includeContains: true });

  console.log(`  ${impl.symbol}`);
  console.log(`    File: ${impl.location.file}`);
  console.log(`    Methods: ${details.contains?.length || 0}`);
});
```

**Output**:
```
Implementations of IPaymentProvider:

  StripeProvider
    File: src/payments/stripe.ts
    Methods: 5
  PayPalProvider
    File: src/payments/paypal.ts
    Methods: 5
  MockPaymentProvider
    File: tests/mocks/payment.ts
    Methods: 5
```

#### Example 5: Build dependency tree

```typescript
function buildDependencyTree(rootSymbol, maxDepth = 2) {
  const tree = { name: rootSymbol, children: [] };

  function buildNode(symbol, depth) {
    if (depth >= maxDepth) return null;

    const deps = agent.queryRelationships(symbol, "calls", {
      depth: 1,
      limit: 10
    });

    return {
      name: symbol,
      children: deps.map(dep => buildNode(dep.symbol, depth + 1)).filter(Boolean)
    };
  }

  return buildNode(rootSymbol, 0);
}

const tree = buildDependencyTree("processPayment", 3);
console.log(JSON.stringify(tree, null, 2));
```

**Output**:
```json
{
  "name": "processPayment",
  "children": [
    {
      "name": "validatePayment",
      "children": [
        { "name": "checkAmount", "children": [] },
        { "name": "verifyCard", "children": [] }
      ]
    },
    {
      "name": "chargeCard",
      "children": [
        { "name": "callStripeAPI", "children": [] },
        { "name": "recordTransaction", "children": [] }
      ]
    }
  ]
}
```

### Performance Considerations

#### Depth Impact

```typescript
// Depth 1: ~10ms, 5 results
agent.queryRelationships(fqn, "calls", { depth: 1 });

// Depth 2: ~50ms, 25 results
agent.queryRelationships(fqn, "calls", { depth: 2 });

// Depth 3: ~200ms, 100+ results
agent.queryRelationships(fqn, "calls", { depth: 3 });

// Depth 5+: Can be very slow in large codebases
agent.queryRelationships(fqn, "calls", { depth: 5 });
```

**Recommendation**: Start with depth 1-2, increase only if needed.

#### Using Limits

```typescript
// ❌ Potentially slow - no limit
const all = agent.queryRelationships(fqn, "called-by", { depth: 3 });

// ✅ Fast - reasonable limit
const limited = agent.queryRelationships(fqn, "called-by", {
  depth: 3,
  limit: 50
});

if (limited.length === 50) {
  console.log("Results truncated, increase limit if needed");
}
```

### Common Patterns

#### Pattern: "Find entry points"
```typescript
function findEntryPoints() {
  const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 100 });

  return allFunctions.filter(fn => {
    const callers = agent.queryRelationships(fn.fqn, "called-by", { depth: 1 });
    return callers.length === 0;
  });
}
```

#### Pattern: "Find leaf functions (call nothing)"
```typescript
function findLeafFunctions() {
  const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 100 });

  return allFunctions.filter(fn => {
    const callees = agent.queryRelationships(fn.fqn, "calls", { depth: 1 });
    return callees.length === 0;
  });
}
```

#### Pattern: "Find circular dependencies"
```typescript
function findCircularDeps(startSymbol, maxDepth = 5) {
  const visited = new Set();
  const path = [];
  const cycles = [];

  function dfs(symbol, depth) {
    if (depth > maxDepth) return;

    if (visited.has(symbol)) {
      // Found a cycle
      const cycleStart = path.indexOf(symbol);
      if (cycleStart !== -1) {
        cycles.push([...path.slice(cycleStart), symbol]);
      }
      return;
    }

    visited.add(symbol);
    path.push(symbol);

    const callees = agent.queryRelationships(symbol, "calls", { depth: 1 });
    callees.forEach(callee => dfs(callee.symbol, depth + 1));

    path.pop();
    visited.delete(symbol);
  }

  dfs(startSymbol, 0);
  return cycles;
}
```

---

## `findSymbols()`

Search for code symbols by name with filtering options.

### Signature

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

### Parameters

#### `query` (required)
- **Type**: `string`
- **Description**: Search pattern (supports SQL LIKE wildcards: `%`, `_`)
- **Matching**: Case-sensitive partial match
- **Wildcards**:
  - `%` - Match any sequence of characters
  - `_` - Match any single character
- **Examples**:
  ```typescript
  "auth"           // Contains "auth" anywhere
  "validate%"      // Starts with "validate"
  "%User"          // Ends with "User"
  "get_____"       // Exactly 9 chars starting with "get"
  ""               // Match all (use with filters and limit!)
  ```

#### `filters.kind`
- **Type**: `string[]`
- **Default**: `undefined` (all kinds)
- **Description**: Filter by symbol kind
- **Common values**:
  - `"Function"`
  - `"Method"`
  - `"Class"`
  - `"Interface"`
  - `"Variable"`
  - `"Constant"`
  - `"Type"`
  - `"Enum"`
  - `"Field"`
  - `"Module"`
  - `"Struct"`
- **Example**:
  ```typescript
  // Find only functions and methods
  agent.findSymbols("validate", { kind: ["Function", "Method"] });

  // Find only classes
  agent.findSymbols("", { kind: ["Class"], limit: 20 });
  ```

#### `filters.inFile`
- **Type**: `string`
- **Default**: `undefined` (all files)
- **Description**: Scope search to specific file
- **Must be**: Exact file path as stored in database
- **Example**:
  ```typescript
  // Find all symbols in a specific file
  agent.findSymbols("", { inFile: "src/auth/service.ts" });

  // Find all functions in a specific file
  agent.findSymbols("", {
    inFile: "src/utils/helpers.ts",
    kind: ["Function"]
  });
  ```

#### `filters.limit`
- **Type**: `number`
- **Default**: `100`
- **Description**: Maximum number of results
- **Recommendation**: Always set a reasonable limit
- **Example**:
  ```typescript
  // Get top 10 matches
  agent.findSymbols("user", { limit: 10 });

  // Get all (use carefully!)
  agent.findSymbols("", { limit: 1000 });
  ```

### Return Value

Returns `SymbolSearchResult[]` - Array of matching symbols.

Each result contains:
- `fqn` - Fully qualified name
- `name` - Short name
- `kind` - Symbol type
- `location` - Where defined (`{ file, line }`)
- `signature` - Type signature (if available)

### Examples

#### Example 1: Simple search

```typescript
const results = agent.findSymbols("authenticate");

console.log(`Found ${results.length} symbols with "authenticate":\n`);
results.forEach(r => {
  console.log(`  ${r.name} (${r.kind})`);
  console.log(`    ${r.location.file}:${r.location.line}`);
});
```

**Output**:
```
Found 3 symbols with "authenticate":

  authenticate (Function)
    src/auth/service.ts:42
  authenticateUser (Method)
    src/services/user.ts:89
  authenticate (Method)
    src/api/auth.ts:15
```

#### Example 2: Filter by kind

```typescript
// Find all classes
const classes = agent.findSymbols("", {
  kind: ["Class"],
  limit: 50
});

console.log(`Found ${classes.length} classes:\n`);
classes.forEach(cls => {
  console.log(`  - ${cls.name} in ${cls.location.file}`);
});
```

**Output**:
```
Found 15 classes:

  - UserService in src/services/user.ts
  - AuthService in src/services/auth.ts
  - Database in src/db/connection.ts
  - Logger in src/utils/logger.ts
  ...
```

#### Example 3: Scope to specific file

```typescript
const fileSymbols = agent.findSymbols("", {
  inFile: "src/auth/service.ts"
});

console.log(`Symbols in src/auth/service.ts:\n`);

// Group by kind
const byKind = fileSymbols.reduce((acc, sym) => {
  acc[sym.kind] = acc[sym.kind] || [];
  acc[sym.kind].push(sym);
  return acc;
}, {});

Object.entries(byKind).forEach(([kind, symbols]) => {
  console.log(`  ${kind} (${symbols.length}):`);
  symbols.forEach(s => console.log(`    - ${s.name}`));
});
```

**Output**:
```
Symbols in src/auth/service.ts:

  Class (1):
    - AuthService
  Method (4):
    - authenticate
    - validateToken
    - refreshToken
    - logout
  Field (2):
    - jwtSecret
    - tokenExpiry
```

#### Example 4: Pattern matching with wildcards

```typescript
// Find all "validate" functions
const validators = agent.findSymbols("validate%", {
  kind: ["Function", "Method"]
});

console.log("Validation functions:");
validators.forEach(v => console.log(`  - ${v.name}`));
```

**Output**:
```
Validation functions:
  - validateUser
  - validateEmail
  - validatePassword
  - validateCredentials
  - validateToken
```

#### Example 5: Building a symbol index

```typescript
function buildSymbolIndex() {
  const index = {
    classes: {},
    functions: {},
    types: {}
  };

  // Get all classes
  const classes = agent.findSymbols("", { kind: ["Class"], limit: 100 });
  classes.forEach(cls => {
    index.classes[cls.name] = {
      fqn: cls.fqn,
      file: cls.location.file,
      line: cls.location.line
    };
  });

  // Get all functions
  const functions = agent.findSymbols("", { kind: ["Function"], limit: 200 });
  functions.forEach(fn => {
    index.functions[fn.name] = {
      fqn: fn.fqn,
      file: fn.location.file,
      line: fn.location.line,
      signature: fn.signature
    };
  });

  // Get all types
  const types = agent.findSymbols("", {
    kind: ["Type", "Interface"],
    limit: 100
  });
  types.forEach(t => {
    index.types[t.name] = {
      fqn: t.fqn,
      file: t.location.file,
      line: t.location.line
    };
  });

  return index;
}

const index = buildSymbolIndex();
console.log("Symbol index:");
console.log(`  Classes: ${Object.keys(index.classes).length}`);
console.log(`  Functions: ${Object.keys(index.functions).length}`);
console.log(`  Types: ${Object.keys(index.types).length}`);
```

### Common Patterns

#### Pattern: "Find all test functions"
```typescript
const testFunctions = agent.findSymbols("test", {
  kind: ["Function"],
  limit: 100
});

// Or more specific
const unitTests = agent.findSymbols("test_%", {
  kind: ["Function"]
});
```

#### Pattern: "Find all constants"
```typescript
const constants = agent.findSymbols("", {
  kind: ["Constant"],
  limit: 50
});

console.log("Constants:");
constants.forEach(c => {
  console.log(`  ${c.name} = ${c.signature || '?'}`);
});
```

#### Pattern: "Explore a module"
```typescript
function exploreModule(modulePath) {
  const symbols = agent.findSymbols("", { inFile: modulePath });

  const summary = {
    total: symbols.length,
    byKind: symbols.reduce((acc, s) => {
      acc[s.kind] = (acc[s.kind] || 0) + 1;
      return acc;
    }, {})
  };

  return summary;
}

const summary = exploreModule("src/services/user.ts");
console.log(summary);
// { total: 12, byKind: { Class: 1, Method: 6, Field: 5 } }
```

---

## Type Definitions

### SymbolInfo

Complete information about a code symbol.

```typescript
interface SymbolInfo {
  // Identity
  fqn: string;                    // Fully qualified name
  name: string;                   // Short name
  kind: string;                   // Symbol type

  // Location
  location: Location;             // Where defined

  // Type information
  signature?: string;             // Type signature

  // Hierarchy
  containedIn?: string;           // Parent FQN
  contains?: string[];            // Child FQNs

  // Relationships (conditional)
  callers?: string[];             // FQNs of callers
  callees?: string[];             // FQNs of callees
  references?: Location[];        // All usages
  imports?: string[];             // Imported symbols
  importedBy?: string[];          // Importers
}
```

### SymbolSearchResult

Lightweight search result.

```typescript
interface SymbolSearchResult {
  fqn: string;
  name: string;
  kind: string;
  location: Location;
  signature?: string;
}
```

### RelationshipNode

Node in a relationship traversal.

```typescript
interface RelationshipNode {
  symbol: string;                 // FQN
  kind: string;                   // Symbol type
  location: Location;             // Where defined
  depth: number;                  // Distance from start (1, 2, 3...)
  path?: string[];                // Full path from start
}
```

### Location

File location.

```typescript
interface Location {
  file: string;                   // Relative file path
  line: number;                   // Line number (1-indexed)
  column?: number;                // Column number (0-indexed)
}
```

### RelationshipType

```typescript
type RelationshipType =
  | "calls"
  | "called-by"
  | "imports"
  | "imported-by"
  | "implements"
  | "implemented-by"
  | "contains"
  | "contained-in";
```

### Symbol Kinds

Common values for `kind` field:

- `"Function"` - Top-level function
- `"Method"` - Class method
- `"Class"` - Class definition
- `"Interface"` - Interface definition
- `"Variable"` - Variable declaration
- `"Constant"` - Constant declaration
- `"Type"` - Type alias
- `"Enum"` - Enum definition
- `"Field"` - Class field/property
- `"Module"` - Module/namespace
- `"Struct"` - Struct (Rust, Go, etc.)

---

## Error Handling

### Common Errors

#### Database not found
```typescript
try {
  const agent = new AgentCodeGraph("/wrong/path");
} catch (err) {
  console.error(err.message);
  // "Database not found at /wrong/path/.reviewbot/graph.db"
}
```

**Solution**: Ensure the repository has been scanned first.

#### Symbol not found
```typescript
const info = agent.getSymbol("nonExistent");
// Returns: null (not an error)

if (!info) {
  // Handle missing symbol
  const suggestions = agent.findSymbols("nonExistent");
  if (suggestions.length > 0) {
    console.log("Did you mean:", suggestions[0].name);
  }
}
```

#### Invalid relationship type
```typescript
try {
  agent.queryRelationships(fqn, "invalid-type" as any);
} catch (err) {
  console.error(err.message);
  // "Unknown relationship type: invalid-type"
}
```

### Best Practices

1. **Always check for null**:
   ```typescript
   const info = agent.getSymbol(fqn);
   if (!info) {
     console.log("Symbol not found");
     return;
   }
   ```

2. **Use try-catch for initialization**:
   ```typescript
   let agent;
   try {
     agent = new AgentCodeGraph(repoPath);
   } catch (err) {
     console.error("Failed to initialize:", err.message);
     process.exit(1);
   }
   ```

3. **Close connections**:
   ```typescript
   const agent = new AgentCodeGraph(repoPath);
   try {
     // Use the API
   } finally {
     agent.close();
   }
   ```

---

## Performance Considerations

### Query Performance

| Operation | Typical Time | Notes |
|-----------|-------------|-------|
| `findSymbols()` | 5-20ms | Fast, uses index |
| `getSymbol()` basic | 1-5ms | Very fast, direct lookup |
| `getSymbol()` with 1 relation | 5-15ms | Medium, one join |
| `getSymbol()` with all relations | 20-50ms | Slower, multiple joins |
| `queryRelationships()` depth=1 | 10-30ms | Fast, direct query |
| `queryRelationships()` depth=2 | 30-100ms | Medium, recursive |
| `queryRelationships()` depth=3+ | 100ms-1s+ | Slow, exponential growth |

### Optimization Tips

1. **Minimize depth**:
   ```typescript
   // ✅ Fast
   agent.queryRelationships(fqn, "calls", { depth: 1 });

   // ⚠️ Slow
   agent.queryRelationships(fqn, "calls", { depth: 5 });
   ```

2. **Use limits**:
   ```typescript
   // ✅ Good - reasonable limit
   agent.findSymbols("", { kind: ["Function"], limit: 100 });

   // ❌ Bad - potentially thousands of results
   agent.findSymbols("", { kind: ["Function"], limit: 10000 });
   ```

3. **Cache results**:
   ```typescript
   const symbolCache = new Map();

   function getCached(fqn) {
     if (!symbolCache.has(fqn)) {
       symbolCache.set(fqn, agent.getSymbol(fqn));
     }
     return symbolCache.get(fqn);
   }
   ```

4. **Batch related queries**:
   ```typescript
   // ✅ Better - one query with multiple relations
   const info = agent.getSymbol(fqn, {
     includeCallers: true,
     includeCallees: true
   });

   // ❌ Worse - multiple queries
   const info1 = agent.getSymbol(fqn, { includeCallers: true });
   const info2 = agent.getSymbol(fqn, { includeCallees: true });
   ```

5. **Filter early**:
   ```typescript
   // ✅ Better - filter in query
   agent.findSymbols("user", { kind: ["Function"], limit: 10 });

   // ❌ Worse - filter after
   agent.findSymbols("user", { limit: 100 })
     .filter(s => s.kind === "Function")
     .slice(0, 10);
   ```

### Memory Considerations

Large result sets can consume significant memory:

```typescript
// ⚠️ Could be 10MB+ with thousands of results
const all = agent.queryRelationships(fqn, "called-by", {
  depth: 5,
  limit: 10000
});

// ✅ Better - process in chunks
function processInChunks(fqn, relationship, depth, chunkSize = 100) {
  let offset = 0;
  while (true) {
    const chunk = agent.queryRelationships(fqn, relationship, {
      depth,
      limit: chunkSize
    });

    if (chunk.length === 0) break;

    // Process chunk
    chunk.forEach(node => { /* ... */ });

    offset += chunkSize;
    if (chunk.length < chunkSize) break;
  }
}
```

---

## Complete Examples

See:
- [agent-example.ts](./agent-example.ts) - Comprehensive examples
- [test-agent-api.ts](./test-agent-api.ts) - Test cases
- [AGENT_API.md](./AGENT_API.md) - Full documentation
- [AGENT_API_QUICK_REF.md](./AGENT_API_QUICK_REF.md) - Quick reference
