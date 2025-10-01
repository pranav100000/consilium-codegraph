# Quick Start Guide

Get started with the Consilium CodeGraph TypeScript client in under 5 minutes.

## Prerequisites

1. **Build the code graph** first using the Rust CLI:
   ```bash
   # From the root of the consilium-codegraph repo
   cargo run -- --repo /path/to/your/repo scan --semantic
   ```

2. **Node.js 18+** installed on your system

## Installation

### Option 1: Use locally (recommended for development)

```bash
cd ts-client
npm install
npm run build
```

### Option 2: Publish to NPM (for production use)

```bash
cd ts-client
npm publish
# Then in your project:
npm install @consilium/codegraph-client
```

## Basic Usage

Create a file `example.ts`:

```typescript
import { CodeGraphAPI } from "./src";

// Initialize the API
const api = new CodeGraphAPI("/path/to/your/repo");

// Find symbols
const symbols = api.findSymbols("User");
console.log(`Found ${symbols.length} symbols matching "User"`);

// Show first few matches
symbols.slice(0, 3).forEach(sym => {
  console.log(`  - ${sym.name} (${sym.kind})`);
  console.log(`    Location: ${sym.location.file}:${sym.location.line}`);
});

// Get statistics
const stats = api.getStats();
console.log(`\nRepository Statistics:`);
console.log(`  Files: ${stats.totalFiles}`);
console.log(`  Symbols: ${stats.totalSymbols}`);
console.log(`  Edges: ${stats.totalEdges}`);

// Clean up
api.close();
```

Run it:
```bash
npx tsx example.ts
# or compile first:
npm run build
node dist/example.js
```

## Common Operations

### Find and Analyze a Function

```typescript
import { CodeGraphAPI } from "@consilium/codegraph-client";

const api = new CodeGraphAPI("./my-repo");

// Find a function
const functions = api.findSymbols("processData", "function");
if (functions.length > 0) {
  const func = functions[0];

  // Get its callers
  const callers = api.getCallers(func.fqn);
  console.log(`Called by ${callers.length} functions`);

  // Get its callees
  const callees = api.getCallees(func.fqn);
  console.log(`Calls ${callees.length} functions`);

  // Impact analysis
  const impact = api.getImpactRadius(func.fqn, 3);
  console.log(`Changing this would affect ${impact.size} symbols`);
}

api.close();
```

### Quick Codebase Analysis

```typescript
import { analyzeCodebase } from "@consilium/codegraph-client";

const analysis = analyzeCodebase("./my-repo");

console.log(`Total Files: ${analysis.stats.totalFiles}`);
console.log(`Total Symbols: ${analysis.stats.totalSymbols}`);
console.log(`Cycles Found: ${analysis.cycles.length}`);
console.log(`Entry Points: ${analysis.entryPoints.length}`);

console.log("\nMost Complex Functions:");
analysis.complexFunctions.forEach(f => {
  console.log(`  ${f.function} (${f.calleesCount} callees)`);
});
```

### Find Related Code

```typescript
import { findRelatedCode } from "@consilium/codegraph-client";

const related = findRelatedCode("./my-repo", "UserService.login");

console.log(`Functions that call this: ${related.callers.length}`);
console.log(`Functions this calls: ${related.callees.length}`);
console.log(`Total impact: ${related.impact.length} symbols`);
```

## Running the Examples

```bash
# Build first
npm run build

# Run the example file
node examples/basic-usage.js

# Or use tsx to run TypeScript directly
npx tsx examples/basic-usage.ts
```

## Testing

```bash
# Run tests (requires test_repo to be scanned)
npm test

# Run with coverage
npm run test:coverage
```

## Common Issues

### Database not found

**Error:** `Database not found at .reviewbot/graph.db`

**Solution:** Run the scan first:
```bash
cargo run -- --repo /path/to/repo scan --semantic
```

### Module not found

**Error:** `Cannot find module '@consilium/codegraph-client'`

**Solution:** Make sure you've built the project:
```bash
npm run build
```

### TypeScript errors

**Error:** TypeScript compilation errors

**Solution:** Make sure you have the correct TypeScript version:
```bash
npm install
npm run build
```

## Next Steps

- Read the [README.md](./README.md) for complete API documentation
- Check [MIGRATION.md](./MIGRATION.md) if migrating from Python
- Browse [examples/](./examples/) for more use cases
- Review [tests/](./tests/) for testing patterns

## Need Help?

- **API Documentation**: See [README.md](./README.md)
- **Migration Guide**: See [MIGRATION.md](./MIGRATION.md)
- **Examples**: See [examples/basic-usage.ts](./examples/basic-usage.ts)
- **Tests**: See [tests/](./tests/)
