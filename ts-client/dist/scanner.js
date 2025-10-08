"use strict";
/**
 * Scanner wrapper - Calls the Rust CLI to index a repository
 *
 * This allows TypeScript code to trigger scans without manually running
 * the Rust binary.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.scanRepository = scanRepository;
exports.scanRepositorySync = scanRepositorySync;
exports.isScanned = isScanned;
exports.getCLIInfo = getCLIInfo;
const child_process_1 = require("child_process");
const fs_1 = require("fs");
const path_1 = require("path");
/**
 * Get the platform-specific binary name and directory
 */
function getPlatformBinary() {
    const platform = process.platform;
    const arch = process.arch;
    // Map Node.js platform/arch to our bin directory structure
    let platformDir;
    if (platform === "darwin" && arch === "arm64") {
        platformDir = "darwin-arm64";
    }
    else if (platform === "darwin" && arch === "x64") {
        platformDir = "darwin-x64";
    }
    else if (platform === "linux" && arch === "x64") {
        platformDir = "linux-x64";
    }
    else if (platform === "win32" && arch === "x64") {
        platformDir = "win32-x64";
    }
    else {
        platformDir = `${platform}-${arch}`;
    }
    const binaryName = platform === "win32" ? "reviewbot.exe" : "reviewbot";
    return { platform: platformDir, binaryName };
}
/**
 * Find the Rust CLI binary
 */
function findRustCLI() {
    const { platform, binaryName } = getPlatformBinary();
    // 1. FIRST: Try bundled binary (distributed with npm package)
    // This is relative to the compiled JS in dist/
    const bundledPaths = [
        (0, path_1.join)(__dirname, "..", "bin", platform, binaryName),
        (0, path_1.join)(__dirname, "..", "..", "bin", platform, binaryName),
    ];
    for (const path of bundledPaths) {
        if ((0, fs_1.existsSync)(path)) {
            return path;
        }
    }
    // 2. FALLBACK: Try development/build locations
    const possiblePaths = [
        // Development build
        (0, path_1.join)(process.cwd(), "target", "release", binaryName),
        (0, path_1.join)(process.cwd(), "target", "debug", binaryName),
        // Installed location (if ts-client is in the repo)
        (0, path_1.join)(process.cwd(), "..", "target", "release", binaryName),
        (0, path_1.join)(process.cwd(), "..", "target", "debug", binaryName),
        // Two levels up (if using from node_modules)
        (0, path_1.join)(process.cwd(), "..", "..", "target", "release", binaryName),
        // Global install
        "/usr/local/bin/reviewbot",
        (0, path_1.join)(process.env.HOME || "", ".cargo", "bin", binaryName),
    ];
    for (const path of possiblePaths) {
        if ((0, fs_1.existsSync)(path)) {
            return path;
        }
    }
    // 3. LAST: Try PATH
    try {
        const whichCmd = process.platform === "win32" ? "where" : "which";
        (0, child_process_1.execSync)(`${whichCmd} reviewbot`, { stdio: "pipe" });
        return "reviewbot";
    }
    catch {
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
async function scanRepository(repoPath, options = {}, cliPath) {
    const binary = cliPath || findRustCLI();
    if (!binary) {
        throw new Error("Rust CLI not found. Please build it first:\n" +
            "  cd /path/to/consilium-codegraph\n" +
            "  cargo build --release\n\n" +
            "Or provide the path via the cliPath parameter.");
    }
    // Build command
    const args = ["--repo", repoPath, "scan"];
    if (options.semantic) {
        args.push("--semantic");
    }
    const startTime = Date.now();
    return new Promise((resolve) => {
        const cmd = `${binary} ${args.join(" ")}`;
        (0, child_process_1.exec)(cmd, { maxBuffer: 10 * 1024 * 1024 }, (error, stdout, stderr) => {
            const duration = Date.now() - startTime;
            if (error) {
                resolve({
                    success: false,
                    duration,
                    output: stdout + stderr,
                    error: error.message
                });
            }
            else {
                if (!options.quiet) {
                    console.log(stdout);
                    if (stderr)
                        console.error(stderr);
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
function scanRepositorySync(repoPath, options = {}, cliPath) {
    const binary = cliPath || findRustCLI();
    if (!binary) {
        throw new Error("Rust CLI not found. Please build it first:\n" +
            "  cd /path/to/consilium-codegraph\n" +
            "  cargo build --release\n\n" +
            "Or provide the path via the cliPath parameter.");
    }
    // Build command
    const args = ["--repo", repoPath, "scan"];
    if (options.semantic) {
        args.push("--semantic");
    }
    const startTime = Date.now();
    try {
        const output = (0, child_process_1.execSync)(`${binary} ${args.join(" ")}`, {
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
    }
    catch (error) {
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
function isScanned(repoPath) {
    const dbPath = (0, path_1.join)(repoPath, ".reviewbot", "graph.db");
    return (0, fs_1.existsSync)(dbPath);
}
/**
 * Get information about the Rust CLI
 *
 * @returns CLI path and version info, or null if not found
 */
function getCLIInfo() {
    const path = findRustCLI();
    if (!path) {
        return null;
    }
    try {
        const output = (0, child_process_1.execSync)(`${path} --version`, { encoding: "utf-8" });
        return {
            path,
            version: output.trim()
        };
    }
    catch {
        return { path };
    }
}
//# sourceMappingURL=scanner.js.map