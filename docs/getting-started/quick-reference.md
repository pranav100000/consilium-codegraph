# Quick Reference Card

One-page cheat sheet for Consilium CodeGraph.

## 🚀 Quick Start (3 Commands)

```bash
# 1. Index your codebase
cargo run -- --repo /path/to/repo scan --semantic

# 2. Set up TypeScript client
cd ts-client && npm install && npm run build

# 3. Run the demo
./run-demo.sh
```

## 📋 Essential Commands

### Indexing (Rust CLI)

```bash
# Basic scan (syntactic only, fast)
cargo run -- --repo ./my-repo scan

# Full scan (with semantic analysis, recommended)
cargo run -- --repo ./my-repo scan --semantic

# Incremental scan (only changed files)
cargo run -- --repo ./my-repo scan --incremental --semantic

# Query with CLI
cargo run -- --repo ./my-repo search "FunctionName"
cargo run -- --repo ./my-repo show --symbol "Class.method"
cargo run -- --repo ./my-repo graph stats
```

### TypeScript Client Setup

```bash
cd ts-client
npm install          # Install dependencies
npm run build        # Compile TypeScript
npm test             # Run tests
./run-demo.sh        # Run demo script
```

## 💻 TypeScript API Examples

### Basic Usage

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/repo");

// Search symbols
const symbols = api.findSymbols("User");

// Get symbol details
const symbol = api.getSymbol("UserService.authenticate");

// Find callers/callees
const callers = api.getCallers("MyClass.method");
const callees = api.getCallees("MyClass.method");

// Get statistics
const stats = api.getStats();
console.log(`Files: ${stats.totalFiles}, Symbols: ${stats.totalSymbols}`);

api.close();
```

### Quick Analysis

```typescript
import { analyzeCodebase } from "./src/index";

const analysis = analyzeCodebase("/path/to/repo");
console.log(`Files: ${analysis.stats.totalFiles}`);
console.log(`Cycles: ${analysis.cycles.length}`);
console.log(`Complex: ${analysis.complexFunctions.length}`);
```

### Impact Analysis

```typescript
import { CodeGraphAPI } from "./src/index";

const api = new CodeGraphAPI("/path/to/repo");
const impact = api.getImpactRadius("functionName", 3);
console.log(`${impact.size} symbols would be affected`);
api.close();
```

### Find Related Code

```typescript
import { findRelatedCode } from "./src/index";

const related = findRelatedCode("/path/to/repo", "UserService.login");
console.log(`Callers: ${related.callers.length}`);
console.log(`Impact: ${related.impact.length}`);
```

## 🔍 Common Queries

| What | How |
|------|-----|
| Find all classes | `api.findSymbols("", "class")` |
| Find all functions | `api.findSymbols("", "function")` |
| Find by name | `api.findSymbols("User")` |
| Get file symbols | `api.getFileSymbols("src/user.ts")` |
| Who calls this? | `api.getCallers("func")` |
| What does this call? | `api.getCallees("func")` |
| Find paths | `api.findPaths("main", "query", 10)` |
| Check cycles | `api.findCycles()` |
| Get dependencies | `api.getDependencies("symbol")` |
| Impact analysis | `api.getImpactRadius("symbol", 3)` |

## 📁 File Locations

```
consilium-codegraph/
├── cargo run -- ...           # Run Rust CLI
├── ts-client/
│   ├── src/                   # TypeScript source
│   ├── dist/                  # Compiled output
│   ├── examples/              # Example scripts
│   ├── tests/                 # Test suite
│   ├── analyze-test-repo.ts   # Demo script
│   ├── run-demo.sh           # Quick demo
│   └── README.md             # Full docs
└── test_repo/
    └── .reviewbot/
        └── graph.db          # Database (after indexing)
```

## 🎯 Symbol Types (SymbolKind)

- `function` - Functions
- `method` - Class methods
- `class` - Classes
- `interface` - Interfaces
- `variable` - Variables
- `type` - Type definitions
- `module` - Modules
- `enum` - Enumerations
- `constant` - Constants
- `property` - Class properties

## 🔗 Edge Types (EdgeType)

- `calls` - Function calls
- `imports` - Import statements
- `uses` - Variable usage
- `extends` - Class inheritance
- `implements` - Interface implementation
- `contains` - Containment
- `returns` - Return type
- `throws` - Exception handling

## 📊 API Classes

### CodeGraphAPI (Simple)
Best for: Quick queries, common operations
```typescript
import { CodeGraphAPI } from "./src/index";
const api = new CodeGraphAPI("/path");
```

### CodeGraph (Full)
Best for: Advanced analysis, call path traversal
```typescript
import { CodeGraph } from "./src/index";
const graph = new CodeGraph("/path", undefined, true);
const paths = graph.getCallers("func", 3); // depth 3
```

### Convenience Functions
Best for: Quick analysis, one-off scripts
```typescript
import { analyzeCodebase, findRelatedCode } from "./src/index";
```

## ⚡ Performance Tips

1. **Use syntactic-only for speed**: `scan` instead of `scan --semantic`
2. **Use incremental scans**: `scan --incremental` for changed files
3. **Limit depth**: Use smaller depth values in traversal
4. **Close connections**: Always call `api.close()`
5. **Cache results**: Store query results if reusing

## 🐛 Troubleshooting

| Problem | Solution |
|---------|----------|
| Database not found | Run `cargo run -- --repo /path scan --semantic` |
| Module not found | Run `npm run build` in ts-client/ |
| Permission denied | `chmod -R u+w .reviewbot/` |
| Tests failing | Ensure test_repo is indexed first |
| Out of memory | Reduce batch size or depth |

## 📚 Documentation

- **Getting Started**: [GETTING_STARTED.md](./GETTING_STARTED.md)
- **Full API Docs**: [ts-client/README.md](./ts-client/README.md)
- **Migration Guide**: [ts-client/MIGRATION.md](./ts-client/MIGRATION.md)
- **TypeScript Overview**: [TYPESCRIPT_CLIENT.md](./TYPESCRIPT_CLIENT.md)

## 🎓 Learning Path

1. ✅ Index test_repo: `cargo run -- --repo ./test_repo scan --semantic`
2. ✅ Run demo: `cd ts-client && ./run-demo.sh`
3. ✅ Read examples: `cat examples/basic-usage.ts`
4. ✅ Create your script: `npx tsx my-script.ts`
5. ✅ Read full docs: `ts-client/README.md`

## 💡 Tips

- Always index with `--semantic` for best results
- Use `analyzeCodebase()` for quick insights
- Check for cycles in complex projects
- Use impact radius before refactoring
- Test queries on test_repo first

---

**Need help?** See [GETTING_STARTED.md](./GETTING_STARTED.md) for detailed walkthrough.
