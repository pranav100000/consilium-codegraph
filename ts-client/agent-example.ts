#!/usr/bin/env node
/**
 * Example: Using AgentCodeGraph API with existing file tools
 *
 * This demonstrates how the graph API complements Read/Grep/Glob
 * to help an AI agent navigate and understand a codebase.
 */

import { AgentCodeGraph } from "./src/agent-api";
import { readFileSync } from "fs";

const repoPath = "/Users/pranavsharan/Developer/consilium-codegraph";

console.log("🤖 AI Agent Code Navigation Example\n");
console.log("=".repeat(70));

// ========== Scenario: "Understand how authentication works" ==========

console.log("\n📋 Task: Understand how authentication works in this codebase\n");

const agent = new AgentCodeGraph(repoPath);

// Step 1: Find auth-related symbols (better than Grep for structure)
console.log("1️⃣  Finding authentication-related symbols...\n");
const authSymbols = agent.findSymbols("auth", { kind: ["Function", "Class", "Method"] });

console.log(`   Found ${authSymbols.length} symbols with 'auth' in name:`);
authSymbols.slice(0, 5).forEach(s => {
  console.log(`   - ${s.name} (${s.kind}) in ${s.location.file}:${s.location.line}`);
});

if (authSymbols.length === 0) {
  console.log("\n   No auth symbols found. Trying broader search...\n");
  const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 10 });
  console.log(`   Found ${allFunctions.length} functions to analyze instead:\n`);
  allFunctions.slice(0, 3).forEach(s => {
    console.log(`   - ${s.name} in ${s.location.file}:${s.location.line}`);
  });
}

// Step 2: Get detailed info about a specific symbol
console.log("\n2️⃣  Getting detailed info about a symbol...\n");

const targetSymbol = authSymbols.length > 0
  ? authSymbols[0]
  : agent.findSymbols("", { kind: ["Function"], limit: 1 })[0];

if (targetSymbol) {
  const symbolInfo = agent.getSymbol(targetSymbol.fqn, {
    includeCallers: true,
    includeCallees: true,
  });

  if (symbolInfo) {
    console.log(`   Symbol: ${symbolInfo.name}`);
    console.log(`   Kind: ${symbolInfo.kind}`);
    console.log(`   Location: ${symbolInfo.location.file}:${symbolInfo.location.line}`);
    console.log(`   Signature: ${symbolInfo.signature || "N/A"}`);

    if (symbolInfo.containedIn) {
      console.log(`   Contained in: ${symbolInfo.containedIn}`);
    }

    console.log(`\n   This function:`);
    console.log(`   - Calls ${symbolInfo.callees?.length || 0} other functions`);
    console.log(`   - Is called by ${symbolInfo.callers?.length || 0} functions`);

    // Step 3: Read the actual source code (using standard file tools)
    console.log("\n3️⃣  Reading source code (simulated Read tool)...\n");

    try {
      const sourceFile = readFileSync(symbolInfo.location.file, 'utf-8');
      const lines = sourceFile.split('\n');
      const startLine = Math.max(0, symbolInfo.location.line - 2);
      const endLine = Math.min(lines.length, symbolInfo.location.line + 8);

      console.log(`   Source code preview (${symbolInfo.location.file}:${symbolInfo.location.line}):\n`);
      for (let i = startLine; i < endLine; i++) {
        const marker = i === symbolInfo.location.line ? '→' : ' ';
        console.log(`   ${marker} ${String(i + 1).padStart(4)}: ${lines[i]}`);
      }
    } catch (err) {
      console.log(`   (Could not read file: ${err})`);
    }

    // Step 4: Navigate the call graph
    if (symbolInfo.callees && symbolInfo.callees.length > 0) {
      console.log("\n4️⃣  Exploring what this function calls...\n");

      symbolInfo.callees.slice(0, 3).forEach(calleeFqn => {
        const calleeInfo = agent.getSymbol(calleeFqn);
        if (calleeInfo) {
          console.log(`   → Calls: ${calleeInfo.name} (${calleeInfo.kind})`);
          console.log(`      at ${calleeInfo.location.file}:${calleeInfo.location.line}`);
        }
      });

      if (symbolInfo.callees.length > 3) {
        console.log(`   ... and ${symbolInfo.callees.length - 3} more`);
      }
    }

    // Step 5: Deep traversal - find transitive dependencies
    console.log("\n5️⃣  Finding transitive call dependencies (depth=2)...\n");

    const dependencies = agent.queryRelationships(symbolInfo.fqn, "calls", { depth: 2, limit: 10 });

    console.log(`   Found ${dependencies.length} transitive dependencies:\n`);

    const byDepth = dependencies.reduce((acc, dep) => {
      acc[dep.depth] = acc[dep.depth] || [];
      acc[dep.depth].push(dep);
      return acc;
    }, {} as Record<number, typeof dependencies>);

    Object.entries(byDepth).forEach(([depth, deps]) => {
      console.log(`   Depth ${depth}:`);
      deps.slice(0, 3).forEach(dep => {
        console.log(`     - ${dep.symbol} (${dep.kind})`);
      });
      if (deps.length > 3) {
        console.log(`     ... and ${deps.length - 3} more`);
      }
    });

    // Step 6: Find who calls this (impact analysis)
    console.log("\n6️⃣  Finding who calls this function (impact analysis)...\n");

    const callers = agent.queryRelationships(symbolInfo.fqn, "called-by", { depth: 2, limit: 10 });

    if (callers.length > 0) {
      console.log(`   This function is called by ${callers.length} other symbols:\n`);
      callers.slice(0, 5).forEach(caller => {
        console.log(`   - ${caller.symbol} (depth: ${caller.depth})`);
      });

      console.log(`\n   💡 Impact: Changing this function affects ${callers.length} callers`);
    } else {
      console.log(`   ⚠️  No callers found - this might be an entry point or unused code`);
    }
  }
}

// ========== Example: Combining with Grep ==========

console.log("\n\n" + "=".repeat(70));
console.log("\n🔍 Advanced: Combining Graph API with Text Search\n");

// Scenario: Find all functions that might handle errors
console.log("Task: Find error-handling code\n");

console.log("1. Use findSymbols to find functions with 'error' in name:");
const errorFunctions = agent.findSymbols("error", { kind: ["Function", "Method"], limit: 5 });
console.log(`   Found ${errorFunctions.length} error-related functions\n`);

errorFunctions.forEach(fn => {
  console.log(`   - ${fn.name} in ${fn.location.file}:${fn.location.line}`);

  // Then use Read to check the actual implementation
  console.log(`     (You would use Read tool here to see the code)`);
});

// ========== Example: Finding Entry Points ==========

console.log("\n\n" + "=".repeat(70));
console.log("\n🚪 Finding Entry Points\n");

const allFunctions = agent.findSymbols("", { kind: ["Function"], limit: 50 });

console.log(`Analyzing ${allFunctions.length} functions...\n`);

const entryPoints = allFunctions.filter(fn => {
  const info = agent.getSymbol(fn.fqn, { includeCallers: true });
  return info && (!info.callers || info.callers.length === 0);
});

console.log(`Found ${entryPoints.length} potential entry points (functions with no callers):\n`);

entryPoints.slice(0, 5).forEach(ep => {
  console.log(`   - ${ep.name} in ${ep.location.file}:${ep.location.line}`);
});

console.log("\n\n" + "=".repeat(70));
console.log("\n✅ Example complete!\n");

console.log("Key Takeaways:");
console.log("  • Use findSymbols() to search for code entities (better than grep for structure)");
console.log("  • Use getSymbol() to understand relationships (what calls what)");
console.log("  • Use queryRelationships() to traverse the graph (deep dependencies)");
console.log("  • Combine with Read/Grep for source code and text search");
console.log("");

agent.close();
