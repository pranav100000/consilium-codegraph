#!/usr/bin/env ts-node
/**
 * Standalone test/demo script for Agent API
 * Run with: npx ts-node test-agent-api.ts
 */

import { AgentCodeGraph } from "./src/agent-api";
import { join } from "path";

const testRepoPath = join(__dirname, "..", "test_repo");

console.log("🧪 Testing Agent API\n");

const agent = new AgentCodeGraph(testRepoPath);

console.log("1️⃣  Finding symbols with 'User' in name:");
const userSymbols = agent.findSymbols("User");
console.log(`   Found ${userSymbols.length} symbols`);
userSymbols.slice(0, 3).forEach((sym) => {
  console.log(`   - ${sym.name} (${sym.kind}) at ${sym.location.file}:${sym.location.line}`);
});

console.log("\n2️⃣  Finding all functions:");
const functions = agent.findSymbols("", { kind: ["Function"] });
console.log(`   Found ${functions.length} functions`);
functions.slice(0, 3).forEach((sym) => {
  console.log(`   - ${sym.name} at ${sym.location.file}:${sym.location.line}`);
});

if (functions.length > 0) {
  console.log("\n3️⃣  Getting detailed info for first function:");
  const info = agent.getSymbol(functions[0].fqn, {
    includeCallers: true,
    includeCallees: true,
  });
  console.log(`   Name: ${info?.name}`);
  console.log(`   Kind: ${info?.kind}`);
  console.log(`   Location: ${info?.location.file}:${info?.location.line}`);
  console.log(`   Callers: ${info?.callers?.length || 0}`);
  console.log(`   Callees: ${info?.callees?.length || 0}`);

  if (info?.callers && info.callers.length > 0) {
    console.log("\n4️⃣  Navigating call graph (who calls this):");
    const callers = agent.queryRelationships(functions[0].fqn, "called-by", { depth: 2 });
    console.log(`   Found ${callers.length} callers (depth ≤ 2)`);
    callers.slice(0, 5).forEach((caller) => {
      console.log(`   - ${caller.symbol} at depth ${caller.depth}`);
    });
  }

  console.log("\n5️⃣  Navigating call graph (what this calls):");
  const callees = agent.queryRelationships(functions[0].fqn, "calls", { depth: 2 });
  console.log(`   Found ${callees.length} callees (depth ≤ 2)`);
  callees.slice(0, 5).forEach((callee) => {
    console.log(`   - ${callee.symbol} at depth ${callee.depth}`);
  });
}

console.log("\n6️⃣  Testing error handling:");
const nonExistent = agent.getSymbol("This::Does::Not::Exist");
console.log(`   Non-existent symbol: ${nonExistent === null ? "✓ null" : "✗ unexpected result"}`);

const noMatches = agent.findSymbols("ThisDefinitelyDoesNotExist12345");
console.log(`   No matches search: ${noMatches.length === 0 ? "✓ empty array" : "✗ unexpected result"}`);

agent.close();

console.log("\n✅ All tests completed!");
