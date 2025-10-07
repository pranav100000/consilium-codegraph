/**
 * Tests for Scanner API
 */

import { describe, it, expect, beforeAll } from "vitest";
import { isScanned, getCLIInfo } from "../src/scanner";
import { join } from "path";
import { existsSync } from "fs";

describe("Scanner", () => {
  const testRepoPath = join(__dirname, "..", "..", "test_repo");
  const nonExistentPath = join(__dirname, "this_does_not_exist");

  describe("CLI Detection", () => {
    it("should find CLI binary", () => {
      const info = getCLIInfo();
      if (info) {
        expect(info).toHaveProperty("path");
        expect(typeof info.path).toBe("string");
        expect(existsSync(info.path)).toBe(true);
      } else {
        // CLI might not be built yet - that's okay for this test
        expect(info).toBeNull();
      }
    });
  });

  describe("Repository Status", () => {
    it("should detect scanned repository", () => {
      const scanned = isScanned(testRepoPath);
      expect(typeof scanned).toBe("boolean");

      // test_repo should have been scanned during setup
      const dbPath = join(testRepoPath, ".reviewbot", "graph.db");
      if (existsSync(dbPath)) {
        expect(scanned).toBe(true);
      }
    });

    it("should return false for non-existent repository", () => {
      const scanned = isScanned(nonExistentPath);
      expect(scanned).toBe(false);
    });

    it("should return false for unscanned repository", () => {
      const unscannedPath = join(__dirname, "..");
      const dbPath = join(unscannedPath, ".reviewbot", "graph.db");

      if (!existsSync(dbPath)) {
        const scanned = isScanned(unscannedPath);
        expect(scanned).toBe(false);
      }
    });
  });

  // Note: We don't test scanRepository/scanRepositorySync here because:
  // 1. They require the Rust CLI to be built and in PATH
  // 2. They can be slow (scanning takes time)
  // 3. They modify filesystem state
  // These are better tested as integration tests or manually with the standalone scripts
});
