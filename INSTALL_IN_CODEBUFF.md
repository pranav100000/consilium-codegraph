# Install Consilium in Codebuff

## Quick Installation

### Step 1: Go to Codebuff

```bash
cd /path/to/your/codebuff/npm-app
```

### Step 2: Install Consilium

```bash
bun add /Users/pranavsharan/Developer/consilium-codegraph/ts-client
```

This will:
- Install the package locally
- Include the pre-built binary at `node_modules/@consilium/codegraph-client/bin/darwin-arm64/reviewbot`
- Install dependencies (better-sqlite3 is optional, Bun won't use it)

### Step 3: Verify Installation

```bash
bun run -e "import { getDatabaseRuntime } from '@consilium/codegraph-client'; console.log(getDatabaseRuntime())"
```

Should output:
```json
{ "runtime": "bun", "version": "1.x.x" }
```

### Step 4: Create Adapter (if you haven't already)

Create `packages/code-map/src/consilium-adapter.ts`:

```typescript
import {
  AgentCodeGraph,
  scanRepositorySync,
  isScanned
} from "@consilium/codegraph-client";

export async function getFileTokenScoresFromConsilium(projectPath: string) {
  // Step 1: Ensure repository is scanned
  if (!isScanned(projectPath)) {
    console.log("🔍 Scanning repository with Consilium...");

    const result = scanRepositorySync(projectPath, {
      semantic: true,
      quiet: false,  // Show progress
    });

    if (!result.success) {
      throw new Error(`Scan failed: ${result.error}\n${result.output}`);
    }

    console.log(`✅ Scan completed in ${result.duration}ms`);
  } else {
    console.log("✅ Using cached code graph");
  }

  // Step 2: Get token data
  const agent = new AgentCodeGraph(projectPath);
  const data = agent.getFileTokenData();
  agent.close();

  return data;
}
```

### Step 5: Update Your Code to Use It

In your `project-files.ts` or wherever you currently call tree-sitter:

```typescript
import { getFileTokenScoresFromConsilium } from "@code-map/consilium-adapter";

// Replace tree-sitter call with:
const { tokenScores, tokenCallers } = await getFileTokenScoresFromConsilium(projectRoot);
```

### Step 6: Test It

```bash
bun run your-codebuff-command
```

You should see:
```
Welcome back Pranav Sharan! What would you like to do?
codebuff/npm-app > 🔍 Scanning repository with Consilium...
✅ Scan completed in 45230ms
[continues with Codebuff...]
```

On subsequent runs:
```
Welcome back Pranav Sharan! What would you like to do?
codebuff/npm-app > ✅ Using cached code graph
[continues with Codebuff...]
```

## Troubleshooting

### "Cannot find module '@consilium/codegraph-client'"

The package wasn't installed. Run:
```bash
bun add /Users/pranavsharan/Developer/consilium-codegraph/ts-client
```

### "Rust CLI not found"

The binary isn't being detected. Check:
```bash
ls node_modules/@consilium/codegraph-client/bin/darwin-arm64/reviewbot
```

If missing, rebuild Consilium:
```bash
cd /Users/pranavsharan/Developer/consilium-codegraph
cargo build --release
cp target/release/reviewbot ts-client/bin/darwin-arm64/
cd ts-client && bun run build
```

Then reinstall in Codebuff:
```bash
cd /path/to/codebuff/npm-app
bun remove @consilium/codegraph-client
bun add /Users/pranavsharan/Developer/consilium-codegraph/ts-client
```

### "better_sqlite3 ABI version error"

This shouldn't happen with Bun! If you see this, you're running with Node.js by accident.

Check:
```bash
which bun
bun --version
```

Make sure you're using `bun` commands, not `npm` or `node`.

### Scan hangs

First scan takes time! This is normal:
- Small project (10k LOC): ~10 seconds
- Medium project (100k LOC): ~60 seconds
- Large project (1M LOC): ~10 minutes

You can monitor progress - the scan outputs to stdout.

If it's truly stuck (no output for 5+ minutes), check:
```bash
ps aux | grep reviewbot
```

If the process is running, let it finish. If not, there might be an error - check the output.

## Expected Behavior

### First Run
1. Detects no `.reviewbot/graph.db`
2. Runs binary to scan project
3. Takes 30-60 seconds for typical project
4. Creates `.reviewbot/graph.db` (SQLite database)
5. Returns token scores to Codebuff

### Subsequent Runs
1. Detects existing `.reviewbot/graph.db`
2. Skips scanning
3. Queries database directly (~20ms)
4. Returns token scores to Codebuff

## What Gets Created

```
your-project/
├── .reviewbot/
│   └── graph.db          # SQLite database (cached code graph)
├── node_modules/
│   └── @consilium/codegraph-client/
│       ├── bin/
│       │   └── darwin-arm64/
│       │       └── reviewbot
│       └── dist/
│           └── index.js
└── your-code.ts
```

## Cleaning Up

To force a fresh scan:
```bash
rm -rf .reviewbot/
```

Next run will scan from scratch.

## Success!

If you see:
```
✅ Scan completed in XXXXXms
```

Or:
```
✅ Using cached code graph
```

Then Consilium is working! 🎉
