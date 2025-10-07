# Installing from GitHub

This guide explains how to use Consilium CodeGraph directly from GitHub in your projects (like Codebuff).

## Quick Start

### 1. Install from GitHub

```bash
npm install github:yourusername/consilium-codegraph#main --install-links
# or
yarn add github:yourusername/consilium-codegraph#main
```

The package includes pre-built binaries for:
- macOS ARM64 (Apple Silicon)
- macOS x64 (Intel)
- Linux x64
- Windows x64

**No Rust toolchain required!** The binary for your platform is bundled.

### 2. Use in Your Code

```typescript
import { AgentCodeGraph, scanRepositorySync, isScanned } from "@consilium/codegraph-client";

// Scan repository (one-time setup)
if (!isScanned("./my-project")) {
  scanRepositorySync("./my-project", { semantic: true });
}

// Query the code graph
const agent = new AgentCodeGraph("./my-project");
const data = agent.getFileTokenData();
console.log(data.tokenScores);
```

## For Codebuff Integration

When using in Codebuff's `npm-app`:

### 1. Add to package.json

```json
{
  "dependencies": {
    "@consilium/codegraph-client": "github:yourusername/consilium-codegraph#main"
  }
}
```

### 2. Update your adapter

```typescript
import { AgentCodeGraph, scanRepositorySync, isScanned } from "@consilium/codegraph-client";

export async function getFileTokenScoresFromConsilium(projectPath: string) {
  // Ensure repository is scanned
  if (!isScanned(projectPath)) {
    console.log("Scanning repository with Consilium...");
    const result = scanRepositorySync(projectPath, { semantic: true });
    if (!result.success) {
      throw new Error(`Scan failed: ${result.error}`);
    }
  }

  // Get token data
  const agent = new AgentCodeGraph(projectPath);
  const data = agent.getFileTokenData();
  agent.close();

  return data;
}
```

## Platform Support

The package automatically detects your platform and uses the appropriate binary:

| Platform | Architecture | Binary Location |
|----------|--------------|----------------|
| macOS | ARM64 (M1/M2/M3) | `bin/darwin-arm64/reviewbot` |
| macOS | x64 (Intel) | `bin/darwin-x64/reviewbot` |
| Linux | x64 | `bin/linux-x64/reviewbot` |
| Windows | x64 | `bin/win32-x64/reviewbot.exe` |

## Building Binaries for Other Platforms

If you need to build for a platform not included:

```bash
# Clone the repo
git clone https://github.com/yourusername/consilium-codegraph
cd consilium-codegraph

# Build for your platform
cargo build --release

# Copy to ts-client bin directory
mkdir -p ts-client/bin/YOUR-PLATFORM-ARCH
cp target/release/reviewbot ts-client/bin/YOUR-PLATFORM-ARCH/

# Rebuild TypeScript
cd ts-client
npm run build
```

## Troubleshooting

### Binary not found

If you get "Rust CLI not found" error:

1. Check that the binary exists:
   ```bash
   ls node_modules/@consilium/codegraph-client/bin/
   ```

2. Verify it's executable:
   ```bash
   chmod +x node_modules/@consilium/codegraph-client/bin/darwin-arm64/reviewbot
   ```

3. As fallback, build manually:
   ```bash
   cd node_modules/@consilium/codegraph-client/../../
   cargo build --release
   ```

### Scan gets stuck

This usually means:
- The binary isn't built yet (run `cargo build --release`)
- The binary isn't executable (run `chmod +x` on it)
- The repository is very large (scanning can take 1-2 minutes)

Check the scan output for errors:
```typescript
const result = scanRepositorySync(projectPath);
if (!result.success) {
  console.error("Scan failed:", result.error);
  console.error("Output:", result.output);
}
```

## Development Mode

When developing Consilium itself:

```bash
# Build the Rust binary
cargo build --release

# Copy to bin directory for testing
cp target/release/reviewbot ts-client/bin/darwin-arm64/

# Build TypeScript
cd ts-client
npm run build

# Test it works
npx ts-node test-codebuff-integration.ts
```

## CI/CD Setup

To automatically build binaries for all platforms, add to your GitHub Actions:

```yaml
name: Build Binaries

on: [push, pull_request]

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            platform: darwin-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            platform: darwin-x64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            platform: linux-x64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            platform: win32-x64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Copy binary
        run: |
          mkdir -p ts-client/bin/${{ matrix.platform }}
          cp target/${{ matrix.target }}/release/reviewbot* ts-client/bin/${{ matrix.platform }}/

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: binaries
          path: ts-client/bin/
```

Then download and commit the binaries after each release.
