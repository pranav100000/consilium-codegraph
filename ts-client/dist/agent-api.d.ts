/**
 * Agent-focused API for code navigation
 *
 * Designed to complement existing agentic tools (Read, Grep, Glob)
 * by adding the graph/relationship layer.
 */
export interface Location {
    file: string;
    line: number;
    column?: number;
}
export interface SymbolInfo {
    fqn: string;
    name: string;
    kind: string;
    location: Location;
    signature?: string;
    containedIn?: string;
    contains?: string[];
    callers?: string[];
    callees?: string[];
    references?: Location[];
    imports?: string[];
    importedBy?: string[];
}
export interface SymbolSearchResult {
    fqn: string;
    name: string;
    kind: string;
    location: Location;
    signature?: string;
}
export interface RelationshipNode {
    symbol: string;
    kind: string;
    location: Location;
    depth: number;
    path?: string[];
}
export type RelationshipType = "calls" | "called-by" | "imports" | "imported-by" | "implements" | "implemented-by" | "contains" | "contained-in";
export declare class AgentCodeGraph {
    private db;
    private dbPath;
    /**
     * Initialize the agent API for a repository.
     *
     * @param repoPath - Path to the repository root
     * @param dbPath - Path to the graph database (default: .reviewbot/graph.db)
     */
    constructor(repoPath: string, dbPath?: string);
    /**
     * Get detailed information about a symbol by name or FQN.
     *
     * This complements Read/Grep by providing structured symbol information
     * and relationships that can't be determined from text search alone.
     *
     * @param identifier - Symbol name or fully qualified name
     * @param options - What additional information to include
     *
     * @example
     * // Basic info only
     * const symbol = getSymbol("authenticateUser");
     *
     * @example
     * // Include call graph
     * const symbol = getSymbol("authenticateUser", {
     *   includeCallers: true,
     *   includeCallees: true
     * });
     * console.log("Calls:", symbol.callees);
     * console.log("Called by:", symbol.callers);
     */
    getSymbol(identifier: string, options?: {
        includeCallers?: boolean;
        includeCallees?: boolean;
        includeReferences?: boolean;
        includeImports?: boolean;
        includeImportedBy?: boolean;
        includeContains?: boolean;
    }): SymbolInfo | null;
    /**
     * Find symbols by name pattern.
     *
     * This is more structured than Grep - it searches the symbol table
     * and returns typed results with locations and metadata.
     *
     * @param query - Search pattern (supports SQL LIKE wildcards: %, _)
     * @param filters - Optional filters for kind, file, etc.
     *
     * @example
     * // Find all functions with "auth" in the name
     * const funcs = findSymbols("auth", { kind: ["Function"] });
     *
     * @example
     * // Find all symbols in a specific file
     * const symbols = findSymbols("%", { inFile: "src/auth.ts" });
     */
    findSymbols(query: string, filters?: {
        kind?: string[];
        inFile?: string;
        limit?: number;
    }): SymbolSearchResult[];
    /**
     * Navigate relationships in the code graph.
     *
     * This is the key capability that Read/Grep/Glob don't provide -
     * understanding how code entities relate to each other.
     *
     * @param symbolFqn - Starting symbol FQN
     * @param relationship - Type of relationship to follow
     * @param options - Depth and filters
     *
     * @example
     * // Find all functions that call authenticateUser
     * const callers = queryRelationships("authenticateUser", "called-by");
     *
     * @example
     * // Find transitive dependencies (what this calls, and what those call)
     * const deps = queryRelationships("processPayment", "calls", { depth: 3 });
     */
    queryRelationships(symbolFqn: string, relationship: RelationshipType, options?: {
        depth?: number;
        limit?: number;
    }): RelationshipNode[];
    private getCallers;
    private getCallees;
    private getReferences;
    private getImports;
    private getImportedBy;
    private getContains;
    private getContainer;
    private traverseCalls;
    private traverseImports;
    private traverseImplements;
    private traverseContains;
    /**
     * Strip quotes from database values (handles "Function" vs Function)
     */
    private normalizeValue;
    /**
     * Get file token data for Codebuff integration.
     *
     * Returns symbol scores and caller file paths in the exact format
     * expected by Codebuff's code-map system.
     *
     * @returns Object with tokenScores and tokenCallers
     *
     * @example
     * ```typescript
     * const data = agent.getFileTokenData();
     *
     * // Returns:
     * // {
     * //   tokenScores: {
     * //     "src/auth.ts": { "authenticate": 2.386, "validateToken": 1.609 }
     * //   },
     * //   tokenCallers: {
     * //     "src/auth.ts": { "authenticate": ["src/api/routes.ts", "src/middleware.ts"] }
     * //   }
     * // }
     * ```
     */
    getFileTokenData(): {
        tokenScores: {
            [filePath: string]: {
                [token: string]: number;
            };
        };
        tokenCallers: {
            [filePath: string]: {
                [token: string]: string[];
            };
        };
    };
    /**
     * Close the database connection
     */
    close(): void;
}
//# sourceMappingURL=agent-api.d.ts.map