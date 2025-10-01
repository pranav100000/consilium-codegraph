# Using Consilium CodeGraph in Your Own Repository

Quick guide to analyze any codebase with the TypeScript client.

## 🚀 Three Simple Steps

### Step 1: Index Your Repository

From the `consilium-codegraph` directory:

```bash
cargo run -- --repo /path/to/your/repo scan --semantic
```

**Examples:**

```bash
# Index a Next.js app
cargo run -- --repo ~/Projects/my-nextjs-app scan --semantic

# Index a Python project
cargo run -- --repo ~/work/backend-api scan --semantic

# Index using absolute path
cargo run -- --repo /Users/pranavsharan/Projects/my-app scan --semantic
```

**What this does:**
- Scans all source files (TS, JS, Python, Go, Rust, Java, C++, C#)
- Creates `.reviewbot/graph.db` in your repository
- Takes 10-60 seconds depending on size

### Step 2: Customize the Analysis Script

Edit the template script:

```bash
cd ts-client

# Copy the template
cp analyze-any-repo.ts analyze-my-project.ts

# Edit the REPO_PATH (line 17)
nano analyze-my-project.ts
```

Change this line:
```typescript
const REPO_PATH = process.env.REPO || join(homedir(), "Projects", "my-app");
//                                                                  ^^^^^^
//                                                         Change to your repo name
```

### Step 3: Run the Analysis

```bash
npx tsx analyze-my-project.ts
```

Or use environment variable:
```bash
REPO=~/Projects/my-app npx tsx analyze-any-repo.ts
```

## 📋 What You Get

The analysis shows:

```
📊 QUICK OVERVIEW
  📁 Files: 74
  🔤 Symbols: 579
  🔗 Relationships: 277
  🔄 Circular Dependencies: 0

📦 Symbols by Type:
  Variable       : 268
  Function       : 124
  Method         : 73
  Interface      : 56
  Class          : 31

🔗 Relationships by Type:
  Imports        : 175
  Contains       : 76
  Implements     : 24
  Calls          : 2

🔧 Most Complex Functions:
  1. processUserData
     Calls: 15 functions

🔍 Common Patterns:
  Handlers: 12 found
  Services: 8 found
  Utilities: 15 found

✅ Analysis complete!
```

## 💻 Custom Analysis Examples

### Find All React Components

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/your/nextjs-app");

// Find components
const components = api.findSymbols("", "Function")
  .filter(f => f.location.file.includes("/components/"));

console.log(`Found ${components.length} React components\n`);

components.forEach(comp => {
  const usedBy = api.getCallers(comp.fqn).length;
  console.log(`${comp.name}: used ${usedBy} times`);
});

api.close();
```

### Check Impact Before Refactoring

```typescript
import { findRelatedCode } from "./src/index";

const related = findRelatedCode(
  "/path/to/repo",
  "functionToRefactor"
);

console.log(`This function is called by: ${related.callers.length} places`);
console.log(`Changing it will affect: ${related.impact.length} symbols`);

console.log("\nYou'll need to update:");
related.callers.forEach(caller => console.log(`  - ${caller}`));
```

### Find Unused Code

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/repo");

const allFunctions = api.findSymbols("", "Function");
const unused = allFunctions.filter(fn => {
  const callers = api.getCallers(fn.fqn);
  return callers.length === 0 && !fn.name.match(/^(main|handler)/);
});

console.log(`Found ${unused.length} potentially unused functions`);
unused.forEach(fn => {
  console.log(`  ${fn.name} (${fn.location.file}:${fn.location.line})`);
});

api.close();
```

### Find Security Functions

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/repo");

const auth = api.findSymbols("auth");
const validate = api.findSymbols("validate");
const encrypt = api.findSymbols("encrypt");

console.log("Security-related functions:\n");

[...auth, ...validate, ...encrypt].forEach(fn => {
  const impact = api.getImpactRadius(fn.fqn, 2);
  console.log(`${fn.name}:`);
  console.log(`  Location: ${fn.location.file}:${fn.location.line}`);
  console.log(`  Impact: ${impact.size} symbols affected`);
});

api.close();
```

## 🔧 Complete Workflow

### Workflow 1: New Codebase Exploration

```bash
# 1. Index the codebase
cargo run -- --repo ~/Downloads/new-project scan --semantic

# 2. Quick overview
cd ts-client
REPO=~/Downloads/new-project npx tsx analyze-any-repo.ts

# 3. Find entry points
npx tsx -e "
import { analyzeCodebase } from './src/index';
const a = analyzeCodebase(process.env.HOME + '/Downloads/new-project');
console.log('Entry points:', a.entryPoints.slice(0, 10));
"

# 4. Explore from there
npx tsx -e "
import { CodeGraphAPI } from './src/index';
const api = new CodeGraphAPI(process.env.HOME + '/Downloads/new-project');
const main = api.findSymbols('main');
main.forEach(m => {
  console.log(m.name, '→', m.location.file);
  const calls = api.getCallees(m.fqn);
  console.log('  Calls:', calls.slice(0, 5));
});
api.close();
"
```

### Workflow 2: Before Making Changes

```bash
# 1. Make sure index is current
cargo run -- --repo /path/to/repo scan --semantic

# 2. Check impact
npx tsx -e "
import { findRelatedCode } from './src/index';
const r = findRelatedCode('/path/to/repo', 'functionToChange');
console.log('Callers:', r.callers.length);
console.log('Total impact:', r.impact.length, 'symbols');
console.log('\nFiles to update:');
r.callers.forEach(c => console.log('  -', c));
"

# 3. Make your changes

# 4. Re-index and verify
cargo run -- --repo /path/to/repo scan --semantic
# Run analysis again to confirm changes
```

### Workflow 3: Finding Dependencies

```bash
# 1. Index
cargo run -- --repo /path/to/repo scan --semantic

# 2. Find what uses a specific module
npx tsx -e "
import { CodeGraphAPI } from './src/index';
const api = new CodeGraphAPI('/path/to/repo');

// Find all imports of a module
const edges = api.getEdges(undefined, 'moduleName', 'Imports');
console.log('Imported by', edges.length, 'files');
edges.forEach(e => console.log('  -', e.source));

api.close();
"
```

## 📝 Common Patterns

### Pattern: One-liner Analysis

```bash
cd ts-client
REPO=/path/to/repo npx tsx -e "
const a = require('./dist/index.js').analyzeCodebase(process.env.REPO);
console.log(JSON.stringify(a.stats, null, 2));
"
```

### Pattern: Specific File Analysis

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/repo");

// Analyze a specific file
const symbols = api.getFileSymbols("src/components/Header.tsx");

console.log(`Header.tsx contains ${symbols.length} symbols:`);
symbols.forEach(s => {
  console.log(`  ${s.kind}: ${s.name} (line ${s.location.line})`);
});

api.close();
```

### Pattern: Dependency Graph

```typescript
import { CodeGraph } from "./src/index";

const graph = new CodeGraph("/path/to/repo");

const deps = graph.getDependencies("MyClass.method");

console.log("Dependencies:");
deps.dependencies[deps.root.fqn]?.forEach(d => console.log(`  → ${d}`));

console.log("\nDependents:");
deps.dependents[deps.root.fqn]?.forEach(d => console.log(`  ← ${d}`));

if (deps.cycles.length > 0) {
  console.log("\n⚠️  Circular dependencies detected!");
}

graph.close();
```

## 🎯 Quick Reference

### Index a Repository
```bash
cargo run -- --repo /path/to/repo scan --semantic
```

### Run Analysis
```bash
cd ts-client
REPO=/path/to/repo npx tsx analyze-any-repo.ts
```

### Quick Query
```bash
npx tsx -e "
import { CodeGraphAPI } from './src/index';
const api = new CodeGraphAPI('/path/to/repo');
// Your code here
api.close();
"
```

### Find Symbols
```typescript
api.findSymbols("User")           // Find by name
api.findSymbols("", "Class")      // Find by type
api.getFileSymbols("src/app.ts")  // Get all in file
```

### Check Relationships
```typescript
api.getCallers("function")        // Who calls this?
api.getCallees("function")        // What does this call?
api.getImpactRadius("symbol", 3)  // Impact analysis
```

## 💡 Tips

1. **Always use absolute paths** or `process.env.HOME + '/relative/path'`
2. **Re-index after changes** to get updated analysis
3. **Use environment variables** for flexibility: `REPO=/path npx tsx script.ts`
4. **Start simple** with `analyze-any-repo.ts` then customize
5. **Check database exists** at `/path/to/repo/.reviewbot/graph.db`

## 🆘 Troubleshooting

### "Database not found"
```bash
# Solution: Index the repo first
cargo run -- --repo /path/to/repo scan --semantic
```

### "No results returned"
```bash
# Check the path is correct
ls /path/to/repo/.reviewbot/graph.db

# Re-index if needed
cargo run -- --repo /path/to/repo scan --semantic
```

### "Module not found"
```bash
# Make sure you're in ts-client directory
cd ts-client

# And dependencies are installed
npm install
```

## 📚 More Resources

- **Full API Docs**: [ts-client/README.md](./ts-client/README.md)
- **Detailed Guide**: [HOW_TO_USE.md](./HOW_TO_USE.md)
- **Quick Reference**: [QUICK_REFERENCE.md](./QUICK_REFERENCE.md)
- **Examples**: [ts-client/examples/](./ts-client/examples/)

## 🎉 You're Ready!

1. Index your repo: `cargo run -- --repo /path scan --semantic`
2. Edit the script: Change `REPO_PATH` in `analyze-any-repo.ts`
3. Run it: `npx tsx analyze-any-repo.ts`
4. Customize: Add your own queries and analysis

Happy analyzing! 🚀
