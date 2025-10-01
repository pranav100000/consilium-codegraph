# Example: Using Consilium CodeGraph as a Library

This example shows how to use the Consilium CodeGraph TypeScript client as a library in your own project.

## Setup

```bash
# 1. Link the codegraph library
cd /Users/pranavsharan/Developer/consilium-codegraph/ts-client
npm link

# 2. Install dependencies in this example
cd ../example-usage
npm install
npm link @consilium/codegraph-client
```

## Before Running

**Important**: You must index the repository you want to analyze first!

```bash
# Index a repository (from consilium-codegraph root)
cd /Users/pranavsharan/Developer/consilium-codegraph
cargo run -- --repo /path/to/target/repo scan --semantic

# Example: Index the test_repo
cargo run -- --repo ./test_repo scan --semantic
```

## Usage

### Option 1: Run the Analysis Script

```bash
# Analyze current directory
npm run analyze

# Analyze specific repository
REPO=/path/to/repo npm run analyze

# Example with test_repo
REPO=../test_repo npm run analyze
```

### Option 2: Use Programmatically

```typescript
import { ProjectAnalyzer, getQuickOverview } from "./src/index";

// Quick overview
const overview = getQuickOverview("/path/to/repo");
console.log(overview);

// Detailed analysis
const analyzer = new ProjectAnalyzer("/path/to/repo");
const classes = analyzer.getAllClasses();
const functions = analyzer.getAllFunctions();
const cycles = analyzer.findCycles();
```

### Option 3: Import Individual Functions

```typescript
import {
  getRepositoryStats,
  findComponents,
  analyzeImpact
} from "./src/index";

// Get stats
const stats = getRepositoryStats("/path/to/repo");

// Find components
const components = findComponents("/path/to/repo", "User");

// Analyze impact
const impact = analyzeImpact("/path/to/repo", "processPayment");
```

## Available Functions

### Convenience Functions

- `getRepositoryStats(repoPath)` - Get overall statistics
- `findComponents(repoPath, name)` - Find symbols by name
- `analyzeImpact(repoPath, symbol)` - Impact analysis
- `getQuickOverview(repoPath)` - Quick analysis overview

### ProjectAnalyzer Class

```typescript
const analyzer = new ProjectAnalyzer("/path/to/repo");

analyzer.getAllClasses()           // Get all classes
analyzer.getAllFunctions()         // Get all functions
analyzer.findCycles()              // Find circular dependencies
analyzer.getFileSymbols(file)      // Get symbols in a file
analyzer.getCallers(symbol)        // Who calls this?
analyzer.getCallees(symbol)        // What does this call?
analyzer.getCompleteAnalysis()     // Full analysis
```

## Example Output

```json
{
  "overview": {
    "stats": {
      "files": 74,
      "symbols": 579,
      "edges": 277
    },
    "quality": {
      "cycles": 0,
      "complexFunctions": 3,
      "entryPoints": 12
    },
    "topComplexFunctions": [
      {
        "name": "processData",
        "complexity": 15
      }
    ]
  }
}
```

## Integration Examples

### In Your Own Project

1. **Link the library**:
   ```bash
   npm link @consilium/codegraph-client
   ```

2. **Import and use**:
   ```typescript
   import { CodeGraphAPI } from "@consilium/codegraph-client";

   const api = new CodeGraphAPI(process.cwd());
   const stats = api.getStats();
   api.close();
   ```

3. **Use in your code**:
   - Express API endpoints
   - CLI tools
   - Build scripts
   - Testing helpers
   - Documentation generators

## Tips

- Always index the repository before analyzing
- Use absolute paths for repositories
- Remember to call `api.close()` when done
- Wrap in try-finally for safe cleanup
- Re-index after major code changes

## More Examples

See [../USING_AS_LIBRARY.md](../USING_AS_LIBRARY.md) for more integration patterns:
- Express.js API
- CLI tools
- VS Code extensions
- Build plugins
- Testing helpers
- Documentation generators
