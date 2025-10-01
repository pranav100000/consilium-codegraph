/**
 * Example analysis script
 * Run with: npm run analyze
 */

import { ProjectAnalyzer } from "./index.js";

// Change this to your repository path
const REPO_PATH = process.env.REPO || process.cwd();

console.log(`\n🔍 Analyzing Repository: ${REPO_PATH}\n`);
console.log("=".repeat(70));

const analyzer = new ProjectAnalyzer(REPO_PATH);

// Get complete analysis
const analysis = analyzer.getCompleteAnalysis();

console.log("\n📊 Repository Statistics:");
console.log(`  Files: ${analysis.stats.files}`);
console.log(`  Symbols: ${analysis.stats.symbols}`);
console.log(`  Relationships: ${analysis.stats.edges}`);

console.log("\n📦 Symbol Breakdown:");
Object.entries(analysis.stats.symbolBreakdown).forEach(([kind, count]) => {
  console.log(`  ${kind.padEnd(15)}: ${count}`);
});

console.log("\n🔗 Relationship Breakdown:");
Object.entries(analysis.stats.edgeBreakdown).forEach(([type, count]) => {
  console.log(`  ${type.padEnd(15)}: ${count}`);
});

console.log("\n🎯 Quality Metrics:");
console.log(`  Circular Dependencies: ${analysis.overview.quality.cycles}`);
console.log(`  Complex Functions: ${analysis.overview.quality.complexFunctions}`);
console.log(`  Entry Points: ${analysis.overview.quality.entryPoints}`);

if (analysis.overview.topComplexFunctions.length > 0) {
  console.log("\n🔧 Most Complex Functions:");
  analysis.overview.topComplexFunctions.forEach((fn, idx) => {
    console.log(`  ${idx + 1}. ${fn.name} (calls ${fn.complexity} functions)`);
  });
}

if (analysis.cycles.length > 0) {
  console.log(`\n⚠️  Circular Dependencies Found: ${analysis.cycles.length}`);
  analysis.cycles.slice(0, 3).forEach((cycle, idx) => {
    console.log(`\n  Cycle ${idx + 1}:`);
    cycle.forEach(sym => console.log(`    → ${sym}`));
  });
}

console.log("\n" + "=".repeat(70));
console.log("\n✅ Analysis Complete!\n");
