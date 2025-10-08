/**
 * Simplified Code Graph API for agents - focused on database queries
 */
import { Symbol, Edge, GraphStats } from "./models";
/**
 * Simple API for querying the code graph database.
 * Designed to run on the same server as agents - no auth needed.
 */
export declare class CodeGraphAPI {
    private db;
    private dbPath;
    /**
     * Initialize the API for a repository.
     *
     * @param repoPath - Path to the repository root
     * @param dbPath - Path to the graph database (default: .reviewbot/graph.db)
     * @param semantic - Whether to recommend semantic analysis (default: true)
     */
    constructor(repoPath: string, dbPath?: string, semantic?: boolean);
    /**
     * Get a symbol by its fully qualified name.
     */
    getSymbol(fqn: string): Symbol | null;
    /**
     * Search for symbols by name pattern.
     */
    findSymbols(pattern: string, kind?: string): Symbol[];
    /**
     * Get all symbols in a file.
     */
    getFileSymbols(filePath: string): Symbol[];
    /**
     * Get all functions that call this symbol.
     */
    getCallers(symbol: string): string[];
    /**
     * Get all functions called by this symbol.
     */
    getCallees(symbol: string): string[];
    /**
     * Get edges with optional filters.
     */
    getEdges(source?: string, target?: string, edgeType?: string): Edge[];
    /**
     * Find all paths between two symbols.
     */
    findPaths(start: string, end: string, maxDepth?: number): string[][];
    /**
     * Get all dependencies of a symbol.
     */
    getDependencies(symbol: string): Record<string, string[]>;
    /**
     * Get all symbols that would be affected if this symbol changes.
     */
    getImpactRadius(symbol: string, maxDepth?: number): Set<string>;
    /**
     * Get overall statistics about the code graph.
     */
    getStats(): GraphStats;
    /**
     * Find all cycles in the call graph.
     */
    findCycles(): string[][];
    /**
     * Begin an explicit transaction.
     */
    beginTransaction(): void;
    /**
     * Commit the current transaction.
     */
    commit(): void;
    /**
     * Rollback the current transaction.
     */
    rollback(): void;
    /**
     * Close the database connection.
     */
    close(): void;
}
/**
 * Quick analysis of a codebase for agents.
 *
 * @returns Dictionary with key metrics and insights
 */
export declare function analyzeCodebase(repoPath: string): {
    stats: GraphStats;
    cycles: string[][];
    entryPoints: string[];
    complexFunctions: Array<{
        function: string;
        calleesCount: number;
    }>;
};
/**
 * Find all code related to a symbol.
 *
 * @returns Dictionary with callers, callees, and dependencies
 */
export declare function findRelatedCode(repoPath: string, symbol: string): {
    symbol: string;
    callers: string[];
    callees: string[];
    dependencies: Record<string, string[]>;
    impact: string[];
};
//# sourceMappingURL=code-graph-api.d.ts.map