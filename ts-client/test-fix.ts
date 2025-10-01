#!/usr/bin/env node
/**
 * Quick test to verify the quoted kind fix
 */

import { CodeGraphAPI } from "./dist/code-graph-api.js";

const repoPath = "/Users/pranavsharan/Developer/consilium-codegraph";

console.log("🔍 Testing kind filtering fix...\n");

const api = new CodeGraphAPI(repoPath);

// Test stats
console.log("📊 Overall Stats:");
const stats = api.getStats();
console.log("  Total Symbols:", stats.totalSymbols);
console.log("  Total Edges:", stats.totalEdges);
console.log("\n  Symbols by Kind:");
for (const [kind, count] of Object.entries(stats.symbolsByKind)) {
  console.log(`    ${kind}: ${count}`);
}
console.log("\n  Edges by Type:");
for (const [type, count] of Object.entries(stats.edgesByType)) {
  console.log(`    ${type}: ${count}`);
}

// Test finding functions
console.log("\n🔎 Finding Functions:");
const functions = api.findSymbols("", "Function");
console.log(`  Found ${functions.length} functions`);

if (functions.length > 0) {
  console.log("  Sample functions:");
  functions.slice(0, 5).forEach((fn) => {
    console.log(`    - ${fn.name} (${fn.kind}) in ${fn.location.file}:${fn.location.line}`);
  });
}

// Test finding classes
console.log("\n🔎 Finding Classes:");
const classes = api.findSymbols("", "Class");
console.log(`  Found ${classes.length} classes`);

if (classes.length > 0) {
  console.log("  Sample classes:");
  classes.slice(0, 5).forEach((cls) => {
    console.log(`    - ${cls.name} (${cls.kind}) in ${cls.location.file}:${cls.location.line}`);
  });
}

// Verify call edges
console.log("\n📞 Testing Call Edges:");
const allEdges = api.getEdges();
console.log(`  Total edges in DB: ${allEdges.length}`);

const callEdges = allEdges.filter((e) => e.edgeType.toLowerCase().includes("call"));
console.log(`  Call-related edges: ${callEdges.length}`);

api.close();

console.log("\n✅ Test complete!");
