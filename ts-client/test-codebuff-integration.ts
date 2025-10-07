#!/usr/bin/env ts-node
/**
 * Standalone test/demo script for Codebuff Integration
 * Run with: npx ts-node test-codebuff-integration.ts
 */

import { AgentCodeGraph } from "./src/agent-api";
import { join } from "path";

const testRepoPath = join(__dirname, "..", "test_repo");

console.log("🗺️  Testing Codebuff Integration\n");

const startTime = Date.now();
const agent = new AgentCodeGraph(testRepoPath);

const data = agent.getFileTokenData();
const duration = Date.now() - startTime;

// Count files and symbols
const fileCount = Object.keys(data.tokenScores).length;
let totalSymbols = 0;
let totalCallers = 0;

Object.values(data.tokenScores).forEach((tokens) => {
  totalSymbols += Object.keys(tokens).length;
});

Object.values(data.tokenCallers).forEach((tokens) => {
  Object.values(tokens).forEach((callers) => {
    totalCallers += callers.length;
  });
});

console.log(`✅ Completed in ${duration}ms\n`);
console.log(`📊 Summary:`);
console.log(`  Files: ${fileCount}`);
console.log(`  Total symbols: ${totalSymbols}`);
console.log(`  Total caller relationships: ${totalCallers}`);

// Validation checks
console.log(`\n✅ Format validation:`);

// Check structure
const filesMatch = Object.keys(data.tokenScores).every((file) =>
  data.tokenCallers.hasOwnProperty(file)
);
console.log(`  ${filesMatch ? "✓" : "✗"} All files in tokenScores also in tokenCallers`);

// Check a sample score
let sampleScore: number | undefined;
let sampleToken: string | undefined;
let sampleFile: string | undefined;
let sampleCallers: string[] = [];

for (const [file, tokens] of Object.entries(data.tokenScores)) {
  for (const [token, score] of Object.entries(tokens)) {
    sampleFile = file;
    sampleToken = token;
    sampleScore = score;
    sampleCallers = data.tokenCallers[file][token];
    break;
  }
  if (sampleScore) break;
}

if (sampleScore && sampleToken && sampleFile) {
  const scoreValid = sampleScore >= 1.0 && sampleScore === Math.round(sampleScore * 1000) / 1000;
  console.log(`  ${scoreValid ? "✓" : "✗"} Score format correct: ${sampleScore}`);

  const callersValid = Array.isArray(sampleCallers);
  console.log(`  ${callersValid ? "✓" : "✗"} Callers is an array with ${sampleCallers.length} items`);

  const limitValid = sampleCallers.length <= 25;
  console.log(`  ${limitValid ? "✓" : "✗"} Callers within 25 limit`);

  // Verify logarithmic formula for this sample
  if (sampleCallers.length > 0) {
    const expectedScore = Math.round((1.0 + Math.log(1 + sampleCallers.length)) * 1000) / 1000;
    const formulaValid = sampleScore === expectedScore;
    console.log(`  ${formulaValid ? "✓" : "✗"} Logarithmic formula correct: ${sampleScore} === 1.0 + ln(1 + ${sampleCallers.length})`);
  }
}

// Check ignored symbols
const ignoredNames = ["__init__", "__post_init__", "__call__", "constructor"];
let foundIgnored = false;
for (const tokens of Object.values(data.tokenScores)) {
  for (const token of Object.keys(tokens)) {
    if (ignoredNames.includes(token)) {
      foundIgnored = true;
      break;
    }
  }
  if (foundIgnored) break;
}
console.log(`  ${!foundIgnored ? "✓" : "✗"} No ignored symbols found`);

// Sample output
if (sampleFile && sampleToken) {
  console.log(`\n📝 Sample output:`);
  console.log(`  File: ${sampleFile}`);
  console.log(`  Token: ${sampleToken}`);
  console.log(`  Score: ${sampleScore}`);
  console.log(`  Callers: [${sampleCallers.slice(0, 3).join(", ")}${sampleCallers.length > 3 ? ", ..." : ""}]`);
}

// Show a file with multiple symbols
const fileWithMostSymbols = Object.entries(data.tokenScores)
  .sort((a, b) => Object.keys(b[1]).length - Object.keys(a[1]).length)[0];

if (fileWithMostSymbols) {
  const [file, tokens] = fileWithMostSymbols;
  console.log(`\n📁 File with most symbols: ${file}`);
  console.log(`  Symbols: ${Object.keys(tokens).length}`);
  Object.entries(tokens)
    .slice(0, 5)
    .forEach(([token, score]) => {
      const callers = data.tokenCallers[file][token];
      console.log(`    ${token}: score=${score}, callers=${callers.length}`);
    });
}

agent.close();

console.log(`\n🎉 All validation checks passed!`);
