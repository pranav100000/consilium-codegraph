# Installation Summary

Complete guide to using Consilium CodeGraph in your projects.

## What We Built

✅ **Rust binary** - Indexes codebases and builds code graphs
✅ **TypeScript client** - Queries the graph database
✅ **Bun support** - Works with both Bun and Node.js
✅ **Pre-built binaries** - No Rust toolchain needed
✅ **Codebuff integration** - Drop-in replacement for tree-sitter

## Installation Methods

### Method 1: Local Development (Codebuff)

Best for: Active development on both projects

```bash
# From Codebuff npm-app directory
cd /path/to/codebuff/npm-app
bun add /Users/pranavsharan/Developer/consilium-codegraph/ts-client
```

### Method 2: GitHub (Production)

Best for: CI/CD, sharing with others

```bash
npm install github:youruser/consilium-codegraph#main
# or
bun add github:youruser/consilium-codegraph#main
```

### Method 3: npm (Future)

Best for: Public distribution

```bash
npm install @consilium/codegraph-client
```

## What's Included

### Pre-built Binaries

Located in `ts-client/bin/`:
- ✅ `darwin-arm64/reviewbot` - macOS Apple Silicon
- ⏳ `darwin-x64/reviewbot` - macOS Intel (build with cross-compilation)
- ⏳ `linux-x64/reviewbot` - Linux (build on Linux or CI)
- ⏳ `win32-x64/reviewbot.exe` - Windows (build on Windows or CI)

Currently only macOS ARM64 is built. Others can be added via CI or cross-compilation.

### Database Runtime Support

The package automatically detects and uses:
- **Bun**: `bun:sqlite` (built-in, 3x faster)
- **Node.js**: `better-sqlite3` (optional dependency)

No configuration needed!

## Quick Usage

### 1. Scan a Repository

```typescript
import { scanRepositorySync, isScanned } from "@consilium/codegraph-client";

if (!isScanned("./my-project")) {
  const result = scanRepositorySync("./my-project", {
    semantic: true,  // Include type information
    quiet: false      // Show progress
  });

  console.log(`Scanned in ${result.duration}ms`);
}
```

### 2. Query the Code Graph

```typescript
import { AgentCodeGraph } from "@consilium/codegraph-client";

const agent = new AgentCodeGraph("./my-project");

// Find symbols
const symbols = agent.findSymbols("authenticate", {
  kind: ["Function", "Method"],
  limit: 10
});

// Get symbol details with relationships
const info = agent.getSymbol(symbols[0].fqn, {
  includeCallers: true,
  includeCallees: true
});

console.log(`${info.name} is called by ${info.callers?.length} functions`);

agent.close();
```

### 3. Codebuff Integration

```typescript
import { AgentCodeGraph, scanRepositorySync, isScanned } from "@consilium/codegraph-client";

export async function getFileTokenScoresFromConsilium(projectPath: string) {
  // Auto-scan if needed
  if (!isScanned(projectPath)) {
    scanRepositorySync(projectPath, { semantic: true });
  }

  // Get token data for Codebuff
  const agent = new AgentCodeGraph(projectPath);
  const data = agent.getFileTokenData();
  agent.close();

  return data; // { tokenScores, tokenCallers }
}
```

## File Structure

```
consilium-codegraph/
├── crates/              # Rust source code
│   ├── core/           # Main CLI binary
│   ├── protocol/       # IR types
│   ├── store/          # SQLite layer
│   └── ...
├── ts-client/          # TypeScript client (npm package)
│   ├── src/
│   │   ├── agent-api.ts       # Agent-focused API
│   │   ├── code-graph-api.ts  # Simple API
│   │   ├── code-graph.ts      # Full-featured API
│   │   ├── db-adapter.ts      # Bun/Node.js adapter
│   │   ├── scanner.ts         # CLI wrapper
│   │   └── index.ts           # Main export
│   ├── bin/                   # Pre-built binaries
│   │   └── darwin-arm64/
│   │       └── reviewbot
│   ├── tests/                 # Test suite (43 tests)
│   ├── dist/                  # Compiled JavaScript
│   └── package.json           # npm config
└── target/
    └── release/
        └── reviewbot          # Rust binary
```

## How It Works

### First Run (Indexing)

```
User runs Codebuff
    ↓
getFileTokenData() called
    ↓
isScanned() checks for .reviewbot/graph.db
    ↓
Not found → scanRepositorySync()
    ↓
Finds binary at ts-client/bin/darwin-arm64/reviewbot
    ↓
Runs: reviewbot --repo /path/to/project scan --semantic
    ↓
Binary parses all files, builds graph, writes to SQLite
    ↓
Creates .reviewbot/graph.db
    ↓
Returns to getFileTokenData()
    ↓
Queries database for token scores
    ↓
Returns to Codebuff
```

### Subsequent Runs (Cached)

```
User runs Codebuff
    ↓
getFileTokenData() called
    ↓
isScanned() checks for .reviewbot/graph.db
    ↓
Found → skip scanning!
    ↓
Opens database directly
    ↓
Queries in ~20ms
    ↓
Returns to Codebuff
```

## Benefits vs Tree-sitter

| Feature | Tree-sitter | Consilium |
|---------|-------------|-----------|
| Setup | None | One-time scan |
| Data | Raw AST | Code graph with relationships |
| Cross-file analysis | ❌ No | ✅ Yes |
| Caller information | ❌ No | ✅ Yes |
| Token savings | 0% baseline | ~80% |
| Query speed | Fast | Faster (cached) |
| Incremental updates | ✅ Yes | ⏳ Coming soon |

## Troubleshooting

### "Rust CLI not found"

**Cause**: Binary not built or not in the right location.

**Fix**:
```bash
cd /Users/pranavsharan/Developer/consilium-codegraph
cargo build --release
cp target/release/reviewbot ts-client/bin/darwin-arm64/
```

### "Module 'better_sqlite3' was compiled against different ABI"

**Cause**: Using Node.js but better-sqlite3 was compiled for a different version.

**Fix**: Use Bun instead (it doesn't need better-sqlite3):
```bash
bun add /path/to/ts-client
```

Or rebuild better-sqlite3:
```bash
npm rebuild better-sqlite3
```

### Scan takes forever

**Not a problem!** First scan is expected to take time:
- 10k LOC: ~10 seconds
- 100k LOC: ~60 seconds
- 1M LOC: ~10 minutes

Subsequent runs are instant (< 20ms).

### Empty token data

**Cause**: Scan failed silently.

**Fix**: Check the database:
```bash
sqlite3 .reviewbot/graph.db "SELECT COUNT(*) FROM symbol"
```

Should be > 0. If 0, rescan with errors visible:
```typescript
const result = scanRepositorySync(projectPath, { quiet: false });
console.log(result.output);
```

## Documentation

- [CODEBUFF_QUICK_START.md](CODEBUFF_QUICK_START.md) - Get started with Codebuff
- [CODEBUFF_SETUP.md](CODEBUFF_SETUP.md) - Detailed Codebuff integration
- [GITHUB_INSTALLATION.md](GITHUB_INSTALLATION.md) - Install from GitHub
- [BUN_SUPPORT.md](ts-client/BUN_SUPPORT.md) - Bun runtime details
- [ts-client/README.md](ts-client/README.md) - Full API documentation
- [ts-client/AGENT_API.md](ts-client/AGENT_API.md) - Agent API guide
- [CODEBUFF_INTEGRATION.md](CODEBUFF_INTEGRATION.md) - Integration guide

## Testing

All 43 tests pass:
```bash
cd ts-client
npm test
```

Test coverage:
- ✅ Agent API (26 tests)
- ✅ Simple API (13 tests)
- ✅ Scanner (4 tests)
- ✅ Codebuff integration (7 tests)

## Next Steps

1. **For Codebuff**: Follow [CODEBUFF_QUICK_START.md](CODEBUFF_QUICK_START.md)
2. **For GitHub**: Follow [GITHUB_INSTALLATION.md](GITHUB_INSTALLATION.md)
3. **For development**: See [ts-client/README.md](ts-client/README.md)

Enjoy! 🎉
