#!/bin/bash

# Test incremental scanning

echo "=== Setting up test repository ==="
rm -rf test_incremental
mkdir test_incremental
cd test_incremental
git init

# Create initial files
echo "export function main() { helper(); }" > main.ts
echo "export function helper() { util(); }" > helper.ts  
echo "export function util() { return 42; }" > util.ts

git add .
git commit -m "initial"

echo "=== Running initial scan ==="
cargo run -p reviewbot -- --repo . scan

echo "=== Making a change to util.ts ==="
echo "export function util() { return 100; } // changed" > util.ts
git add .
git commit -m "changed util"

echo "=== Running incremental scan (should only process util.ts and its dependents) ==="
cargo run -p reviewbot -- --repo . scan

echo "=== Checking if scan detects no changes on re-run ==="
cargo run -p reviewbot -- --repo . scan

cd ..