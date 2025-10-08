/**
 * Core CodeGraph class for querying the unified code graph
 */
import { Symbol, SymbolKind, CallPath, DependencyGraph, GraphStats } from "./models";
/**
 * Main interface for querying the code graph.
 * Integrates multiple analyzers and provides unified access.
 */
export declare class CodeGraph {
    private db;
    private dbPath;
    private repoPath;
    private semantic;
    private symbolCache;
    private callgraphCache;
    /**
     * Initialize the code graph for a repository.
     *
     * @param repoPath - Path to the repository root
     * @param dbPath - Path to the graph database (default: .reviewbot/graph.db)
     * @param semantic - Whether to enable semantic analysis with SCIP indexers (default: true)
     */
    constructor(repoPath: string, dbPath?: string, semantic?: boolean);
    /**
     * Initialize database connection
     */
    private initDatabase;
    /**
     * Run consilium scan to build initial graph
     */
    private runInitialScan;
    /**
     * Get symbol details by fully qualified name.
     *
     * @param fqn - Fully qualified name (e.g., "MyClass::myMethod")
     * @returns Symbol object or null if not found
     */
    getSymbol(fqn: string): Symbol | null;
    /**
     * Search symbols by pattern and optional type filter.
     *
     * @param pattern - Search pattern (supports SQL wildcards)
     * @param kind - Optional symbol type filter
     * @param limit - Maximum results to return (default: 100)
     * @returns List of matching symbols
     */
    findSymbols(pattern: string, kind?: SymbolKind, limit?: number): Symbol[];
    /**
     * Get all symbols defined in a file.
     *
     * @param filepath - Path to the file (relative to repo root)
     * @returns List of symbols in the file
     */
    getFileSymbols(filepath: string): Symbol[];
    /**
     * Find all functions that call this symbol.
     *
     * @param symbol - FQN of the symbol
     * @param maxDepth - Maximum call chain depth to explore (default: 1)
     * @returns List of call paths leading to this symbol
     */
    getCallers(symbol: string, maxDepth?: number): CallPath[];
    /**
     * Find all functions called by this symbol.
     *
     * @param symbol - FQN of the symbol
     * @param maxDepth - Maximum call chain depth to explore (default: 1)
     * @returns List of call paths from this symbol
     */
    getCallees(symbol: string, maxDepth?: number): CallPath[];
    /**
     * Get all dependencies of a symbol.
     *
     * @param symbol - FQN of the symbol
     * @returns DependencyGraph showing all dependencies
     */
    getDependencies(symbol: string): DependencyGraph;
    /**
     * Find execution paths between two symbols.
     *
     * @param fromSymbol - Starting symbol FQN
     * @param toSymbol - Target symbol FQN
     * @param maxDepth - Maximum path length (default: 10)
     * @returns List of possible paths (each path is a list of symbols)
     */
    findPath(fromSymbol: string, toSymbol: string, maxDepth?: number): Symbol[][];
    /**
     * Get overall graph statistics
     */
    getStatistics(): GraphStats;
    /**
     * Convert database row to Symbol object
     */
    private rowToSymbol;
    /**
     * Traverse call graph in either direction
     */
    private traverseCalls;
    /**
     * Find cycles in dependency graph using DFS
     */
    private findCyclesFrom;
    /**
     * Clear all caches
     */
    refreshCache(): void;
    /**
     * Close database connection
     */
    close(): void;
}
//# sourceMappingURL=code-graph.d.ts.map