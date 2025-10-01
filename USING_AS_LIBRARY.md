# Using Consilium CodeGraph as a Library in Your Project

How to import and use the TypeScript client programmatically in your own codebase.

## Option 1: NPM Link (Recommended for Development)

This creates a symlink so your project can import the library locally.

### Setup (One-Time)

```bash
# 1. In the ts-client directory, create an npm link
cd /Users/pranavsharan/Developer/consilium-codegraph/ts-client
npm link

# 2. In YOUR project directory, link to it
cd /path/to/your/project
npm link @consilium/codegraph-client
```

### Use in Your Project

```typescript
// your-project/src/analyzer.ts
import { CodeGraphAPI, analyzeCodebase } from "@consilium/codegraph-client";

export function analyzeMyCode() {
  // Analyze the current project
  const api = new CodeGraphAPI(process.cwd());

  const stats = api.getStats();
  console.log(`Your project has ${stats.totalSymbols} symbols`);

  api.close();
  return stats;
}

// Or use convenience functions
export function quickAnalysis() {
  const analysis = analyzeCodebase(process.cwd());
  return analysis;
}
```

Run it:
```bash
# First, index your project
cd /Users/pranavsharan/Developer/consilium-codegraph
cargo run -- --repo /path/to/your/project scan --semantic

# Then use it in your code
cd /path/to/your/project
npm run build  # or ts-node src/analyzer.ts
```

---

## Option 2: Direct Import (Quick & Easy)

Import directly from the compiled files without installing.

### In Your TypeScript Project

```typescript
// your-project/src/code-analysis.ts
import { CodeGraphAPI } from "../../../consilium-codegraph/ts-client/dist/index.js";

const api = new CodeGraphAPI(process.cwd());
const symbols = api.findSymbols("User");
console.log(`Found ${symbols.length} symbols`);
api.close();
```

### In Your JavaScript Project

```javascript
// your-project/analyze.js
const { CodeGraphAPI } = require('/Users/pranavsharan/Developer/consilium-codegraph/ts-client/dist/index.js');

const api = new CodeGraphAPI(process.cwd());
const stats = api.getStats();
console.log('Stats:', stats);
api.close();
```

---

## Option 3: Copy as Dependency

Copy the compiled library into your project.

```bash
# In your project
mkdir -p lib/codegraph
cp -r /Users/pranavsharan/Developer/consilium-codegraph/ts-client/dist/* lib/codegraph/
cp -r /Users/pranavsharan/Developer/consilium-codegraph/ts-client/node_modules/better-sqlite3 lib/codegraph/
```

Then use:
```typescript
import { CodeGraphAPI } from "./lib/codegraph/index.js";
```

---

## Option 4: Publish to NPM (Production)

For production use, publish the package.

### Publish to NPM

```bash
cd ts-client

# Login to NPM (if not already)
npm login

# Publish
npm publish
```

### Install in Your Project

```bash
cd /path/to/your/project
npm install @consilium/codegraph-client
```

### Use Like Any NPM Package

```typescript
import { CodeGraphAPI } from "@consilium/codegraph-client";

const api = new CodeGraphAPI("/path/to/repo");
// Use the API...
```

---

## Real-World Integration Examples

### Example 1: Express.js API Server

```typescript
// your-project/src/api/code-stats.ts
import express from 'express';
import { CodeGraphAPI } from '@consilium/codegraph-client';

const router = express.Router();

router.get('/stats', (req, res) => {
  try {
    const api = new CodeGraphAPI(process.cwd());
    const stats = api.getStats();
    api.close();

    res.json(stats);
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

router.get('/symbols/:name', (req, res) => {
  const api = new CodeGraphAPI(process.cwd());
  const symbols = api.findSymbols(req.params.name);
  api.close();

  res.json(symbols);
});

export default router;
```

### Example 2: CLI Tool

```typescript
// your-project/src/cli.ts
#!/usr/bin/env node
import { Command } from 'commander';
import { CodeGraphAPI, analyzeCodebase } from '@consilium/codegraph-client';

const program = new Command();

program
  .name('my-analyzer')
  .description('Custom code analyzer using Consilium CodeGraph');

program
  .command('analyze')
  .description('Analyze the current project')
  .action(() => {
    const analysis = analyzeCodebase(process.cwd());
    console.log('Analysis Results:');
    console.log(JSON.stringify(analysis.stats, null, 2));
  });

program
  .command('find <symbol>')
  .description('Find symbols by name')
  .action((symbol) => {
    const api = new CodeGraphAPI(process.cwd());
    const results = api.findSymbols(symbol);
    console.log(`Found ${results.length} matches:`);
    results.forEach(r => console.log(`  - ${r.name} (${r.location.file})`));
    api.close();
  });

program.parse();
```

Usage:
```bash
npm link
my-analyzer analyze
my-analyzer find User
```

### Example 3: VS Code Extension

```typescript
// your-extension/src/extension.ts
import * as vscode from 'vscode';
import { CodeGraphAPI } from '@consilium/codegraph-client';

export function activate(context: vscode.ExtensionContext) {
  let disposable = vscode.commands.registerCommand(
    'extension.showCodeStats',
    () => {
      const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
      if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
      }

      const api = new CodeGraphAPI(workspaceFolder.uri.fsPath);
      const stats = api.getStats();
      api.close();

      vscode.window.showInformationMessage(
        `Files: ${stats.totalFiles}, Symbols: ${stats.totalSymbols}`
      );
    }
  );

  context.subscriptions.push(disposable);
}
```

### Example 4: Build Tool Plugin

```typescript
// your-project/plugins/complexity-checker.ts
import { analyzeCodebase } from '@consilium/codegraph-client';

export default function complexityChecker() {
  return {
    name: 'complexity-checker',

    buildStart() {
      console.log('Checking code complexity...');

      const analysis = analyzeCodebase(process.cwd());

      // Fail build if too complex
      if (analysis.complexFunctions.length > 10) {
        throw new Error(
          `Too many complex functions: ${analysis.complexFunctions.length}`
        );
      }

      // Warn about cycles
      if (analysis.cycles.length > 0) {
        console.warn(`⚠️  Found ${analysis.cycles.length} circular dependencies`);
      }
    }
  };
}
```

### Example 5: Testing Helper

```typescript
// your-project/tests/helpers/code-analysis.ts
import { CodeGraphAPI } from '@consilium/codegraph-client';

export class TestCodeAnalyzer {
  private api: CodeGraphAPI;

  constructor(repoPath: string = process.cwd()) {
    this.api = new CodeGraphAPI(repoPath);
  }

  // Check if a function is tested
  hasTests(functionName: string): boolean {
    const testSymbols = this.api.findSymbols(functionName, 'Function')
      .filter(s => s.location.file.includes('.test.') || s.location.file.includes('.spec.'));

    return testSymbols.length > 0;
  }

  // Find untested functions
  getUntestedFunctions(): string[] {
    const allFunctions = this.api.findSymbols('', 'Function')
      .filter(f => !f.location.file.includes('.test.'));

    return allFunctions
      .filter(f => !this.hasTests(f.name))
      .map(f => f.fqn);
  }

  close() {
    this.api.close();
  }
}

// Usage in tests
describe('Code Coverage', () => {
  it('should have tests for all critical functions', () => {
    const analyzer = new TestCodeAnalyzer();
    const untested = analyzer.getUntestedFunctions();

    expect(untested.length).toBeLessThan(5);
    analyzer.close();
  });
});
```

### Example 6: Documentation Generator

```typescript
// your-project/scripts/generate-docs.ts
import { CodeGraphAPI } from '@consilium/codegraph-client';
import * as fs from 'fs';

class DocumentationGenerator {
  private api: CodeGraphAPI;
  private output: string = '';

  constructor(repoPath: string) {
    this.api = new CodeGraphAPI(repoPath);
  }

  generateAPIDocs(): void {
    this.output = '# API Documentation\n\n';

    // Document all exported classes
    const classes = this.api.findSymbols('', 'Class');

    classes.forEach(cls => {
      this.output += `## ${cls.name}\n\n`;
      this.output += `**Location**: \`${cls.location.file}\`\n\n`;

      // Find methods
      const methods = this.api.getFileSymbols(cls.location.file)
        .filter(s => s.kind === 'Method');

      if (methods.length > 0) {
        this.output += '### Methods\n\n';
        methods.forEach(method => {
          this.output += `- **${method.name}**`;
          if (method.signature) {
            this.output += `: ${method.signature}`;
          }
          this.output += '\n';
        });
      }

      this.output += '\n';
    });
  }

  save(filename: string): void {
    fs.writeFileSync(filename, this.output);
    this.api.close();
  }
}

// Usage
const generator = new DocumentationGenerator(process.cwd());
generator.generateAPIDocs();
generator.save('API.md');
console.log('Documentation generated!');
```

---

## Package.json Setup

Add a script to your `package.json`:

```json
{
  "name": "your-project",
  "scripts": {
    "analyze": "tsx src/analyze.ts",
    "analyze:stats": "tsx -e \"const a=require('@consilium/codegraph-client').analyzeCodebase(process.cwd());console.log(a.stats)\"",
    "check:complexity": "tsx scripts/check-complexity.ts",
    "docs:generate": "tsx scripts/generate-docs.ts"
  },
  "dependencies": {
    "@consilium/codegraph-client": "file:../consilium-codegraph/ts-client"
  }
}
```

Or if using npm link:
```json
{
  "dependencies": {
    "@consilium/codegraph-client": "*"
  }
}
```

---

## TypeScript Configuration

Make sure your `tsconfig.json` allows the import:

```json
{
  "compilerOptions": {
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "moduleResolution": "node",
    "resolveJsonModule": true
  }
}
```

---

## Complete Working Example

### Setup

```bash
# 1. Link the library
cd /Users/pranavsharan/Developer/consilium-codegraph/ts-client
npm link

# 2. Create your project
mkdir my-code-analyzer
cd my-code-analyzer
npm init -y
npm install typescript tsx @types/node
npm link @consilium/codegraph-client

# 3. Initialize TypeScript
npx tsc --init
```

### Create Your Tool

```typescript
// src/index.ts
import { CodeGraphAPI, analyzeCodebase, findRelatedCode } from '@consilium/codegraph-client';

export class MyCodeAnalyzer {
  private repoPath: string;

  constructor(repoPath: string = process.cwd()) {
    this.repoPath = repoPath;
  }

  // Get overview
  getOverview() {
    const analysis = analyzeCodebase(this.repoPath);
    return {
      files: analysis.stats.totalFiles,
      symbols: analysis.stats.totalSymbols,
      complexity: analysis.complexFunctions.length,
      cycles: analysis.cycles.length
    };
  }

  // Find specific symbols
  findSymbol(name: string, kind?: string) {
    const api = new CodeGraphAPI(this.repoPath);
    const results = api.findSymbols(name, kind);
    api.close();
    return results;
  }

  // Check impact of changing code
  checkImpact(symbolName: string) {
    const related = findRelatedCode(this.repoPath, symbolName);
    return {
      directCallers: related.callers.length,
      totalImpact: related.impact.length,
      callees: related.callees.length
    };
  }

  // Get detailed stats
  getDetailedStats() {
    const api = new CodeGraphAPI(this.repoPath);
    const stats = api.getStats();
    api.close();
    return stats;
  }
}

// Export for use in other projects
export * from '@consilium/codegraph-client';
```

### Use It

```typescript
// src/main.ts
import { MyCodeAnalyzer } from './index';

const analyzer = new MyCodeAnalyzer('/path/to/target/repo');

// Get overview
const overview = analyzer.getOverview();
console.log('Overview:', overview);

// Find specific code
const userClasses = analyzer.findSymbol('User', 'Class');
console.log(`Found ${userClasses.length} User classes`);

// Check impact
const impact = analyzer.checkImpact('processPayment');
console.log('Impact:', impact);
```

### Build and Run

```bash
# Build
npx tsc

# Run
node dist/main.js

# Or with tsx
npx tsx src/main.ts
```

---

## Pre-requisite: Indexing

Remember, before using the API, you must index the target repository:

```bash
cd /Users/pranavsharan/Developer/consilium-codegraph
cargo run -- --repo /path/to/target/repo scan --semantic
```

You can also automate this in your code:

```typescript
import { execSync } from 'child_process';

function ensureIndexed(repoPath: string) {
  const dbPath = `${repoPath}/.reviewbot/graph.db`;

  if (!fs.existsSync(dbPath)) {
    console.log('Indexing repository...');
    execSync(
      `cargo run -- --repo ${repoPath} scan --semantic`,
      { cwd: '/Users/pranavsharan/Developer/consilium-codegraph' }
    );
  }
}
```

---

## Best Practices

1. **Always close connections**: Call `api.close()` when done
2. **Handle errors**: Wrap in try-catch blocks
3. **Cache results**: Store frequently accessed data
4. **Validate paths**: Check repository exists and is indexed
5. **Use absolute paths**: Avoid relative path issues

```typescript
import { CodeGraphAPI } from '@consilium/codegraph-client';
import * as path from 'path';
import * as fs from 'fs';

function safeAnalyze(repoPath: string) {
  // Resolve to absolute path
  const absPath = path.resolve(repoPath);

  // Check path exists
  if (!fs.existsSync(absPath)) {
    throw new Error(`Repository not found: ${absPath}`);
  }

  // Check database exists
  const dbPath = path.join(absPath, '.reviewbot', 'graph.db');
  if (!fs.existsSync(dbPath)) {
    throw new Error('Repository not indexed. Run: cargo run -- --repo ' + absPath + ' scan --semantic');
  }

  // Now safe to use
  let api: CodeGraphAPI | null = null;
  try {
    api = new CodeGraphAPI(absPath);
    const stats = api.getStats();
    return stats;
  } catch (error) {
    console.error('Analysis failed:', error);
    throw error;
  } finally {
    if (api) api.close();
  }
}
```

---

## Summary

**To use as a library in your project:**

1. **Link it**: `npm link @consilium/codegraph-client`
2. **Import it**: `import { CodeGraphAPI } from '@consilium/codegraph-client'`
3. **Use it**: Create API instance, query, close

**That's it! You can now use Consilium CodeGraph programmatically in any Node.js/TypeScript project.**

The library is fully typed, documented, and ready to integrate into your tools, APIs, extensions, or applications.
