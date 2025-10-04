# How to Use Consilium CodeGraph in Another Repository

Complete guide to analyzing any codebase with the TypeScript client.

## Quick Start (3 Steps)

### Step 1: Index Your Repository

```bash
# Navigate to the consilium-codegraph directory
cd /Users/pranavsharan/Developer/consilium-codegraph

# Index your target repository
cargo run -- --repo /path/to/your/repo scan --semantic

# Example: Index a project in your home directory
cargo run -- --repo ~/Projects/my-app scan --semantic

# Or use an absolute path
cargo run -- --repo /Users/pranavsharan/Projects/my-nextjs-app scan --semantic
```

**What happens:**
- Scans all TypeScript, JavaScript, Python, Go, Rust, Java, C++, C# files
- Creates `.reviewbot/graph.db` in your target repository
- Takes 10-60 seconds depending on repo size

### Step 2: Create Your Analysis Script

```bash
# Navigate to ts-client
cd ts-client

# Create a new analysis script
nano analyze-my-project.ts
```

**Simple Example:**

```typescript
import { CodeGraphAPI, analyzeCodebase } from "./src/index";

// Replace with your repo path
const MY_REPO = "/Users/pranavsharan/Projects/my-app";

// Quick analysis
console.log("=== Quick Analysis ===\n");
const analysis = analyzeCodebase(MY_REPO);

console.log(`Files: ${analysis.stats.totalFiles}`);
console.log(`Symbols: ${analysis.stats.totalSymbols}`);
console.log(`Edges: ${analysis.stats.totalEdges}`);
console.log(`Cycles: ${analysis.cycles.length}`);

console.log("\nMost Complex Functions:");
analysis.complexFunctions.slice(0, 5).forEach(f => {
  console.log(`  ${f.function} (${f.calleesCount} calls)`);
});

// Detailed analysis
console.log("\n=== Detailed Analysis ===\n");
const api = new CodeGraphAPI(MY_REPO);

// Find all React components
const components = api.findSymbols("", "function");
console.log(`Found ${components.length} functions`);

// Show some examples
components.slice(0, 5).forEach(c => {
  console.log(`  - ${c.name} (${c.location.file}:${c.location.line})`);
});

api.close();
```

### Step 3: Run Your Analysis

```bash
# Make sure dependencies are installed
npm install

# Run your script
npx tsx analyze-my-project.ts
```

## Real-World Examples

### Example 1: Analyze a Next.js Application

```bash
# Step 1: Index the repo
cargo run -- --repo ~/Projects/my-nextjs-app scan --semantic

# Step 2: Create script
cat > ts-client/analyze-nextjs.ts << 'EOF'
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI(process.env.HOME + "/Projects/my-nextjs-app");

// Find all React components
const components = api.findSymbols("", "function")
  .filter(f => f.location.file.includes("/components/"));

console.log(`Found ${components.length} components in /components/\n`);

// Find which components are most connected
components.forEach(comp => {
  const callers = api.getCallers(comp.fqn);
  const callees = api.getCallees(comp.fqn);

  if (callers.length > 3 || callees.length > 5) {
    console.log(`${comp.name}:`);
    console.log(`  Used by: ${callers.length} components`);
    console.log(`  Uses: ${callees.length} components`);
  }
});

api.close();
EOF

# Step 3: Run it
cd ts-client && npx tsx analyze-nextjs.ts
```

### Example 2: Find Security-Critical Functions

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/your/repo");

// Find functions that might handle authentication
const authFunctions = api.findSymbols("auth");
const loginFunctions = api.findSymbols("login");
const validateFunctions = api.findSymbols("validate");

console.log("=== Security-Critical Functions ===\n");

[...authFunctions, ...loginFunctions, ...validateFunctions].forEach(fn => {
  console.log(`${fn.name} (${fn.location.file}:${fn.location.line})`);

  const callers = api.getCallers(fn.fqn);
  console.log(`  Called by ${callers.length} functions`);

  const callees = api.getCallees(fn.fqn);
  console.log(`  Calls ${callees.length} functions`);

  // Check impact if this function changes
  const impact = api.getImpactRadius(fn.fqn, 2);
  console.log(`  Impact: ${impact.size} symbols affected\n`);
});

api.close();
```

### Example 3: Refactoring Helper

```typescript
import { CodeGraphAPI, findRelatedCode } from "./src/index";

const REPO = "/path/to/your/repo";
const FUNCTION_TO_REFACTOR = "calculateUserScore";

// Find all related code before refactoring
const related = findRelatedCode(REPO, FUNCTION_TO_REFACTOR);

console.log(`=== Refactoring Impact for ${FUNCTION_TO_REFACTOR} ===\n`);

console.log(`Direct callers (need to update): ${related.callers.length}`);
related.callers.forEach(caller => console.log(`  - ${caller}`));

console.log(`\nFunctions called (might need to change): ${related.callees.length}`);
related.callees.forEach(callee => console.log(`  - ${callee}`));

console.log(`\nTotal impact radius: ${related.impact.length} symbols`);
console.log("\nRecommendation: Update tests for all callers above");
```

### Example 4: Dependency Analysis

```typescript
import { CodeGraph } from "./src/index";

const graph = new CodeGraph("/path/to/your/repo", undefined, true);

// Find circular dependencies
const symbol = "MyService.processData";
const deps = graph.getDependencies(symbol);

console.log(`=== Dependencies for ${symbol} ===\n`);

console.log(`Direct dependencies: ${deps.dependencies[symbol]?.length || 0}`);
deps.dependencies[symbol]?.forEach(dep => console.log(`  → ${dep}`));

console.log(`\nDirect dependents: ${deps.dependents[symbol]?.length || 0}`);
deps.dependents[symbol]?.forEach(dep => console.log(`  ← ${dep}`));

if (deps.cycles.length > 0) {
  console.log(`\n⚠️  WARNING: Circular dependencies detected!`);
  deps.cycles.forEach((cycle, i) => {
    console.log(`\nCycle ${i + 1}:`);
    cycle.forEach(sym => console.log(`  → ${sym}`));
  });
}

graph.close();
```

## Using from Your Own Project

Instead of running scripts from `ts-client`, you can use the client as a library:

### Option 1: Link Locally

```bash
# In ts-client directory
npm link

# In your project directory
npm link @consilium/codegraph-client

# Then in your project's code
import { CodeGraphAPI } from "@consilium/codegraph-client";
```

### Option 2: Copy and Use Directly

```bash
# Copy the compiled files to your project
cp -r /Users/pranavsharan/Developer/consilium-codegraph/ts-client/dist ./codegraph-client
cp -r /Users/pranavsharan/Developer/consilium-codegraph/ts-client/node_modules ./codegraph-client/

# In your code
const { CodeGraphAPI } = require('./codegraph-client/index.js');
```

### Option 3: Use in Node.js Script

Create `analyze.js` in your project:

```javascript
// analyze.js
const { CodeGraphAPI } = require('/Users/pranavsharan/Developer/consilium-codegraph/ts-client/dist/index.js');

const api = new CodeGraphAPI('.');  // Current directory

const stats = api.getStats();
console.log('Repository Statistics:');
console.log(`  Files: ${stats.totalFiles}`);
console.log(`  Symbols: ${stats.totalSymbols}`);

api.close();
```

Run it:
```bash
node analyze.js
```

## Integration Patterns

### Pattern 1: CI/CD Check

```typescript
// scripts/check-complexity.ts
import { analyzeCodebase } from "./src/index";

const analysis = analyzeCodebase(process.cwd());

// Fail if we have too many complex functions
if (analysis.complexFunctions.length > 10) {
  console.error(`❌ Too many complex functions: ${analysis.complexFunctions.length}`);
  process.exit(1);
}

// Fail if we have circular dependencies
if (analysis.cycles.length > 0) {
  console.error(`❌ Circular dependencies detected: ${analysis.cycles.length}`);
  process.exit(1);
}

console.log("✅ Code complexity checks passed");
```

### Pattern 2: Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Index the repository
cd /Users/pranavsharan/Developer/consilium-codegraph
cargo run -- --repo $PROJECT_DIR scan --semantic

# Check for issues
cd ts-client
npx tsx check-complexity.ts
```

### Pattern 3: Documentation Generator

```typescript
import { CodeGraphAPI } from "./src/index";
import * as fs from 'fs';

const api = new CodeGraphAPI(process.cwd());

let markdown = "# API Documentation\n\n";

// Document all exported functions
const functions = api.findSymbols("", "function");
functions.forEach(fn => {
  markdown += `## ${fn.name}\n\n`;
  markdown += `Location: \`${fn.location.file}:${fn.location.line}\`\n\n`;

  const callers = api.getCallers(fn.fqn);
  if (callers.length > 0) {
    markdown += `Used by: ${callers.length} functions\n\n`;
  }
});

fs.writeFileSync('API.md', markdown);
api.close();
```

## Common Workflows

### Workflow 1: Understanding a New Codebase

```bash
# 1. Index the codebase
cargo run -- --repo /path/to/new/repo scan --semantic

# 2. Get overview
cd ts-client
npx tsx -e "
import { analyzeCodebase } from './src/index';
const a = analyzeCodebase('/path/to/new/repo');
console.log('Files:', a.stats.totalFiles);
console.log('Languages:', Object.keys(a.stats.symbolsByKind));
console.log('Entry points:', a.entryPoints.slice(0, 5));
"

# 3. Find main components
npx tsx -e "
import { CodeGraphAPI } from './src/index';
const api = new CodeGraphAPI('/path/to/new/repo');
const mains = api.findSymbols('main');
mains.forEach(m => console.log(m.name, m.location.file));
api.close();
"
```

### Workflow 2: Impact Analysis Before Refactoring

```bash
# 1. Re-index to get latest state
cargo run -- --repo /path/to/repo scan --semantic

# 2. Check impact of changing a function
npx tsx -e "
import { findRelatedCode } from './src/index';
const related = findRelatedCode('/path/to/repo', 'functionToChange');
console.log('Will affect:', related.impact.length, 'symbols');
console.log('Update these callers:', related.callers);
"
```

### Workflow 3: Finding Dead Code

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/repo");

const allFunctions = api.findSymbols("", "function");
const deadCode = [];

allFunctions.forEach(fn => {
  const callers = api.getCallers(fn.fqn);

  // If nobody calls it and it's not an entry point
  if (callers.length === 0 && !fn.name.match(/^(main|handler|default)/)) {
    deadCode.push(fn);
  }
});

console.log(`Found ${deadCode.length} potentially unused functions:`);
deadCode.forEach(fn => {
  console.log(`  ${fn.name} (${fn.location.file}:${fn.location.line})`);
});

api.close();
```

## Environment Variables

You can use environment variables for flexibility:

```typescript
import { CodeGraphAPI } from "./src/index";

const REPO_PATH = process.env.REPO_PATH || process.cwd();
const api = new CodeGraphAPI(REPO_PATH);

// Your analysis code...

api.close();
```

Then run:
```bash
REPO_PATH=/path/to/repo npx tsx my-script.ts
```

## Troubleshooting

### Database Not Found

**Error:** `Database not found at .reviewbot/graph.db`

**Solution:** Index the repository first:
```bash
cargo run -- --repo /path/to/repo scan --semantic
```

### Wrong Path

**Error:** Database exists but queries return no results

**Solution:** Use absolute paths:
```typescript
// ❌ Wrong
const api = new CodeGraphAPI("../my-repo");

// ✅ Correct
const api = new CodeGraphAPI("/Users/pranavsharan/Projects/my-repo");

// ✅ Also correct
const api = new CodeGraphAPI(process.env.HOME + "/Projects/my-repo");
```

### Re-indexing After Changes

If you've made code changes and want to re-analyze:

```bash
# Re-index the repository
cargo run -- --repo /path/to/repo scan --semantic

# The database will be updated with the latest code
```

## Tips & Best Practices

1. **Use Absolute Paths**: Always use absolute paths for repositories
2. **Re-index After Changes**: Re-run scan after major code changes
3. **Close Connections**: Always call `api.close()` when done
4. **Check Database Exists**: Verify `.reviewbot/graph.db` exists before querying
5. **Use Semantic Analysis**: Include `--semantic` flag for best results
6. **Cache Results**: Store query results if you'll reuse them
7. **Limit Query Depth**: Use smaller depth values (1-3) for faster queries

## Next Steps

- **Explore Examples**: Check `ts-client/examples/basic-usage.ts`
- **Read API Docs**: See `ts-client/README.md` for all available methods
- **Build Tools**: Create custom analysis tools for your workflow
- **Automate**: Integrate into CI/CD pipelines

## Quick Reference

```bash
# Index a repository
cargo run -- --repo /path/to/repo scan --semantic

# Quick analysis from command line
cd ts-client && npx tsx -e "
import { analyzeCodebase } from './src/index';
const a = analyzeCodebase('/path/to/repo');
console.log(a.stats);
"

# Run a script
npx tsx your-script.ts

# With environment variable
REPO=/path/to/repo npx tsx your-script.ts
```

Happy analyzing! 🚀
