#!/usr/bin/env node
/**
 * Analyze any repository - just change the REPO_PATH
 *
 * Usage:
 *   1. Edit REPO_PATH below to point to your repository
 *   2. Make sure it's indexed: cargo run -- --repo /path/to/repo scan --semantic
 *   3. Run: npx tsx analyze-any-repo.ts
 *
 * Or use environment variable:
 *   REPO=/path/to/repo npx tsx analyze-any-repo.ts
 */

import { CodeGraphAPI, analyzeCodebase } from "./src/index";
import { join } from "path";
import { homedir } from "os";

// ====== CONFIGURE THIS ======
// Change this to your repository path
const REPO_PATH = process.env.REPO || join(homedir(), "Projects", "my-app");
// ============================

console.log(`\n🔍 Analyzing: ${REPO_PATH}\n`);
console.log("=".repeat(70));

try {
  // Quick overview
  console.log("\n📊 QUICK OVERVIEW\n");
  const analysis = analyzeCodebase(REPO_PATH);

  console.log("Repository Statistics:");
  console.log(`  📁 Files: ${analysis.stats.totalFiles}`);
  console.log(`  🔤 Symbols: ${analysis.stats.totalSymbols}`);
  console.log(`  🔗 Relationships: ${analysis.stats.totalEdges}`);
  console.log(`  🔄 Circular Dependencies: ${analysis.cycles.length}`);

  // Show symbol breakdown
  console.log("\n📦 Symbols by Type:");
  Object.entries(analysis.stats.symbolsByKind)
    .sort((a, b) => b[1] - a[1])
    .forEach(([kind, count]) => {
      const kindName = kind.replace(/"/g, "");
      console.log(`  ${kindName.padEnd(15)}: ${count}`);
    });

  // Show relationship breakdown
  console.log("\n🔗 Relationships by Type:");
  Object.entries(analysis.stats.edgesByType)
    .sort((a, b) => b[1] - a[1])
    .forEach(([type, count]) => {
      const typeName = type.replace(/"/g, "");
      console.log(`  ${typeName.padEnd(15)}: ${count}`);
    });

  // Complex functions
  if (analysis.complexFunctions.length > 0) {
    console.log("\n🔧 Most Complex Functions:");
    analysis.complexFunctions.slice(0, 10).forEach((func, idx) => {
      console.log(`  ${idx + 1}. ${func.function}`);
      console.log(`     Calls: ${func.calleesCount} functions`);
    });
  }

  // Entry points
  if (analysis.entryPoints.length > 0) {
    console.log("\n🚪 Entry Points (functions with few callers):");
    analysis.entryPoints.slice(0, 10).forEach(entry => {
      console.log(`  - ${entry}`);
    });
  }

  // Cycles (if any)
  if (analysis.cycles.length > 0) {
    console.log(`\n⚠️  WARNING: ${analysis.cycles.length} Circular Dependencies Found!`);
    analysis.cycles.slice(0, 3).forEach((cycle, idx) => {
      console.log(`\n  Cycle ${idx + 1}:`);
      cycle.forEach(sym => console.log(`    → ${sym}`));
    });
    if (analysis.cycles.length > 3) {
      console.log(`\n  ... and ${analysis.cycles.length - 3} more cycles`);
    }
  }

  // Detailed analysis
  console.log("\n" + "=".repeat(70));
  console.log("\n🔬 DETAILED ANALYSIS\n");

  const api = new CodeGraphAPI(REPO_PATH);

  // Find key components
  const classes = api.findSymbols("", "Class");
  const functions = api.findSymbols("", "Function");
  const interfaces = api.findSymbols("", "Interface");

  console.log(`Classes: ${classes.length}`);
  console.log(`Functions: ${functions.length}`);
  console.log(`Interfaces: ${interfaces.length}`);

  // Show some examples
  if (classes.length > 0) {
    console.log("\n📋 Sample Classes:");
    classes.slice(0, 5).forEach(cls => {
      console.log(`  - ${cls.name}`);
      console.log(`    Location: ${cls.location.file}:${cls.location.line}`);
    });
  }

  if (functions.length > 0) {
    console.log("\n⚙️  Sample Functions:");
    functions.slice(0, 5).forEach(fn => {
      console.log(`  - ${fn.name}`);
      console.log(`    Location: ${fn.location.file}:${fn.location.line}`);

      // Show relationships
      const callers = api.getCallers(fn.fqn);
      const callees = api.getCallees(fn.fqn);

      if (callers.length > 0 || callees.length > 0) {
        console.log(`    Called by: ${callers.length}, Calls: ${callees.length}`);
      }
    });
  }

  // Search for common patterns
  console.log("\n🔍 Common Patterns:");

  const handlers = api.findSymbols("handler");
  if (handlers.length > 0) {
    console.log(`  Handlers: ${handlers.length} found`);
  }

  const controllers = api.findSymbols("controller");
  if (controllers.length > 0) {
    console.log(`  Controllers: ${controllers.length} found`);
  }

  const services = api.findSymbols("service");
  if (services.length > 0) {
    console.log(`  Services: ${services.length} found`);
  }

  const utils = api.findSymbols("util");
  if (utils.length > 0) {
    console.log(`  Utilities: ${utils.length} found`);
  }

  api.close();

  console.log("\n" + "=".repeat(70));
  console.log("\n✅ Analysis complete!\n");

  // Suggestions
  console.log("💡 Next Steps:");
  console.log("  • Edit this script to customize the analysis");
  console.log("  • Try: api.findSymbols('YourClassName')");
  console.log("  • Try: api.getCallers('YourFunction')");
  console.log("  • Try: api.getImpactRadius('YourSymbol', 3)");
  console.log("  • See ts-client/README.md for full API docs\n");

} catch (error: any) {
  console.error("\n❌ Error during analysis:");
  console.error(error.message);

  if (error.message.includes("Database not found")) {
    console.log("\n💡 To fix this:");
    console.log(`  1. Make sure the path is correct: ${REPO_PATH}`);
    console.log(`  2. Index the repository first:`);
    console.log(`     cargo run -- --repo ${REPO_PATH} scan --semantic`);
    console.log(`  3. Run this script again\n`);
  } else {
    console.log("\n💡 Troubleshooting:");
    console.log("  • Check that the path exists and is correct");
    console.log("  • Make sure the repository has been indexed");
    console.log("  • Try re-indexing with: cargo run -- --repo /path scan --semantic\n");
  }

  process.exit(1);
}
