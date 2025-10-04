# Getting Started with Consilium CodeGraph

Complete guide to indexing your codebase and using the TypeScript client.

## Step 1: Build the Rust CLI

First, build the Consilium CodeGraph indexer:

```bash
# From the root of the consilium-codegraph repository
cd /Users/pranavsharan/Developer/consilium-codegraph

# Build the Rust CLI
cargo build --release

# Verify it works
cargo run -- --help
```

You should see the CLI help output with commands like `scan`, `show`, `search`, etc.

## Step 2: Index Your Codebase

Now index a repository to create the code graph database:

### Option A: Index the test repository (quick test)

```bash
# Index the included test repository
cargo run -- --repo ./test_repo scan --semantic

# This creates: ./test_repo/.reviewbot/graph.db
```

### Option B: Index your own repository

```bash
# Index any repository you want to analyze
cargo run -- --repo /path/to/your/project scan --semantic

# For example:
cargo run -- --repo ~/Projects/my-app scan --semantic

# This creates: /path/to/your/project/.reviewbot/graph.db
```

**What happens during indexing:**
- ✅ Discovers all source files (TypeScript, JavaScript, Python, Go, Rust, Java, C++, C#)
- ✅ Parses files using Tree-sitter (syntactic analysis)
- ✅ Runs SCIP indexers for semantic analysis (with `--semantic` flag)
- ✅ Extracts symbols, relationships, and dependencies
- ✅ Stores everything in SQLite database (`.reviewbot/graph.db`)

**Indexing time:**
- Small repo (<10k LOC): ~5-10 seconds
- Medium repo (10k-100k LOC): ~30-60 seconds
- Large repo (>100k LOC): 1-5 minutes

### Option C: Syntactic-only indexing (faster, no semantic analysis)

```bash
# Skip semantic analysis for faster indexing
cargo run -- --repo ./test_repo scan

# This is faster but won't have cross-file type resolution
```

## Step 3: Verify the Index

Check that the database was created:

```bash
# Check if database exists
ls -lh ./test_repo/.reviewbot/graph.db

# Query the database with the CLI
cargo run -- --repo ./test_repo search "User"
cargo run -- --repo ./test_repo graph stats
```

## Step 4: Set Up the TypeScript Client

Now set up the TypeScript client to query the code graph:

```bash
# Navigate to TypeScript client directory
cd ts-client

# Install dependencies
npm install

# Build the TypeScript client
npm run build

# Verify build succeeded
ls -la dist/
```

## Step 5: Run TypeScript Functions

### Quick Test with the Built-in Example

```bash
# Make sure you're in ts-client directory
cd ts-client

# Run the comprehensive example (requires test_repo to be indexed)
node examples/basic-usage.js
```

This will run all the example functions and show you how to use the API.

### Create Your Own Script

Create a new file `my-analysis.ts`:

```typescript
import { CodeGraphAPI } from "./src/index";

// Point to your indexed repository
const api = new CodeGraphAPI("../test_repo");

console.log("=== Repository Analysis ===\n");

// Get statistics
const stats = api.getStats();
console.log(`Files: ${stats.totalFiles}`);
console.log(`Symbols: ${stats.totalSymbols}`);
console.log(`Edges: ${stats.totalEdges}`);

// Find all classes
console.log("\n=== Classes in the codebase ===");
const classes = api.findSymbols("", "class");
classes.slice(0, 5).forEach(cls => {
  console.log(`  ${cls.name} - ${cls.location.file}:${cls.location.line}`);
});

// Find all functions
console.log("\n=== Functions in the codebase ===");
const functions = api.findSymbols("", "function");
console.log(`Found ${functions.length} functions`);
functions.slice(0, 5).forEach(fn => {
  console.log(`  ${fn.name} - ${fn.location.file}:${fn.location.line}`);
});

// Analyze a specific function
if (functions.length > 0) {
  const func = functions[0];
  console.log(`\n=== Analyzing: ${func.name} ===`);

  const callers = api.getCallers(func.fqn);
  console.log(`  Called by: ${callers.length} functions`);

  const callees = api.getCallees(func.fqn);
  console.log(`  Calls: ${callees.length} functions`);

  const impact = api.getImpactRadius(func.fqn, 2);
  console.log(`  Impact radius: ${impact.size} symbols`);
}

// Check for cycles
console.log("\n=== Cycle Detection ===");
const cycles = api.findCycles();
console.log(`Found ${cycles.length} cycles in the call graph`);

// Clean up
api.close();
console.log("\n✅ Analysis complete!");
```

Run it:

```bash
# Option 1: Using tsx (TypeScript directly)
npx tsx my-analysis.ts

# Option 2: Compile first, then run
npm run build
node my-analysis.js
```

### Using the Convenience Functions

Create `quick-analysis.ts`:

```typescript
import { analyzeCodebase, findRelatedCode } from "./src/index";

// Quick codebase overview
console.log("=== Quick Codebase Analysis ===\n");
const analysis = analyzeCodebase("../test_repo");

console.log(`Total Files: ${analysis.stats.totalFiles}`);
console.log(`Total Symbols: ${analysis.stats.totalSymbols}`);
console.log(`Cycles Found: ${analysis.cycles.length}`);
console.log(`Entry Points: ${analysis.entryPoints.length}`);

console.log("\nMost Complex Functions:");
analysis.complexFunctions.slice(0, 5).forEach(func => {
  console.log(`  ${func.function} (${func.calleesCount} callees)`);
});

console.log("\nEntry Points:");
analysis.entryPoints.slice(0, 5).forEach(entry => {
  console.log(`  ${entry}`);
});

// Find related code for a specific symbol
if (analysis.entryPoints.length > 0) {
  console.log("\n=== Related Code Analysis ===\n");
  const symbol = analysis.entryPoints[0];
  const related = findRelatedCode("../test_repo", symbol);

  console.log(`Symbol: ${related.symbol}`);
  console.log(`  Callers: ${related.callers.length}`);
  console.log(`  Callees: ${related.callees.length}`);
  console.log(`  Impact: ${related.impact.length} symbols`);
}
```

Run it:

```bash
npx tsx quick-analysis.ts
```

## Step 6: Common Use Cases

### Use Case 1: Find Who Calls a Function

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("../test_repo");

// Find the function
const functions = api.findSymbols("getUserData", "function");
if (functions.length > 0) {
  const func = functions[0];
  console.log(`Function: ${func.fqn}`);
  console.log(`Location: ${func.location.file}:${func.location.line}`);

  // Get all callers
  const callers = api.getCallers(func.fqn);
  console.log(`\nCalled by ${callers.length} functions:`);
  callers.forEach(caller => {
    const callerSym = api.getSymbol(caller);
    if (callerSym) {
      console.log(`  - ${callerSym.name} (${callerSym.location.file}:${callerSym.location.line})`);
    }
  });
}

api.close();
```

### Use Case 2: Impact Analysis

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("../test_repo");

// Find what would be affected if you change a function
const functions = api.findSymbols("processData", "function");
if (functions.length > 0) {
  const func = functions[0];

  // Get impact at depth 3
  const impact = api.getImpactRadius(func.fqn, 3);

  console.log(`If you modify ${func.name}:`);
  console.log(`  ${impact.size} symbols would be affected`);
  console.log("\nAffected symbols:");
  Array.from(impact).slice(0, 10).forEach(sym => {
    console.log(`  - ${sym}`);
  });
}

api.close();
```

### Use Case 3: Find Execution Paths

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("../test_repo");

// Find all paths from function A to function B
const paths = api.findPaths("main", "databaseQuery", 10);

console.log(`Found ${paths.length} execution paths`);
paths.slice(0, 3).forEach((path, idx) => {
  console.log(`\nPath ${idx + 1}:`);
  path.forEach(sym => console.log(`  -> ${sym}`));
});

api.close();
```

### Use Case 4: Full-Featured API with Call Paths

```typescript
import { CodeGraph } from "./src/index";

const graph = new CodeGraph("../test_repo", undefined, true);

// Get detailed call paths
const functions = graph.findSymbols("handleRequest", undefined, 10);
if (functions.length > 0) {
  const func = functions[0];

  // Get call paths with depth 3
  const callers = graph.getCallers(func.fqn, 3);

  console.log(`Call paths to ${func.name}:\n`);
  callers.slice(0, 5).forEach((callPath, idx) => {
    console.log(`Path ${idx + 1} (depth: ${callPath.depth})${callPath.isRecursive ? " [RECURSIVE]" : ""}`);
    callPath.path.forEach(sym => {
      console.log(`  -> ${sym.fqn} (${sym.location.file}:${sym.location.line})`);
    });
    console.log();
  });
}

graph.close();
```

## Step 7: Running Tests

```bash
# Make sure test_repo is indexed first
cd ..
cargo run -- --repo ./test_repo scan --semantic

# Run TypeScript tests
cd ts-client
npm test

# Run with coverage
npm run test:coverage
```

## Complete Workflow Example

Here's a complete workflow from scratch:

```bash
# 1. Build the CLI
cd /Users/pranavsharan/Developer/consilium-codegraph
cargo build --release

# 2. Index your project
cargo run -- --repo ~/Projects/my-app scan --semantic

# 3. Set up TypeScript client
cd ts-client
npm install
npm run build

# 4. Create analysis script
cat > analyze-my-app.ts << 'EOF'
import { analyzeCodebase } from "./src/index";

const analysis = analyzeCodebase(process.env.HOME + "/Projects/my-app");

console.log("Codebase Analysis:");
console.log(`  Files: ${analysis.stats.totalFiles}`);
console.log(`  Symbols: ${analysis.stats.totalSymbols}`);
console.log(`  Edges: ${analysis.stats.totalEdges}`);
console.log(`  Cycles: ${analysis.cycles.length}`);

console.log("\nComplex Functions:");
analysis.complexFunctions.forEach(f => {
  console.log(`  ${f.function} (${f.calleesCount} callees)`);
});
EOF

# 5. Run it
npx tsx analyze-my-app.ts
```

## Troubleshooting

### Database not found error

**Error:** `Database not found at .reviewbot/graph.db`

**Solution:**
```bash
# Make sure you indexed the repository first
cargo run -- --repo /path/to/repo scan --semantic

# Verify database exists
ls -la /path/to/repo/.reviewbot/graph.db
```

### Module not found error

**Error:** `Cannot find module '@consilium/codegraph-client'`

**Solution:**
```bash
# Build the TypeScript client
cd ts-client
npm run build
```

### Permission errors

**Error:** `EACCES: permission denied`

**Solution:**
```bash
# Make sure you have write permissions
chmod -R u+w .reviewbot/
```

## Next Steps

- **API Reference**: See [ts-client/README.md](./ts-client/README.md)
- **Migration Guide**: See [ts-client/MIGRATION.md](./ts-client/MIGRATION.md) if coming from Python
- **More Examples**: Check [ts-client/examples/](./ts-client/examples/)
- **Testing**: See [ts-client/tests/](./ts-client/tests/)

## Quick Reference

### Essential Commands

```bash
# Index a repository
cargo run -- --repo /path/to/repo scan --semantic

# Query with CLI
cargo run -- --repo /path/to/repo search "FunctionName"
cargo run -- --repo /path/to/repo show --symbol "Class.method"
cargo run -- --repo /path/to/repo graph stats

# Use TypeScript client
cd ts-client
npx tsx your-script.ts
```

### Essential TypeScript Imports

```typescript
// Simple API
import { CodeGraphAPI } from "./src/index";

// Full API
import { CodeGraph } from "./src/index";

// Convenience functions
import { analyzeCodebase, findRelatedCode } from "./src/index";

// Types
import { Symbol, SymbolKind, Edge, EdgeType } from "./src/index";
```

Happy analyzing! 🚀
