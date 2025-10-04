#!/usr/bin/env node
/**
 * Quick test of the AgentCodeGraph API
 */

import { AgentCodeGraph } from "./src/agent-api";

const repoPath = "/Users/pranavsharan/Developer/consilium-codegraph";

console.log("🧪 Testing AgentCodeGraph API\n");

const agent = new AgentCodeGraph(repoPath);

// Test 1: findSymbols
console.log("Test 1: findSymbols()");
const symbols = agent.findSymbols("test", { kind: ["Function"], limit: 5 });
console.log(`  ✓ Found ${symbols.length} functions with 'test' in name`);
if (symbols.length > 0) {
  console.log(`    Example: ${symbols[0].name} at ${symbols[0].location.file}:${symbols[0].location.line}`);
}

// Test 2: getSymbol (basic)
console.log("\nTest 2: getSymbol() - basic");
if (symbols.length > 0) {
  const symbolInfo = agent.getSymbol(symbols[0].fqn);
  if (symbolInfo) {
    console.log(`  ✓ Got symbol: ${symbolInfo.name} (${symbolInfo.kind})`);
    console.log(`    Location: ${symbolInfo.location.file}:${symbolInfo.location.line}`);
  } else {
    console.log("  ✗ Failed to get symbol");
  }
}

// Test 3: getSymbol (with relationships)
console.log("\nTest 3: getSymbol() - with relationships");
if (symbols.length > 0) {
  const symbolInfo = agent.getSymbol(symbols[0].fqn, {
    includeCallers: true,
    includeCallees: true,
  });
  if (symbolInfo) {
    console.log(`  ✓ Got symbol with relationships`);
    console.log(`    Callers: ${symbolInfo.callers?.length || 0}`);
    console.log(`    Callees: ${symbolInfo.callees?.length || 0}`);
  }
}

// Test 4: queryRelationships
console.log("\nTest 4: queryRelationships()");
const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 10 });
const funcWithCalls = allFunctions.find(fn => {
  const info = agent.getSymbol(fn.fqn, { includeCallees: true });
  return info && info.callees && info.callees.length > 0;
});

if (funcWithCalls) {
  const calls = agent.queryRelationships(funcWithCalls.fqn, "calls", { depth: 1 });
  console.log(`  ✓ Found ${calls.length} things called by ${funcWithCalls.name}`);
  if (calls.length > 0) {
    console.log(`    Example: ${calls[0].symbol} at depth ${calls[0].depth}`);
  }
}

// Test 5: queryRelationships - deep traversal
console.log("\nTest 5: queryRelationships() - deep traversal");
if (funcWithCalls) {
  const deepCalls = agent.queryRelationships(funcWithCalls.fqn, "calls", {
    depth: 2,
    limit: 20
  });
  console.log(`  ✓ Found ${deepCalls.length} transitive dependencies (depth=2)`);

  const byDepth = deepCalls.reduce((acc, node) => {
    acc[node.depth] = (acc[node.depth] || 0) + 1;
    return acc;
  }, {} as Record<number, number>);

  Object.entries(byDepth).forEach(([depth, count]) => {
    console.log(`    Depth ${depth}: ${count} symbols`);
  });
}

// Test 6: Find entry points
console.log("\nTest 6: Finding entry points");
const entryPoints = allFunctions.filter(fn => {
  const info = agent.getSymbol(fn.fqn, { includeCallers: true });
  return info && (!info.callers || info.callers.length === 0);
});
console.log(`  ✓ Found ${entryPoints.length} entry points (functions with no callers)`);
if (entryPoints.length > 0) {
  console.log(`    Examples: ${entryPoints.slice(0, 3).map(ep => ep.name).join(", ")}`);
}

// Test 7: Search by kind
console.log("\nTest 7: Search by kind");
const classes = agent.findSymbols("", { kind: ["Class"], limit: 5 });
console.log(`  ✓ Found ${classes.length} classes`);
if (classes.length > 0) {
  classes.slice(0, 3).forEach(cls => {
    console.log(`    - ${cls.name} in ${cls.location.file}`);
  });
}

// Test 8: Get container relationships
console.log("\nTest 8: Container relationships");
if (classes.length > 0) {
  const classInfo = agent.getSymbol(classes[0].fqn, { includeContains: true });
  if (classInfo && classInfo.contains) {
    console.log(`  ✓ Class "${classInfo.name}" contains ${classInfo.contains.length} members`);
    if (classInfo.contains.length > 0) {
      classInfo.contains.slice(0, 3).forEach(member => {
        console.log(`    - ${member}`);
      });
    }
  }
}

agent.close();

console.log("\n✅ All tests completed!\n");
