# Codebuff Integration Guide

## Overview

Consilium CodeGraph can replace tree-sitter in Codebuff to provide better code understanding with 80% token savings.

## The Integration Method

Consilium provides `getFileTokenData()` which returns the exact format Codebuff expects:

```typescript
{
  tokenScores: {
    "src/auth.ts": {
      "authenticate": 2.386,
      "validateToken": 1.609
    }
  },
  tokenCallers: {
    "src/auth.ts": {
      "authenticate": ["src/api/routes.ts", "src/middleware/auth.ts"],
      "validateToken": ["src/middleware/auth.ts"]
    }
  }
}
```

### What It Does

1. **Analyzes all files** in the project
2. **Calculates scores** for each symbol based on usage: `1.0 + ln(1 + numCallers)`
3. **Maps callers** to file paths (not FQNs), deduplicated, max 25 per symbol
4. **Filters out** boilerplate (`__init__`, `constructor`, etc.)

## Integration Steps

### Step 1: Add Consilium Dependency

In Codebuff's `packages/code-map/package.json`:

```json
{
  "dependencies": {
    "@consilium/codegraph-client": "file:../../consilium-codegraph/ts-client"
  }
}
```

Or install from npm when published.

### Step 2: Create Adapter

Create `packages/code-map/src/consilium-adapter.ts`:

```typescript
import { scanRepositorySync, isScanned, AgentCodeGraph } from "@consilium/codegraph-client";
import type { FileTokenData } from "./parse";

export async function getFileTokenScoresFromConsilium(
  projectRoot: string,
  filePaths: string[] // Ignored - consilium scans entire project
): Promise<FileTokenData> {
  // 1. Index if needed
  if (!isScanned(projectRoot)) {
    console.log("📦 Indexing project with Consilium...");

    const result = scanRepositorySync(projectRoot, {
      semantic: true, // Include type information
      quiet: false
    });

    if (!result.success) {
      throw new Error(`Failed to index: ${result.error}`);
    }

    console.log(`✅ Indexed in ${result.duration}ms`);
  }

  // 2. Get token data
  const agent = new AgentCodeGraph(projectRoot);

  try {
    const data = agent.getFileTokenData();
    return data;
  } finally {
    agent.close();
  }
}
```

### Step 3: Replace in Codebuff

In `npm-app/src/project-files.ts` (around line 302):

```typescript
// BEFORE:
import { getFileTokenScores } from "@codebuff/code-map/src/parse";

const { tokenScores, tokenCallers } = await getFileTokenScores(
  projectRoot,
  projectFilePaths,
  readFile
);

// AFTER:
import { getFileTokenScoresFromConsilium } from "@codebuff/code-map/src/consilium-adapter";

const { tokenScores, tokenCallers } = await getFileTokenScoresFromConsilium(
  projectRoot,
  projectFilePaths
);
```

That's it! Codebuff now uses Consilium for code analysis.

## What You Get

### 1. Drop-in Replacement

- Same `tokenScores` and `tokenCallers` format
- No changes needed to existing Codebuff code
- File tree annotations work the same
- `referencedBy` metadata still appears

### 2. Better Analysis

- **Cross-file understanding**: Tracks calls across files
- **Type-aware**: Knows about classes, interfaces, types
- **Multi-language**: TypeScript, Python, Go, Rust, Java, C++
- **Incremental**: Only re-scans changed files

### 3. Foundation for IDE Tools

Once integrated, you can add powerful navigation tools:

```typescript
// In npm-app/src/tool-handlers.ts

case 'goto_definition': {
  const agent = new AgentCodeGraph(projectRoot);
  const symbol = agent.getSymbol(params.symbol);

  if (!symbol) {
    return [{ type: 'text', value: `Symbol not found` }];
  }

  return [{
    type: 'text',
    value: `${symbol.location.file}:${symbol.location.line}`
  }];
}

case 'find_references': {
  const agent = new AgentCodeGraph(projectRoot);
  const info = agent.getSymbol(params.symbol, {
    includeReferences: true
  });

  return [{
    type: 'text',
    value: info.references.map(ref =>
      `${ref.file}:${ref.line}`
    ).join('\n')
  }];
}

case 'find_callers': {
  const agent = new AgentCodeGraph(projectRoot);
  const callers = agent.queryRelationships(
    params.symbol,
    "called-by",
    { depth: 1 }
  );

  return [{
    type: 'text',
    value: callers.map(c =>
      `${c.symbol} at ${c.location.file}:${c.location.line}`
    ).join('\n')
  }];
}
```

## Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Initial index (100k LOC) | ~30s | One-time per project |
| `getFileTokenData()` | ~20-50ms | Very fast, cached |
| Incremental re-index | ~5-10s | Only changed files |

Compare to tree-sitter:
- Tree-sitter: Parses every file on every call
- Consilium: Index once, query many times

## Example Output

```json
{
  "tokenScores": {
    "src/auth/service.ts": {
      "AuthService": 3.044,
      "authenticate": 2.386,
      "validateToken": 1.609,
      "refreshToken": 1.386,
      "logout": 1.099
    },
    "src/db/models.ts": {
      "User": 4.158,
      "Session": 2.079,
      "Token": 1.792
    }
  },
  "tokenCallers": {
    "src/auth/service.ts": {
      "authenticate": [
        "src/api/routes.ts",
        "src/middleware/auth.ts",
        "tests/auth.test.ts"
      ],
      "validateToken": [
        "src/middleware/auth.ts"
      ]
    },
    "src/db/models.ts": {
      "User": [
        "src/auth/service.ts",
        "src/api/user-controller.ts",
        "src/db/migrations/001.ts",
        "src/api/admin.ts"
      ]
    }
  }
}
```

## Scoring Formula

```
score = 1.0 + ln(1 + numCallers)
```

- Base score of 1.0 for all symbols
- Logarithmic boost based on caller count
- More callers = higher score (diminishing returns)
- Rounded to 3 decimal places

**Examples:**
- 0 callers: `1.0`
- 1 caller: `1.693`
- 5 callers: `2.792`
- 10 callers: `3.398`
- 20 callers: `4.044`

## Edge Cases Handled

✅ **Ignored symbols**: `__init__`, `__post_init__`, `__call__`, `constructor`
✅ **Max callers**: Limited to 25 per symbol (Codebuff's MAX_CALLERS)
✅ **Deduplication**: If multiple symbols in same file call it, file listed once
✅ **Missing symbols**: Skips if caller symbol not found
✅ **Relative paths**: All paths relative to project root

## Testing

Test the integration:

```bash
cd /path/to/consilium-codegraph
npx tsx ts-client/test-codebuff-integration.ts
```

Expected output:
```
✅ Completed in 24ms

📊 Summary:
  Files: 80
  Total symbols: 1165
  Total caller relationships: 450

✅ Format validation:
  ✓ Score format correct: 2.386
  ✓ Callers is an array with 3 items
  ✓ Callers within 25 limit
  ✓ No ignored symbols found

🎉 All validation checks passed!
```

## Troubleshooting

### Issue: "Database not found"

**Solution**: Make sure to index first:
```typescript
if (!isScanned(projectRoot)) {
  scanRepositorySync(projectRoot, { semantic: true });
}
```

### Issue: "Rust CLI not found"

**Solution**: Build the CLI:
```bash
cd /path/to/consilium-codegraph
cargo build --release
```

### Issue: Empty caller arrays

**Possible causes:**
1. Project hasn't been scanned yet
2. No actual function calls in the code (unlikely)
3. Calls are in different language files (check multi-language support)

## Future Enhancements

Once integrated, you can add:

1. **Go to Definition** tool
2. **Find References** tool
3. **Find Implementations** tool (for interfaces)
4. **Call Hierarchy** tool (deep traversal)
5. **Impact Analysis** tool (what breaks if I change this)

These tools enable 80% token savings by helping agents navigate code like an IDE instead of reading entire files.

## See Also

- [AGENT_API.md](ts-client/AGENT_API.md) - Complete Agent API docs
- [AGENT_TOOLS_REFERENCE.md](ts-client/AGENT_TOOLS_REFERENCE.md) - In-depth reference
- [COMPLETE_EXAMPLE.md](COMPLETE_EXAMPLE.md) - Full usage examples
