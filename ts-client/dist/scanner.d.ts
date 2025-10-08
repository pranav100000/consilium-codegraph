/**
 * Scanner wrapper - Calls the Rust CLI to index a repository
 *
 * This allows TypeScript code to trigger scans without manually running
 * the Rust binary.
 */
export interface ScanOptions {
    semantic?: boolean;
    force?: boolean;
    quiet?: boolean;
}
export interface ScanResult {
    success: boolean;
    duration: number;
    output: string;
    error?: string;
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
export declare function scanRepository(repoPath: string, options?: ScanOptions, cliPath?: string): Promise<ScanResult>;
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
export declare function scanRepositorySync(repoPath: string, options?: ScanOptions, cliPath?: string): ScanResult;
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
export declare function isScanned(repoPath: string): boolean;
/**
 * Get information about the Rust CLI
 *
 * @returns CLI path and version info, or null if not found
 */
export declare function getCLIInfo(): {
    path: string;
    version?: string;
} | null;
//# sourceMappingURL=scanner.d.ts.map