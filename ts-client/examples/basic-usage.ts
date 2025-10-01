/**
 * Basic usage examples for Consilium CodeGraph TypeScript Client
 */

import { CodeGraphAPI, CodeGraph, analyzeCodebase, findRelatedCode } from "../src";

// Example 1: Simple API usage
function simpleAPIExample() {
  console.log("\n=== Simple API Example ===\n");

  const api = new CodeGraphAPI("./test_repo");

  // Search for symbols
  const symbols = api.findSymbols("User");
  console.log(`Found ${symbols.length} symbols matching "User"`);

  symbols.slice(0, 3).forEach(symbol => {
    console.log(`  - ${symbol.name} (${symbol.kind}) at ${symbol.location.file}:${symbol.location.line}`);
  });

  // Get statistics
  const stats = api.getStats();
  console.log(`\nRepository Statistics:`);
  console.log(`  Files: ${stats.totalFiles}`);
  console.log(`  Symbols: ${stats.totalSymbols}`);
  console.log(`  Edges: ${stats.totalEdges}`);

  // Cleanup
  api.close();
}

// Example 2: Call graph analysis
function callGraphExample() {
  console.log("\n=== Call Graph Analysis ===\n");

  const api = new CodeGraphAPI("./test_repo");

  // Find a function
  const functions = api.findSymbols("process", "function");
  if (functions.length > 0) {
    const func = functions[0];
    console.log(`Analyzing: ${func.fqn}\n`);

    // Get callers and callees
    const callers = api.getCallers(func.fqn);
    const callees = api.getCallees(func.fqn);

    console.log(`Called by (${callers.length} callers):`);
    callers.slice(0, 5).forEach(caller => {
      console.log(`  - ${caller}`);
    });

    console.log(`\nCalls (${callees.length} callees):`);
    callees.slice(0, 5).forEach(callee => {
      console.log(`  - ${callee}`);
    });

    // Impact analysis
    const impact = api.getImpactRadius(func.fqn, 2);
    console.log(`\nImpact radius: ${impact.size} symbols would be affected`);
  }

  api.close();
}

// Example 3: Full-featured API with call paths
function advancedExample() {
  console.log("\n=== Advanced API Example ===\n");

  const graph = new CodeGraph("./test_repo");

  // Get file symbols
  const fileSymbols = graph.getFileSymbols("src/services/user.ts");
  console.log(`Symbols in user.ts: ${fileSymbols.length}`);

  // Get call paths with depth
  if (fileSymbols.length > 0) {
    const symbol = fileSymbols[0];
    const callers = graph.getCallers(symbol.fqn, 2); // 2 levels deep

    console.log(`\nCall paths to ${symbol.name}:`);
    callers.slice(0, 3).forEach((callPath, idx) => {
      console.log(`\nPath ${idx + 1} (depth: ${callPath.depth})${callPath.isRecursive ? " [RECURSIVE]" : ""}:`);
      callPath.path.forEach(sym => {
        console.log(`  -> ${sym.fqn} (${sym.location.file}:${sym.location.line})`);
      });
    });
  }

  // Get dependency graph
  if (fileSymbols.length > 0) {
    const deps = graph.getDependencies(fileSymbols[0].fqn);
    console.log(`\nDependency Analysis for ${deps.root.name}:`);
    console.log(`  Dependencies: ${deps.dependencies[deps.root.fqn]?.length || 0}`);
    console.log(`  Dependents: ${deps.dependents[deps.root.fqn]?.length || 0}`);

    if (deps.cycles.length > 0) {
      console.log(`  ⚠️  Circular dependencies detected: ${deps.cycles.length}`);
    }
  }

  graph.close();
}

// Example 4: Convenience functions
function convenienceFunctionsExample() {
  console.log("\n=== Convenience Functions ===\n");

  // Quick codebase analysis
  const analysis = analyzeCodebase("./test_repo");

  console.log("Codebase Analysis:");
  console.log(`  Total Files: ${analysis.stats.totalFiles}`);
  console.log(`  Total Symbols: ${analysis.stats.totalSymbols}`);
  console.log(`  Cycles Found: ${analysis.cycles.length}`);
  console.log(`  Entry Points: ${analysis.entryPoints.length}`);

  console.log("\nMost Complex Functions:");
  analysis.complexFunctions.slice(0, 5).forEach(func => {
    console.log(`  - ${func.function} (${func.calleesCount} callees)`);
  });

  // Find related code
  if (analysis.entryPoints.length > 0) {
    const related = findRelatedCode("./test_repo", analysis.entryPoints[0]);
    console.log(`\nRelated code for ${related.symbol}:`);
    console.log(`  Callers: ${related.callers.length}`);
    console.log(`  Callees: ${related.callees.length}`);
    console.log(`  Impact: ${related.impact.length} symbols`);
  }
}

// Example 5: Finding cycles
function cycleDetectionExample() {
  console.log("\n=== Cycle Detection ===\n");

  const api = new CodeGraphAPI("./test_repo");

  const cycles = api.findCycles();
  console.log(`Found ${cycles.length} cycles in the call graph`);

  if (cycles.length > 0) {
    console.log("\nExample cycles:");
    cycles.slice(0, 3).forEach((cycle, idx) => {
      console.log(`\nCycle ${idx + 1}:`);
      cycle.forEach(symbol => {
        console.log(`  -> ${symbol}`);
      });
    });
  }

  api.close();
}

// Run all examples
function main() {
  console.log("Consilium CodeGraph TypeScript Client - Examples");
  console.log("=".repeat(50));

  try {
    simpleAPIExample();
    callGraphExample();
    advancedExample();
    convenienceFunctionsExample();
    cycleDetectionExample();

    console.log("\n" + "=".repeat(50));
    console.log("All examples completed successfully!");
  } catch (error) {
    console.error("\nError running examples:", error);
    console.log("\nMake sure you have:");
    console.log("  1. Built the code graph: cargo run -- --repo ./test_repo scan --semantic");
    console.log("  2. The database exists at: ./test_repo/.reviewbot/graph.db");
  }
}

// Run if executed directly
if (require.main === module) {
  main();
}
