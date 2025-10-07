/**
 * Scanner wrapper - Calls the Rust CLI to index a repository
 *
 * This allows TypeScript code to trigger scans without manually running
 * the Rust binary.
 */

import { execSync, exec } from "child_process";
import { existsSync } from "fs";
import { join } from "path";

export interface ScanOptions {
  semantic?: boolean;      // Include semantic analysis
  force?: boolean;         // Force full re-scan (not incremental)
  quiet?: boolean;         // Suppress output
}

export interface ScanResult {
  success: boolean;
  duration: number;        // milliseconds
  output: string;
  error?: string;
}

/**
 * Get the platform-specific binary name and directory
 */
function getPlatformBinary(): { platform: string; binaryName: string } {
  const platform = process.platform;
  const arch = process.arch;

  // Map Node.js platform/arch to our bin directory structure
  let platformDir: string;
  if (platform === "darwin" && arch === "arm64") {
    platformDir = "darwin-arm64";
  } else if (platform === "darwin" && arch === "x64") {
    platformDir = "darwin-x64";
  } else if (platform === "linux" && arch === "x64") {
    platformDir = "linux-x64";
  } else if (platform === "win32" && arch === "x64") {
    platformDir = "win32-x64";
  } else {
    platformDir = `${platform}-${arch}`;
  }

  const binaryName = platform === "win32" ? "reviewbot.exe" : "reviewbot";

  return { platform: platformDir, binaryName };
}

/**
 * Find the Rust CLI binary
 */
function findRustCLI(): string | null {
  const { platform, binaryName } = getPlatformBinary();

  // 1. FIRST: Try bundled binary (distributed with npm package)
  // This is relative to the compiled JS in dist/
  const bundledPaths = [
    join(__dirname, "..", "bin", platform, binaryName),
    join(__dirname, "..", "..", "bin", platform, binaryName),
  ];

  for (const path of bundledPaths) {
    if (existsSync(path)) {
      return path;
    }
  }

  // 2. FALLBACK: Try development/build locations
  const possiblePaths = [
    // Development build
    join(process.cwd(), "target", "release", binaryName),
    join(process.cwd(), "target", "debug", binaryName),

    // Installed location (if ts-client is in the repo)
    join(process.cwd(), "..", "target", "release", binaryName),
    join(process.cwd(), "..", "target", "debug", binaryName),

    // Two levels up (if using from node_modules)
    join(process.cwd(), "..", "..", "target", "release", binaryName),

    // Global install
    "/usr/local/bin/reviewbot",
    join(process.env.HOME || "", ".cargo", "bin", binaryName),
  ];

  for (const path of possiblePaths) {
    if (existsSync(path)) {
      return path;
    }
  }

  // 3. LAST: Try PATH
  try {
    const whichCmd = process.platform === "win32" ? "where" : "which";
    execSync(`${whichCmd} reviewbot`, { stdio: "pipe" });
    return "reviewbot";
  } catch {
    return null;
  }
}

/**
 * Scan a repository to build the code graph.
 *
 * This runs the Rust CLI to parse the codebase and create the SQLite database.
 *
 * @param repoPath - Path to repository to scan
 * @param options - Scan options
 * @param cliPath - Optional path to the Rust CLI binary
 *
 * @example
 * ```typescript
 * import { scanRepository } from "@consilium/codegraph-client";
 *
 * // Simple scan
 * await scanRepository("./my-project");
 *
 * // With options
 * await scanRepository("./my-project", {
 *   semantic: true,
 *   quiet: false
 * });
 * ```
 */
export async function scanRepository(
  repoPath: string,
  options: ScanOptions = {},
  cliPath?: string
): Promise<ScanResult> {
  const binary = cliPath || findRustCLI();

  if (!binary) {
    throw new Error(
      "Rust CLI not found. Please build it first:\n" +
      "  cd /path/to/consilium-codegraph\n" +
      "  cargo build --release\n\n" +
      "Or provide the path via the cliPath parameter."
    );
  }

  // Build command
  const args = ["--repo", repoPath, "scan"];

  if (options.semantic) {
    args.push("--semantic");
  }

  const startTime = Date.now();

  return new Promise((resolve) => {
    const cmd = `${binary} ${args.join(" ")}`;

    exec(cmd, { maxBuffer: 10 * 1024 * 1024 }, (error, stdout, stderr) => {
      const duration = Date.now() - startTime;

      if (error) {
        resolve({
          success: false,
          duration,
          output: stdout + stderr,
          error: error.message
        });
      } else {
        if (!options.quiet) {
          console.log(stdout);
          if (stderr) console.error(stderr);
        }

        resolve({
          success: true,
          duration,
          output: stdout + stderr
        });
      }
    });
  });
}

/**
 * Synchronous version of scanRepository.
 * Blocks until scan completes.
 *
 * @param repoPath - Path to repository to scan
 * @param options - Scan options
 * @param cliPath - Optional path to the Rust CLI binary
 *
 * @example
 * ```typescript
 * import { scanRepositorySync } from "@consilium/codegraph-client";
 *
 * const result = scanRepositorySync("./my-project");
 * if (result.success) {
 *   console.log(`Scan completed in ${result.duration}ms`);
 * }
 * ```
 */
export function scanRepositorySync(
  repoPath: string,
  options: ScanOptions = {},
  cliPath?: string
): ScanResult {
  const binary = cliPath || findRustCLI();

  if (!binary) {
    throw new Error(
      "Rust CLI not found. Please build it first:\n" +
      "  cd /path/to/consilium-codegraph\n" +
      "  cargo build --release\n\n" +
      "Or provide the path via the cliPath parameter."
    );
  }

  // Build command
  const args = ["--repo", repoPath, "scan"];

  if (options.semantic) {
    args.push("--semantic");
  }

  const startTime = Date.now();

  try {
    const output = execSync(`${binary} ${args.join(" ")}`, {
      encoding: "utf-8",
      stdio: options.quiet ? "pipe" : "inherit",
      maxBuffer: 10 * 1024 * 1024
    });

    const duration = Date.now() - startTime;

    return {
      success: true,
      duration,
      output: output.toString()
    };
  } catch (error: any) {
    const duration = Date.now() - startTime;

    return {
      success: false,
      duration,
      output: error.stdout?.toString() || "",
      error: error.message
    };
  }
}

/**
 * Check if a repository has been scanned.
 *
 * @param repoPath - Path to repository
 * @returns True if graph.db exists
 *
 * @example
 * ```typescript
 * import { isScanned, scanRepositorySync } from "@consilium/codegraph-client";
 *
 * if (!isScanned("./my-project")) {
 *   console.log("Scanning repository...");
 *   scanRepositorySync("./my-project");
 * }
 * ```
 */
export function isScanned(repoPath: string): boolean {
  const dbPath = join(repoPath, ".reviewbot", "graph.db");
  return existsSync(dbPath);
}

/**
 * Get information about the Rust CLI
 *
 * @returns CLI path and version info, or null if not found
 */
export function getCLIInfo(): { path: string; version?: string } | null {
  const path = findRustCLI();

  if (!path) {
    return null;
  }

  try {
    const output = execSync(`${path} --version`, { encoding: "utf-8" });
    return {
      path,
      version: output.trim()
    };
  } catch {
    return { path };
  }
}
