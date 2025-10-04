# Complete Example - Using Agent API in Another Repo

## Full TypeScript API (Index + Query)

You can now do **everything** from TypeScript - both indexing and querying.

### Example Script

```typescript
import { scanRepositorySync, AgentCodeGraph, isScanned } from "@consilium/codegraph-client";

const repoPath = "/path/to/your/project";

// Step 1: Index the repository (if not already indexed)
if (!isScanned(repoPath)) {
  console.log("📦 Indexing repository...");

  const result = scanRepositorySync(repoPath, {
    semantic: true,  // Include type information
    quiet: false     // Show progress
  });

  if (result.success) {
    console.log(`✅ Indexed in ${result.duration}ms\n`);
  } else {
    console.error(`❌ Indexing failed: ${result.error}`);
    process.exit(1);
  }
} else {
  console.log("✅ Repository already indexed\n");
}

// Step 2: Query the code graph
const agent = new AgentCodeGraph(repoPath);

// Find all functions
const functions = agent.findSymbols("", { kind: ["Function"], limit: 20 });
console.log(`Found ${functions.length} functions:\n`);

functions.forEach(fn => {
  console.log(`  - ${fn.name} at ${fn.location.file}:${fn.location.line}`);
});

// Analyze one function in detail
if (functions.length > 0) {
  const fn = functions[0];
  const info = agent.getSymbol(fn.fqn, {
    includeCallers: true,
    includeCallees: true
  });

  console.log(`\n📊 Analysis of ${info.name}:`);
  console.log(`   Calls: ${info.callees?.length || 0} functions`);
  console.log(`   Called by: ${info.callers?.length || 0} functions`);
}

agent.close();
```

---

## Installation

### 1. Install the Package

```bash
cd /path/to/your/project
npm install /path/to/consilium-codegraph/ts-client
```

### 2. Make Sure Rust CLI is Built

```bash
cd /path/to/consilium-codegraph
cargo build --release
```

That's it! The TypeScript scanner will find the Rust binary automatically.

---

## API Reference

### Scanning Functions

#### `scanRepositorySync(repoPath, options?)`

Synchronously index a repository.

```typescript
import { scanRepositorySync } from "@consilium/codegraph-client";

const result = scanRepositorySync("./my-project", {
  semantic: true,   // Include type info (recommended)
  quiet: false      // Show output
});

if (result.success) {
  console.log(`Scan completed in ${result.duration}ms`);
} else {
  console.error(`Scan failed: ${result.error}`);
}
```

**Returns**: `ScanResult`
```typescript
{
  success: boolean;
  duration: number;  // milliseconds
  output: string;    // CLI output
  error?: string;    // Error message if failed
}
```

#### `scanRepository(repoPath, options?)`

Async version (returns Promise).

```typescript
import { scanRepository } from "@consilium/codegraph-client";

const result = await scanRepository("./my-project", {
  semantic: true
});
```

#### `isScanned(repoPath)`

Check if a repository has been indexed.

```typescript
import { isScanned } from "@consilium/codegraph-client";

if (!isScanned("./my-project")) {
  console.log("Need to scan first");
}
```

**Returns**: `boolean`

#### `getCLIInfo()`

Get information about the Rust CLI.

```typescript
import { getCLIInfo } from "@consilium/codegraph-client";

const info = getCLIInfo();
if (info) {
  console.log(`CLI found at: ${info.path}`);
  console.log(`Version: ${info.version}`);
} else {
  console.log("CLI not found");
}
```

**Returns**: `{ path: string; version?: string } | null`

### Query Functions

See [AGENT_TOOLS_REFERENCE.md](ts-client/AGENT_TOOLS_REFERENCE.md) for complete documentation of:
- `AgentCodeGraph.getSymbol()`
- `AgentCodeGraph.findSymbols()`
- `AgentCodeGraph.queryRelationships()`

---

## Complete Workflow Example

```typescript
#!/usr/bin/env node
import {
  scanRepositorySync,
  isScanned,
  AgentCodeGraph
} from "@consilium/codegraph-client";

function analyzeProject(repoPath: string) {
  // 1. Index if needed
  if (!isScanned(repoPath)) {
    console.log("🔄 Indexing...");
    const result = scanRepositorySync(repoPath, { semantic: true });

    if (!result.success) {
      throw new Error(`Indexing failed: ${result.error}`);
    }

    console.log(`✅ Indexed in ${(result.duration / 1000).toFixed(1)}s\n`);
  }

  // 2. Query the graph
  const agent = new AgentCodeGraph(repoPath);

  // Find entry points
  const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 100 });

  const entryPoints = allFunctions.filter(fn => {
    const info = agent.getSymbol(fn.fqn, { includeCallers: true });
    return !info.callers || info.callers.length === 0;
  });

  console.log(`🚪 Entry points: ${entryPoints.length}`);
  entryPoints.slice(0, 5).forEach(ep => {
    console.log(`   - ${ep.name}`);
  });

  // Find most-called functions
  const withCallers = allFunctions
    .map(fn => {
      const info = agent.getSymbol(fn.fqn, { includeCallers: true });
      return { name: fn.name, callers: info.callers?.length || 0 };
    })
    .sort((a, b) => b.callers - a.callers)
    .slice(0, 5);

  console.log(`\n🔥 Most-called functions:`);
  withCallers.forEach(fn => {
    console.log(`   - ${fn.name} (${fn.callers} callers)`);
  });

  agent.close();
}

// Usage
analyzeProject(process.argv[2] || ".");
```

Save as `analyze.ts`, then:

```bash
npx tsx analyze.ts /path/to/any/project
```

---

## Re-indexing After Changes

```typescript
import { scanRepositorySync, AgentCodeGraph } from "@consilium/codegraph-client";

// Re-index to pick up code changes
console.log("🔄 Re-indexing...");
const result = scanRepositorySync(".", { semantic: true, quiet: true });

if (result.success) {
  console.log("✅ Updated");

  // Now query reflects latest code
  const agent = new AgentCodeGraph(".");
  const newFunctions = agent.findSymbols("myNewFunction");
  console.log(`Found: ${newFunctions.length}`);
  agent.close();
}
```

---

## Advanced: Custom CLI Path

If the Rust CLI is in a custom location:

```typescript
import { scanRepositorySync } from "@consilium/codegraph-client";

const result = scanRepositorySync(
  "./my-project",
  { semantic: true },
  "/custom/path/to/reviewbot"  // Custom CLI path
);
```

---

## Troubleshooting

### "Rust CLI not found"

**Problem**: `scanRepositorySync()` throws "Rust CLI not found"

**Solution**:
```bash
# Build the CLI first
cd /path/to/consilium-codegraph
cargo build --release

# Or provide custom path
const result = scanRepositorySync(
  ".",
  {},
  "/absolute/path/to/reviewbot"
);
```

### "Scan failed"

**Problem**: `result.success === false`

**Solution**: Check `result.error` and `result.output` for details:
```typescript
const result = scanRepositorySync(".");
if (!result.success) {
  console.error("Error:", result.error);
  console.error("Output:", result.output);
}
```

---

## Next Steps

- **Complete API Docs**: [AGENT_TOOLS_REFERENCE.md](ts-client/AGENT_TOOLS_REFERENCE.md)
- **Quick Reference**: [AGENT_API_QUICK_REF.md](ts-client/AGENT_API_QUICK_REF.md)
- **Examples**: [agent-example.ts](ts-client/agent-example.ts)
