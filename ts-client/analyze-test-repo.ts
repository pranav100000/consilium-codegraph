#!/usr/bin/env node
/**
 * Simple script to analyze the test_repo
 * Run with: npx tsx analyze-test-repo.ts
 */

import { CodeGraphAPI, analyzeCodebase } from "./src/index";

console.log("🔍 Consilium CodeGraph - Test Repository Analysis\n");
console.log("=".repeat(60));

try {
  // Quick analysis using convenience function
  console.log("\n📊 Running quick analysis...\n");
  const analysis = analyzeCodebase("../test_repo");

  console.log("Repository Statistics:");
  console.log(`  📁 Total Files: ${analysis.stats.totalFiles}`);
  console.log(`  🔤 Total Symbols: ${analysis.stats.totalSymbols}`);
  console.log(`  🔗 Total Edges: ${analysis.stats.totalEdges}`);
  console.log(`  🔄 Cycles Found: ${analysis.cycles.length}`);
  console.log(`  🚪 Entry Points: ${analysis.entryPoints.length}`);

  // Show symbols by kind
  console.log("\n📦 Symbols by Kind:");
  Object.entries(analysis.stats.symbolsByKind).forEach(([kind, count]) => {
    console.log(`  ${kind.padEnd(15)}: ${count}`);
  });

  // Show edges by type
  console.log("\n🔗 Edges by Type:");
  Object.entries(analysis.stats.edgesByType).forEach(([type, count]) => {
    console.log(`  ${type.padEnd(15)}: ${count}`);
  });

  // Show complex functions
  if (analysis.complexFunctions.length > 0) {
    console.log("\n🔧 Most Complex Functions:");
    analysis.complexFunctions.slice(0, 5).forEach((func, idx) => {
      console.log(`  ${idx + 1}. ${func.function}`);
      console.log(`     └─ Calls ${func.calleesCount} functions`);
    });
  }

  // Show entry points
  if (analysis.entryPoints.length > 0) {
    console.log("\n🚪 Entry Points:");
    analysis.entryPoints.slice(0, 5).forEach(entry => {
      console.log(`  - ${entry}`);
    });
  }

  // Show cycles if any
  if (analysis.cycles.length > 0) {
    console.log("\n⚠️  Circular Dependencies Detected:");
    analysis.cycles.slice(0, 3).forEach((cycle, idx) => {
      console.log(`\n  Cycle ${idx + 1}:`);
      cycle.forEach(sym => console.log(`    → ${sym}`));
    });
  }

  // Detailed analysis with full API
  console.log("\n" + "=".repeat(60));
  console.log("\n🔬 Detailed Analysis with Full API\n");

  const api = new CodeGraphAPI("../test_repo");

  // Find all TypeScript files
  const allSymbols = api.findSymbols("", undefined);
  const tsFiles = new Set(
    allSymbols
      .filter(s => s.location.file.endsWith('.ts') || s.location.file.endsWith('.tsx'))
      .map(s => s.location.file)
  );

  console.log(`📄 TypeScript Files: ${tsFiles.size}`);

  // Find all classes
  const classes = api.findSymbols("", "Class");
  console.log(`🏗️  Classes: ${classes.length}`);
  if (classes.length > 0) {
    console.log("\n   Sample Classes:");
    classes.slice(0, 3).forEach(cls => {
      console.log(`   - ${cls.name} (${cls.location.file}:${cls.location.line})`);
    });
  }

  // Find all functions
  const functions = api.findSymbols("", "Function");
  console.log(`\n⚙️  Functions: ${functions.length}`);
  if (functions.length > 0) {
    console.log("\n   Sample Functions:");
    functions.slice(0, 3).forEach(fn => {
      console.log(`   - ${fn.name} (${fn.location.file}:${fn.location.line})`);
    });
  }

  // Analyze a specific function
  if (functions.length > 0) {
    const sampleFunc = functions[0];
    console.log(`\n🔎 Analyzing Function: ${sampleFunc.name}`);
    console.log(`   Location: ${sampleFunc.location.file}:${sampleFunc.location.line}`);

    const callers = api.getCallers(sampleFunc.fqn);
    const callees = api.getCallees(sampleFunc.fqn);
    const impact = api.getImpactRadius(sampleFunc.fqn, 2);

    console.log(`   Called by: ${callers.length} functions`);
    console.log(`   Calls: ${callees.length} functions`);
    console.log(`   Impact radius: ${impact.size} symbols`);

    if (callees.length > 0 && callees.length <= 5) {
      console.log(`\n   Calls these functions:`);
      callees.forEach(callee => {
        const calleeSym = api.getSymbol(callee);
        if (calleeSym) {
          console.log(`   - ${calleeSym.name}`);
        }
      });
    }
  }

  // Get dependencies for a symbol
  if (allSymbols.length > 0) {
    const sample = allSymbols[0];
    const deps = api.getDependencies(sample.fqn);

    if (deps.imports.length > 0 || deps.calls.length > 0 || deps.uses.length > 0) {
      console.log(`\n📦 Dependencies for ${sample.name}:`);
      if (deps.imports.length > 0) {
        console.log(`   Imports: ${deps.imports.length}`);
      }
      if (deps.calls.length > 0) {
        console.log(`   Calls: ${deps.calls.length}`);
      }
      if (deps.uses.length > 0) {
        console.log(`   Uses: ${deps.uses.length}`);
      }
    }
  }

  api.close();

  console.log("\n" + "=".repeat(60));
  console.log("\n✅ Analysis complete!\n");

} catch (error: any) {
  console.error("\n❌ Error during analysis:");
  console.error(error.message);
  console.log("\n💡 Make sure you've indexed the repository first:");
  console.log("   cd ..");
  console.log("   cargo run -- --repo ./test_repo scan --semantic");
  console.log("");
  process.exit(1);
}
