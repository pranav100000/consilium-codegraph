/**
 * Tests for AgentCodeGraph API
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { AgentCodeGraph } from "../src/agent-api";
import { join } from "path";

describe("AgentCodeGraph", () => {
  let agent: AgentCodeGraph;
  const testRepoPath = join(__dirname, "..", "..", "test_repo");

  beforeAll(() => {
    // Initialize agent API with test repository
    agent = new AgentCodeGraph(testRepoPath);
  });

  afterAll(() => {
    // Clean up
    agent.close();
  });

  describe("Symbol Search", () => {
    it("should find symbols by name", () => {
      const symbols = agent.findSymbols("User");
      expect(Array.isArray(symbols)).toBe(true);
      symbols.forEach((sym) => {
        expect(sym).toHaveProperty("fqn");
        expect(sym).toHaveProperty("name");
        expect(sym).toHaveProperty("kind");
        expect(sym).toHaveProperty("location");
        expect(sym.location).toHaveProperty("file");
        expect(sym.location).toHaveProperty("line");
      });
    });

    it("should filter symbols by kind", () => {
      const functions = agent.findSymbols("", { kind: ["Function"] });
      expect(Array.isArray(functions)).toBe(true);
      functions.forEach((sym) => {
        expect(sym.kind).toBe("Function");
      });
    });

    it("should filter symbols by file", () => {
      const allSymbols = agent.findSymbols("");
      if (allSymbols.length > 0) {
        const testFile = allSymbols[0].location.file;
        const fileSymbols = agent.findSymbols("", { inFile: testFile });
        expect(Array.isArray(fileSymbols)).toBe(true);
        fileSymbols.forEach((sym) => {
          expect(sym.location.file).toBe(testFile);
        });
      }
    });

    it("should limit results", () => {
      const symbols = agent.findSymbols("", { limit: 5 });
      expect(symbols.length).toBeLessThanOrEqual(5);
    });
  });

  describe("Symbol Context", () => {
    it("should get symbol with basic info", () => {
      const symbols = agent.findSymbols("");
      if (symbols.length > 0) {
        const info = agent.getSymbol(symbols[0].fqn);
        expect(info).not.toBeNull();
        expect(info?.fqn).toBe(symbols[0].fqn);
        expect(info?.name).toBe(symbols[0].name);
        expect(info?.kind).toBe(symbols[0].kind);
        expect(info?.location.file).toBe(symbols[0].location.file);
        expect(info?.location.line).toBe(symbols[0].location.line);
      }
    });

    it("should include callers when requested", () => {
      const symbols = agent.findSymbols("", { kind: ["Function"] });
      if (symbols.length > 0) {
        const info = agent.getSymbol(symbols[0].fqn, { includeCallers: true });
        expect(info).toHaveProperty("callers");
        expect(Array.isArray(info?.callers)).toBe(true);
      }
    });

    it("should include callees when requested", () => {
      const symbols = agent.findSymbols("", { kind: ["Function"] });
      if (symbols.length > 0) {
        const info = agent.getSymbol(symbols[0].fqn, { includeCallees: true });
        expect(info).toHaveProperty("callees");
        expect(Array.isArray(info?.callees)).toBe(true);
      }
    });

    it("should include imports when requested", () => {
      const symbols = agent.findSymbols("");
      if (symbols.length > 0) {
        const info = agent.getSymbol(symbols[0].fqn, { includeImports: true });
        expect(info).toHaveProperty("imports");
        expect(Array.isArray(info?.imports)).toBe(true);
      }
    });

    it("should include contains when requested", () => {
      const symbols = agent.findSymbols("");
      if (symbols.length > 0) {
        const info = agent.getSymbol(symbols[0].fqn, { includeContains: true });
        expect(info).toHaveProperty("contains");
        expect(Array.isArray(info?.contains)).toBe(true);
      }
    });

    it("should always include containedIn", () => {
      const symbols = agent.findSymbols("");
      if (symbols.length > 0) {
        const info = agent.getSymbol(symbols[0].fqn);
        expect(info).toHaveProperty("containedIn");
        // containedIn can be undefined for top-level symbols
      }
    });
  });

  describe("Relationship Navigation", () => {
    it("should query calls relationships", () => {
      const symbols = agent.findSymbols("", { kind: ["Function"] });
      if (symbols.length > 0) {
        const result = agent.queryRelationships(symbols[0].fqn, "calls");
        expect(Array.isArray(result)).toBe(true);
      }
    });

    it("should query called-by relationships", () => {
      const symbols = agent.findSymbols("", { kind: ["Function"] });
      if (symbols.length > 0) {
        const result = agent.queryRelationships(symbols[0].fqn, "called-by");
        expect(Array.isArray(result)).toBe(true);
      }
    });

    it("should query imports relationships", () => {
      const symbols = agent.findSymbols("");
      if (symbols.length > 0) {
        const result = agent.queryRelationships(symbols[0].fqn, "imports");
        expect(Array.isArray(result)).toBe(true);
      }
    });

    it("should respect depth limit", () => {
      const symbols = agent.findSymbols("", { kind: ["Function"] });
      if (symbols.length > 0) {
        const result = agent.queryRelationships(symbols[0].fqn, "calls", { depth: 2 });
        // All relationships should have depth <= 2
        result.forEach((rel) => {
          expect(rel.depth).toBeLessThanOrEqual(2);
        });
      }
    });

    it("should limit results", () => {
      const symbols = agent.findSymbols("", { kind: ["Function"] });
      if (symbols.length > 0) {
        const result = agent.queryRelationships(symbols[0].fqn, "calls", { limit: 10 });
        expect(result.length).toBeLessThanOrEqual(10);
      }
    });

    it("should return relationship nodes with correct structure", () => {
      const symbols = agent.findSymbols("", { kind: ["Function"] });
      if (symbols.length > 0) {
        const result = agent.queryRelationships(symbols[0].fqn, "calls");
        result.forEach((node) => {
          expect(node).toHaveProperty("symbol");
          expect(node).toHaveProperty("kind");
          expect(node).toHaveProperty("location");
          expect(node).toHaveProperty("depth");
          expect(node.location).toHaveProperty("file");
          expect(node.location).toHaveProperty("line");
        });
      }
    });
  });

  describe("Codebuff Integration", () => {
    it("should return file token data", () => {
      const data = agent.getFileTokenData();
      expect(data).toHaveProperty("tokenScores");
      expect(data).toHaveProperty("tokenCallers");
      expect(typeof data.tokenScores).toBe("object");
      expect(typeof data.tokenCallers).toBe("object");
    });

    it("should have consistent structure", () => {
      const data = agent.getFileTokenData();

      // All files in tokenScores should also be in tokenCallers
      Object.keys(data.tokenScores).forEach((file) => {
        expect(data.tokenCallers).toHaveProperty(file);
      });

      // All files in tokenCallers should also be in tokenScores
      Object.keys(data.tokenCallers).forEach((file) => {
        expect(data.tokenScores).toHaveProperty(file);
      });
    });

    it("should have valid scores", () => {
      const data = agent.getFileTokenData();

      Object.entries(data.tokenScores).forEach(([file, tokens]) => {
        Object.entries(tokens).forEach(([token, score]) => {
          // Score should be >= 1.0 (base score)
          expect(score).toBeGreaterThanOrEqual(1.0);
          // Score should be a number with at most 3 decimal places
          expect(score).toBe(Math.round(score * 1000) / 1000);
        });
      });
    });

    it("should have valid caller lists", () => {
      const data = agent.getFileTokenData();

      Object.entries(data.tokenCallers).forEach(([file, tokens]) => {
        Object.entries(tokens).forEach(([token, callers]) => {
          // Callers should be an array
          expect(Array.isArray(callers)).toBe(true);
          // Should not exceed 25 callers (MAX_CALLERS)
          expect(callers.length).toBeLessThanOrEqual(25);
          // All callers should be strings (file paths)
          callers.forEach((caller) => {
            expect(typeof caller).toBe("string");
          });
        });
      });
    });

    it("should not include ignored symbols", () => {
      const data = agent.getFileTokenData();
      const ignoredNames = ["__init__", "__post_init__", "__call__", "constructor"];

      Object.entries(data.tokenScores).forEach(([file, tokens]) => {
        Object.keys(tokens).forEach((token) => {
          expect(ignoredNames).not.toContain(token);
        });
      });
    });

    it("should have matching tokens in scores and callers", () => {
      const data = agent.getFileTokenData();

      Object.entries(data.tokenScores).forEach(([file, tokens]) => {
        Object.keys(tokens).forEach((token) => {
          // Token in tokenScores should also be in tokenCallers
          expect(data.tokenCallers[file]).toHaveProperty(token);
        });
      });

      Object.entries(data.tokenCallers).forEach(([file, tokens]) => {
        Object.keys(tokens).forEach((token) => {
          // Token in tokenCallers should also be in tokenScores
          expect(data.tokenScores[file]).toHaveProperty(token);
        });
      });
    });

    it("should use logarithmic scoring formula", () => {
      const data = agent.getFileTokenData();

      // Find a symbol with callers to test the formula
      let foundSymbolWithCallers = false;

      Object.entries(data.tokenCallers).forEach(([file, tokens]) => {
        Object.entries(tokens).forEach(([token, callers]) => {
          if (callers.length > 0) {
            foundSymbolWithCallers = true;
            const score = data.tokenScores[file][token];
            const numCallers = callers.length;
            // Formula: 1.0 + ln(1 + numCallers)
            const expectedScore = Math.round((1.0 + Math.log(1 + numCallers)) * 1000) / 1000;
            expect(score).toBe(expectedScore);
          }
        });
      });

      // If no symbols with callers found, at least verify base scores
      if (!foundSymbolWithCallers) {
        Object.entries(data.tokenScores).forEach(([file, tokens]) => {
          Object.values(tokens).forEach((score) => {
            // With 0 callers: 1.0 + ln(1 + 0) = 1.0 + ln(1) = 1.0 + 0 = 1.0
            expect(score).toBeGreaterThanOrEqual(1.0);
          });
        });
      }
    });
  });

  describe("Error Handling", () => {
    it("should return null for non-existent symbol", () => {
      const info = agent.getSymbol("NonExistent::Symbol::That::Does::Not::Exist");
      expect(info).toBeNull();
    });

    it("should return empty array for non-matching pattern", () => {
      const symbols = agent.findSymbols("ThisSymbolDefinitelyDoesNotExistInTheCodebase");
      expect(symbols).toEqual([]);
    });

    it("should throw error for invalid relationship types", () => {
      const symbols = agent.findSymbols("");
      if (symbols.length > 0) {
        expect(() => {
          // @ts-expect-error Testing invalid input
          agent.queryRelationships(symbols[0].fqn, "invalidType");
        }).toThrow("Unknown relationship type");
      }
    });
  });
});
