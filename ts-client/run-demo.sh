#!/bin/bash
# Quick demo script for Consilium CodeGraph TypeScript Client

set -e

echo "🚀 Consilium CodeGraph - Quick Demo"
echo "===================================="
echo ""

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
  echo "❌ Error: Please run this script from the ts-client directory"
  echo "   cd ts-client && ./run-demo.sh"
  exit 1
fi

# Step 1: Check if test_repo is indexed
echo "📋 Step 1: Checking if test_repo is indexed..."
if [ ! -f "../test_repo/.reviewbot/graph.db" ]; then
  echo "⚠️  Database not found. Indexing test_repo..."
  cd ..
  cargo run -- --repo ./test_repo scan --semantic
  cd ts-client
  echo "✅ Indexing complete!"
else
  echo "✅ Database found at ../test_repo/.reviewbot/graph.db"
fi
echo ""

# Step 2: Install dependencies if needed
echo "📋 Step 2: Checking dependencies..."
if [ ! -d "node_modules" ]; then
  echo "📦 Installing npm packages..."
  npm install
  echo "✅ Dependencies installed!"
else
  echo "✅ Dependencies already installed"
fi
echo ""

# Step 3: Build if needed
echo "📋 Step 3: Building TypeScript client..."
if [ ! -d "dist" ] || [ "src/index.ts" -nt "dist/index.js" ]; then
  echo "🔨 Building..."
  npm run build
  echo "✅ Build complete!"
else
  echo "✅ Already built (up to date)"
fi
echo ""

# Step 4: Run the demo
echo "📋 Step 4: Running analysis demo..."
echo "===================================="
echo ""
npx tsx analyze-test-repo.ts

echo ""
echo "===================================="
echo "✨ Demo complete!"
echo ""
echo "📚 Next steps:"
echo "  • Check out examples/basic-usage.ts for more examples"
echo "  • Read README.md for API documentation"
echo "  • Run 'npm test' to run the test suite"
echo "  • Create your own analysis scripts!"
echo ""
