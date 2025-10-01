/**
 * Tests for CodeGraphAPI
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { CodeGraphAPI } from "../src/code-graph-api";
import { join } from "path";

describe("CodeGraphAPI", () => {
  let api: CodeGraphAPI;
  const testRepoPath = join(__dirname, "..", "..", "test_repo");

  beforeAll(() => {
    // Initialize API with test repository
    api = new CodeGraphAPI(testRepoPath);
  });

  afterAll(() => {
    // Clean up
    api.close();
  });

  describe("Symbol Queries", () => {
    it("should get symbol by FQN", () => {
      const symbols = api.findSymbols("", undefined);
      if (symbols.length > 0) {
        const symbol = api.getSymbol(symbols[0].fqn);
        expect(symbol).not.toBeNull();
        expect(symbol?.fqn).toBe(symbols[0].fqn);
      }
    });

    it("should find symbols by pattern", () => {
      const symbols = api.findSymbols("User");
      expect(Array.isArray(symbols)).toBe(true);
      symbols.forEach((sym) => {
        expect(sym).toHaveProperty("fqn");
        expect(sym).toHaveProperty("name");
        expect(sym).toHaveProperty("kind");
        expect(sym).toHaveProperty("location");
      });
    });

    it("should get file symbols", () => {
      const symbols = api.findSymbols("", undefined);
      if (symbols.length > 0) {
        const file = symbols[0].location.file;
        const fileSymbols = api.getFileSymbols(file);
        expect(Array.isArray(fileSymbols)).toBe(true);
        fileSymbols.forEach((sym) => {
          expect(sym.location.file).toBe(file);
        });
      }
    });
  });

  describe("Relationship Queries", () => {
    it("should get callers", () => {
      const symbols = api.findSymbols("", "function");
      if (symbols.length > 0) {
        const callers = api.getCallers(symbols[0].fqn);
        expect(Array.isArray(callers)).toBe(true);
      }
    });

    it("should get callees", () => {
      const symbols = api.findSymbols("", "function");
      if (symbols.length > 0) {
        const callees = api.getCallees(symbols[0].fqn);
        expect(Array.isArray(callees)).toBe(true);
      }
    });

    it("should get edges with filters", () => {
      const edges = api.getEdges();
      expect(Array.isArray(edges)).toBe(true);
      edges.forEach((edge) => {
        expect(edge).toHaveProperty("source");
        expect(edge).toHaveProperty("target");
        expect(edge).toHaveProperty("edgeType");
      });
    });

    it("should get dependencies", () => {
      const symbols = api.findSymbols("", undefined);
      if (symbols.length > 0) {
        const deps = api.getDependencies(symbols[0].fqn);
        expect(deps).toHaveProperty("imports");
        expect(deps).toHaveProperty("calls");
        expect(deps).toHaveProperty("uses");
      }
    });
  });

  describe("Analysis Queries", () => {
    it("should calculate impact radius", () => {
      const symbols = api.findSymbols("", "function");
      if (symbols.length > 0) {
        const impact = api.getImpactRadius(symbols[0].fqn, 2);
        expect(impact instanceof Set).toBe(true);
      }
    });

    it("should find paths between symbols", () => {
      const symbols = api.findSymbols("", "function");
      if (symbols.length >= 2) {
        const paths = api.findPaths(symbols[0].fqn, symbols[1].fqn, 5);
        expect(Array.isArray(paths)).toBe(true);
      }
    });
  });

  describe("Statistics", () => {
    it("should get graph statistics", () => {
      const stats = api.getStats();
      expect(stats).toHaveProperty("symbolsByKind");
      expect(stats).toHaveProperty("edgesByType");
      expect(stats).toHaveProperty("totalFiles");
      expect(stats).toHaveProperty("totalSymbols");
      expect(stats).toHaveProperty("totalEdges");
      expect(typeof stats.totalFiles).toBe("number");
      expect(typeof stats.totalSymbols).toBe("number");
      expect(typeof stats.totalEdges).toBe("number");
    });

    it("should find cycles", () => {
      const cycles = api.findCycles();
      expect(Array.isArray(cycles)).toBe(true);
    });
  });

  describe("Error Handling", () => {
    it("should return null for non-existent symbol", () => {
      const symbol = api.getSymbol("NonExistent::Symbol::That::Does::Not::Exist");
      expect(symbol).toBeNull();
    });

    it("should return empty array for non-matching pattern", () => {
      const symbols = api.findSymbols("ThisSymbolDefinitelyDoesNotExistInTheCodebase");
      expect(symbols).toEqual([]);
    });
  });
});
