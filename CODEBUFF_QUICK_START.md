# Codebuff Quick Start

Get Consilium working in Codebuff in 3 steps.

## Step 1: Install in Codebuff

From your Codebuff `npm-app` directory:

```bash
cd /path/to/codebuff/npm-app

# Install Consilium
bun add /Users/pranavsharan/Developer/consilium-codegraph/ts-client
```

## Step 2: Verify Installation

```bash
bun run -e "import { getDatabaseRuntime } from '@consilium/codegraph-client'; console.log(getDatabaseRuntime())"
```

Should output:
```
{ runtime: "bun", version: "1.x.x" }
```

## Step 3: Use in Your Code

Update your Codebuff adapter (e.g., `packages/code-map/src/consilium-adapter.ts`):

```typescript
import { AgentCodeGraph, scanRepositorySync, isScanned } from "@consilium/codegraph-client";

export async function getFileTokenScoresFromConsilium(projectPath: string) {
  // Ensure repository is scanned
  if (!isScanned(projectPath)) {
    console.log("🔍 Scanning repository with Consilium...");
    const result = scanRepositorySync(projectPath, {
      semantic: true,
      quiet: false,
    });

    if (!result.success) {
      throw new Error(`Scan failed: ${result.error}`);
    }

    console.log(`✅ Scan completed in ${result.duration}ms`);
  }

  // Get token data
  const agent = new AgentCodeGraph(projectPath);
  const data = agent.getFileTokenData();
  agent.close();

  return data;
}
```

Then in your `project-files.ts` (or wherever you call tree-sitter):

```typescript
import { getFileTokenScoresFromConsilium } from "@code-map/consilium-adapter";

// Replace tree-sitter call with:
const { tokenScores, tokenCallers } = await getFileTokenScoresFromConsilium(projectRoot);
```

## That's It!

Now when you run Codebuff:

1. **First time**: It will scan the repository (~30-60 seconds)
2. **Subsequent runs**: Instant! Uses cached database

## Expected Output

```
Welcome back Pranav Sharan! What would you like to do?
codebuff/npm-app > 🔍 Scanning repository with Consilium...
[scanning progress...]
✅ Scan completed in 45230ms
[continues with Codebuff...]
```

## Troubleshooting

### "Rust CLI not found"

The binary isn't built. From `consilium-codegraph`:

```bash
cargo build --release
cp target/release/reviewbot ts-client/bin/darwin-arm64/
```

### Scan takes forever

This is normal for first scan! Large repos:
- 10k LOC: ~10 seconds
- 100k LOC: ~60 seconds
- 1M LOC: ~10 minutes

Subsequent runs are instant (uses cached `.reviewbot/graph.db`).

### Still getting ABI version error

Make sure you rebuilt the TypeScript:

```bash
cd /Users/pranavsharan/Developer/consilium-codegraph/ts-client
npm run build
```

Then reinstall in Codebuff:

```bash
cd /path/to/codebuff/npm-app
bun remove @consilium/codegraph-client
bun add /Users/pranavsharan/Developer/consilium-codegraph/ts-client
```

## Verifying Bun Runtime

To confirm it's using Bun's SQLite (not better-sqlite3):

```typescript
import { getDatabaseRuntime } from "@consilium/codegraph-client";

const runtime = getDatabaseRuntime();
console.log(runtime); // Should show: { runtime: "bun", ... }
```

If it shows "node", you're running with Node.js by accident. Make sure to use `bun` command, not `node` or `npm`.

## Next Steps

Once working:
- The database is cached in `.reviewbot/graph.db`
- You can commit this to git for faster CI/CD
- Or add `.reviewbot/` to `.gitignore` and scan fresh each time

Enjoy! 🎉
