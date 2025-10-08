"use strict";
/**
 * Core CodeGraph class for querying the unified code graph
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.CodeGraph = void 0;
const path_1 = require("path");
const fs_1 = require("fs");
const child_process_1 = require("child_process");
const db_adapter_1 = require("./db-adapter");
/**
 * Main interface for querying the code graph.
 * Integrates multiple analyzers and provides unified access.
 */
class CodeGraph {
    db;
    dbPath;
    repoPath;
    semantic;
    symbolCache;
    callgraphCache;
    /**
     * Initialize the code graph for a repository.
     *
     * @param repoPath - Path to the repository root
     * @param dbPath - Path to the graph database (default: .reviewbot/graph.db)
     * @param semantic - Whether to enable semantic analysis with SCIP indexers (default: true)
     */
    constructor(repoPath, dbPath, semantic = true) {
        this.repoPath = repoPath;
        this.dbPath = dbPath || (0, path_1.join)(repoPath, ".reviewbot", "graph.db");
        this.semantic = semantic;
        this.symbolCache = new Map();
        this.callgraphCache = new Map();
        this.initDatabase();
    }
    /**
     * Initialize database connection
     */
    initDatabase() {
        if ((0, fs_1.existsSync)(this.dbPath)) {
            this.db = (0, db_adapter_1.createDatabase)(this.dbPath);
            this.db.pragma("journal_mode = WAL");
        }
        else {
            this.runInitialScan();
            this.db = (0, db_adapter_1.createDatabase)(this.dbPath);
            this.db.pragma("journal_mode = WAL");
        }
    }
    /**
     * Run consilium scan to build initial graph
     */
    runInitialScan() {
        const analysisType = this.semantic
            ? "semantic + syntactic"
            : "syntactic only";
        console.log(`Building code graph for ${this.repoPath} (${analysisType})...`);
        const cmd = this.semantic
            ? `cargo run -- --repo ${this.repoPath} scan --semantic`
            : `cargo run -- --repo ${this.repoPath} scan`;
        try {
            (0, child_process_1.execSync)(cmd, {
                cwd: (0, path_1.join)(__dirname, "..", "..", "crates", "core"),
                stdio: "pipe",
            });
        }
        catch (error) {
            throw new Error(`Failed to build code graph: ${error.message}`);
        }
    }
    // ========== Symbol Lookups ==========
    /**
     * Get symbol details by fully qualified name.
     *
     * @param fqn - Fully qualified name (e.g., "MyClass::myMethod")
     * @returns Symbol object or null if not found
     */
    getSymbol(fqn) {
        if (this.symbolCache.has(fqn)) {
            return this.symbolCache.get(fqn);
        }
        const stmt = this.db.prepare(`
      SELECT s.*, s.file_path
      FROM symbol s
      
      WHERE s.fqn = ?
    `);
        const row = stmt.get(fqn);
        if (!row)
            return null;
        const symbol = this.rowToSymbol(row);
        this.symbolCache.set(fqn, symbol);
        return symbol;
    }
    /**
     * Search symbols by pattern and optional type filter.
     *
     * @param pattern - Search pattern (supports SQL wildcards)
     * @param kind - Optional symbol type filter
     * @param limit - Maximum results to return (default: 100)
     * @returns List of matching symbols
     */
    findSymbols(pattern, kind, limit = 100) {
        let query = `
      SELECT s.*, s.file_path
      FROM symbol s

      WHERE s.name LIKE ?
    `;
        const params = [`%${pattern}%`];
        if (kind) {
            // Handle both quoted ("Function") and unquoted (Function) kind values in database
            query += " AND (REPLACE(s.kind, '\"', '') = ? OR s.kind = ?)";
            params.push(kind, kind);
        }
        query += ` LIMIT ${limit}`;
        const stmt = this.db.prepare(query);
        const rows = stmt.all(...params);
        return rows.map((row) => this.rowToSymbol(row));
    }
    /**
     * Get all symbols defined in a file.
     *
     * @param filepath - Path to the file (relative to repo root)
     * @returns List of symbols in the file
     */
    getFileSymbols(filepath) {
        const stmt = this.db.prepare(`
      SELECT s.*, s.file_path
      FROM symbol s
      
      WHERE s.file_path = ?
      ORDER BY s.span_start_line
    `);
        const rows = stmt.all(filepath);
        return rows.map((row) => this.rowToSymbol(row));
    }
    // ========== Relationship Traversal ==========
    /**
     * Find all functions that call this symbol.
     *
     * @param symbol - FQN of the symbol
     * @param maxDepth - Maximum call chain depth to explore (default: 1)
     * @returns List of call paths leading to this symbol
     */
    getCallers(symbol, maxDepth = 1) {
        return this.traverseCalls(symbol, "callers", maxDepth);
    }
    /**
     * Find all functions called by this symbol.
     *
     * @param symbol - FQN of the symbol
     * @param maxDepth - Maximum call chain depth to explore (default: 1)
     * @returns List of call paths from this symbol
     */
    getCallees(symbol, maxDepth = 1) {
        return this.traverseCalls(symbol, "callees", maxDepth);
    }
    /**
     * Get all dependencies of a symbol.
     *
     * @param symbol - FQN of the symbol
     * @returns DependencyGraph showing all dependencies
     */
    getDependencies(symbol) {
        const dependencies = {};
        const dependents = {};
        // Get direct dependencies
        const depStmt = this.db.prepare(`
      SELECT DISTINCT e.dst_symbol
      FROM edge e
      WHERE e.src_symbol = ? AND e.edge_type IN ('calls', 'imports', 'uses')
    `);
        const depRows = depStmt.all(symbol);
        dependencies[symbol] = depRows.map((row) => row.dst_symbol);
        // Get dependents
        const depsStmt = this.db.prepare(`
      SELECT DISTINCT e.src_symbol
      FROM edge e
      WHERE e.dst_symbol = ? AND e.edge_type IN ('calls', 'imports', 'uses')
    `);
        const depsRows = depsStmt.all(symbol);
        dependents[symbol] = depsRows.map((row) => row.src_symbol);
        // Check for cycles
        const cycles = this.findCyclesFrom(symbol, dependencies);
        const rootSymbol = this.getSymbol(symbol);
        if (!rootSymbol) {
            throw new Error(`Symbol not found: ${symbol}`);
        }
        return {
            root: rootSymbol,
            dependencies,
            dependents,
            cycles,
        };
    }
    /**
     * Find execution paths between two symbols.
     *
     * @param fromSymbol - Starting symbol FQN
     * @param toSymbol - Target symbol FQN
     * @param maxDepth - Maximum path length (default: 10)
     * @returns List of possible paths (each path is a list of symbols)
     */
    findPath(fromSymbol, toSymbol, maxDepth = 10) {
        const paths = [];
        const visited = new Set();
        const dfs = (current, target, path, depth) => {
            if (depth > maxDepth)
                return;
            if (current === target) {
                // Found a path, convert to symbols
                const symbolPath = path
                    .map((fqn) => this.getSymbol(fqn))
                    .filter((s) => s !== null);
                paths.push(symbolPath);
                return;
            }
            visited.add(current);
            // Get next symbols
            const stmt = this.db.prepare(`
        SELECT DISTINCT dst_symbol
        FROM edge
        WHERE src_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
      `);
            const rows = stmt.all(current);
            for (const row of rows) {
                const nextSym = row.dst_symbol;
                if (!visited.has(nextSym)) {
                    dfs(nextSym, target, [...path, nextSym], depth + 1);
                }
            }
            visited.delete(current);
        };
        dfs(fromSymbol, toSymbol, [fromSymbol], 0);
        return paths;
    }
    // ========== Graph Statistics ==========
    /**
     * Get overall graph statistics
     */
    getStatistics() {
        const stats = {
            symbolsByKind: {},
            edgesByType: {},
            totalFiles: 0,
            totalSymbols: 0,
            totalEdges: 0,
        };
        // Symbol counts by type
        const symbolStmt = this.db.prepare(`
      SELECT kind, COUNT(*) as count
      FROM symbol
      GROUP BY kind
    `);
        const symbolRows = symbolStmt.all();
        for (const row of symbolRows) {
            // Strip quotes from kind values if present (database stores as "Function" not Function)
            const normalizedKind = row.kind.replace(/^"|"$/g, '');
            stats.symbolsByKind[normalizedKind] = row.count;
        }
        // Edge counts by type
        const edgeStmt = this.db.prepare(`
      SELECT edge_type, COUNT(*) as count
      FROM edge
      GROUP BY edge_type
    `);
        const edgeRows = edgeStmt.all();
        for (const row of edgeRows) {
            // Strip quotes from edge_type values if present (database stores as "Calls" not Calls)
            const normalizedType = row.edge_type.replace(/^"|"$/g, '');
            stats.edgesByType[normalizedType] = row.count;
        }
        // File statistics
        stats.totalFiles = this.db.prepare("SELECT COUNT(*) as count FROM file").get().count;
        stats.totalSymbols = this.db.prepare("SELECT COUNT(*) as count FROM symbol").get().count;
        stats.totalEdges = this.db.prepare("SELECT COUNT(*) as count FROM edge").get().count;
        return stats;
    }
    // ========== Helper Methods ==========
    /**
     * Convert database row to Symbol object
     */
    rowToSymbol(row) {
        // Strip quotes from kind values if present (database stores as "Function" not Function)
        const normalizedKind = row.kind.replace(/^"|"$/g, '');
        return {
            fqn: row.fqn,
            name: row.name,
            kind: normalizedKind,
            location: {
                file: row.file_path,
                line: row.span_start_line,
                column: row.column || undefined,
            },
            signature: row.signature || undefined,
            docstring: row.docstring || undefined,
            analyzer: "consilium",
            confidence: 1.0,
        };
    }
    /**
     * Traverse call graph in either direction
     */
    traverseCalls(symbol, direction, maxDepth) {
        const paths = [];
        const traverse = (current, path, depth) => {
            if (depth >= maxDepth)
                return;
            let stmt;
            if (direction === "callers") {
                stmt = this.db.prepare(`
          SELECT DISTINCT src_symbol FROM edge
          WHERE dst_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
        `);
            }
            else {
                stmt = this.db.prepare(`
          SELECT DISTINCT dst_symbol FROM edge
          WHERE src_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
        `);
            }
            const rows = stmt.all(current);
            const nextSymbols = direction === "callers"
                ? rows.map((row) => row.src_symbol)
                : rows.map((row) => row.dst_symbol);
            for (const nextSym of nextSymbols) {
                const newPath = [...path, nextSym];
                // Check for recursion
                const isRecursive = path.includes(nextSym);
                // Convert to symbols and add to results
                const symbolPath = newPath
                    .map((fqn) => this.getSymbol(fqn))
                    .filter((s) => s !== null);
                paths.push({
                    path: symbolPath,
                    depth: newPath.length,
                    isRecursive,
                });
                // Continue traversing if not recursive
                if (!isRecursive) {
                    traverse(nextSym, newPath, depth + 1);
                }
            }
        };
        traverse(symbol, [symbol], 0);
        return paths;
    }
    /**
     * Find cycles in dependency graph using DFS
     */
    findCyclesFrom(start, graph) {
        const cycles = [];
        const visited = new Set();
        const recStack = [];
        const dfs = (node) => {
            visited.add(node);
            recStack.push(node);
            const neighbors = graph[node] || [];
            for (const neighbor of neighbors) {
                if (!visited.has(neighbor)) {
                    dfs(neighbor);
                }
                else if (recStack.includes(neighbor)) {
                    // Found cycle
                    const cycleStart = recStack.indexOf(neighbor);
                    cycles.push(recStack.slice(cycleStart));
                }
            }
            recStack.pop();
        };
        dfs(start);
        return cycles;
    }
    /**
     * Clear all caches
     */
    refreshCache() {
        this.symbolCache.clear();
        this.callgraphCache.clear();
    }
    /**
     * Close database connection
     */
    close() {
        this.db.close();
    }
}
exports.CodeGraph = CodeGraph;
//# sourceMappingURL=code-graph.js.map