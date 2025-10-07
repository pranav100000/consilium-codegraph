# Using Consilium in Codebuff

This guide shows how to use Consilium CodeGraph in your Codebuff installation.

## Setup (One-Time)

### 1. Ensure Binary is Built

From the `consilium-codegraph` repo:

```bash
cargo build --release
```

This creates `target/release/reviewbot` which is already copied to `ts-client/bin/darwin-arm64/`.

### 2. Install in Codebuff

From your Codebuff repo:

```bash
cd npm-app
npm install /path/to/consilium-codegraph/ts-client
```

Or add to `package.json`:

```json
{
  "dependencies": {
    "@consilium/codegraph-client": "file:../consilium-codegraph/ts-client"
  }
}
```

### 3. Verify Installation

```bash
node -e "const { getCLIInfo } = require('@consilium/codegraph-client'); console.log(getCLIInfo())"
```

Should show:
```json
{
  "path": "/path/to/node_modules/@consilium/codegraph-client/bin/darwin-arm64/reviewbot"
}
```

## Usage in Codebuff

Your adapter should look like this:

```typescript
// packages/code-map/src/consilium-adapter.ts
import { AgentCodeGraph, scanRepositorySync, isScanned } from "@consilium/codegraph-client";

export async function getFileTokenScoresFromConsilium(projectPath: string) {
  // Step 1: Ensure repository is scanned
  if (!isScanned(projectPath)) {
    console.log("Scanning repository with Consilium...");
    const result = scanRepositorySync(projectPath, {
      semantic: true,
      quiet: false  // Show progress
    });

    if (!result.success) {
      throw new Error(`Scan failed: ${result.error}\n${result.output}`);
    }

    console.log(`✅ Scan completed in ${result.duration}ms`);
  }

  // Step 2: Get token data
  const agent = new AgentCodeGraph(projectPath);
  const data = agent.getFileTokenData();
  agent.close();

  return data;
}
```

Then update your `project-files.ts` to use this adapter:

```typescript
import { getFileTokenScoresFromConsilium } from "@code-map/consilium-adapter";

// In your main function:
const { tokenScores, tokenCallers } = await getFileTokenScoresFromConsilium(projectRoot);
```

## How It Works

### 1. First Run (Scanning)

When you run Codebuff in a new repository:

1. `isScanned()` checks if `.reviewbot/graph.db` exists
2. If not, `scanRepositorySync()` is called
3. This runs the bundled Rust binary: `reviewbot --repo /path/to/project scan --semantic`
4. The binary:
   - Parses all TypeScript/JavaScript/Python/Java files
   - Builds a code graph with symbols and relationships
   - Stores everything in SQLite at `.reviewbot/graph.db`
   - Takes ~30-60 seconds for a 100k LOC project

### 2. Subsequent Runs (Querying)

On subsequent runs:

1. `isScanned()` returns true (database exists)
2. No scanning happens - instant startup!
3. `AgentCodeGraph` opens the SQLite database
4. `getFileTokenData()` queries the graph and returns scores
5. Takes ~20ms total

### 3. Incremental Updates (Optional)

To rescan after code changes:

```typescript
// Force a rescan
scanRepositorySync(projectPath, { semantic: true, force: true });
```

Or just delete the database:

```bash
rm -rf .reviewbot/
```

## Data Format

### Input (from Consilium)

```typescript
{
  tokenScores: {
    "src/auth.ts": {
      "authenticate": 2.386,  // 1.0 + ln(1 + 10 callers)
      "validateToken": 1.609,  // 1.0 + ln(1 + 1 caller)
      "User": 1.000            // 1.0 + ln(1 + 0 callers)
    }
  },
  tokenCallers: {
    "src/auth.ts": {
      "authenticate": ["src/api/routes.ts", "src/middleware.ts", ...],
      "validateToken": ["src/middleware.ts"],
      "User": []
    }
  }
}
```

### Scoring Formula

```
score = 1.0 + ln(1 + numCallers)
```

- Base score: 1.0 (all symbols start here)
- Grows logarithmically with caller count
- Symbol called 10 times: `1.0 + ln(11) = 2.398`
- Symbol called 100 times: `1.0 + ln(101) = 5.617`

### Comparison with Tree-sitter

| Metric | Tree-sitter | Consilium |
|--------|-------------|-----------|
| Data source | Raw AST | Code graph |
| Caller info | ❌ No | ✅ Yes |
| Token savings | 0% baseline | ~80% |
| Cross-file | ❌ No | ✅ Yes |
| Setup | None | One-time scan |
| Query speed | Fast | Faster (cached) |

## Troubleshooting

### "Rust CLI not found"

Binary isn't in the expected location. Check:

```bash
ls -la node_modules/@consilium/codegraph-client/bin/darwin-arm64/reviewbot
```

If missing, rebuild:

```bash
cd /path/to/consilium-codegraph
cargo build --release
cp target/release/reviewbot ts-client/bin/darwin-arm64/
```

### "Scanning repository..." hangs

This is normal for first scan! Large repositories take time:

- 10k LOC: ~10 seconds
- 100k LOC: ~60 seconds
- 1M LOC: ~10 minutes

Check progress by removing `quiet: true` option.

### "Database is locked"

Another process is scanning. Wait or kill it:

```bash
ps aux | grep reviewbot
kill <PID>
```

### Empty results (no tokens)

The scan might have failed silently. Check the database:

```bash
sqlite3 .reviewbot/graph.db "SELECT COUNT(*) FROM symbol"
```

Should return a number > 0. If 0, rescan with error output:

```typescript
const result = scanRepositorySync(projectPath, { quiet: false });
console.log(result.output);
```

## Performance Tips

### 1. Scan Once, Cache Forever

The `.reviewbot/` directory can be committed to git or cached in CI:

```bash
# .gitignore - Don't ignore if you want to cache it!
# .reviewbot/
```

### 2. Incremental Scans

Currently, Consilium always does full scans. Incremental support coming soon.

### 3. Parallel Scanning

If indexing multiple projects, scan them in parallel:

```typescript
await Promise.all([
  scanRepository(project1),
  scanRepository(project2),
  scanRepository(project3),
]);
```

## What's Next?

- ✅ Codebuff integration working
- ✅ Pre-built binaries for GitHub distribution
- ⏳ Incremental scanning
- ⏳ IDE navigation tools
- ⏳ Cross-repository analysis

Enjoy using Consilium! 🎉
