/**
 * Core CodeGraph class for querying the unified code graph
 */

import Database from "better-sqlite3";
import { join } from "path";
import { existsSync } from "fs";
import { execSync } from "child_process";
import {
  Symbol,
  SymbolKind,
  CallPath,
  DependencyGraph,
  GraphStats,
} from "./models";

/**
 * Main interface for querying the code graph.
 * Integrates multiple analyzers and provides unified access.
 */
export class CodeGraph {
  private db!: Database.Database;
  private dbPath: string;
  private repoPath: string;
  private semantic: boolean;
  private symbolCache: Map<string, Symbol>;
  private callgraphCache: Map<string, any>;

  /**
   * Initialize the code graph for a repository.
   *
   * @param repoPath - Path to the repository root
   * @param dbPath - Path to the graph database (default: .reviewbot/graph.db)
   * @param semantic - Whether to enable semantic analysis with SCIP indexers (default: true)
   */
  constructor(
    repoPath: string,
    dbPath?: string,
    semantic: boolean = true
  ) {
    this.repoPath = repoPath;
    this.dbPath = dbPath || join(repoPath, ".reviewbot", "graph.db");
    this.semantic = semantic;
    this.symbolCache = new Map();
    this.callgraphCache = new Map();

    this.initDatabase();
  }

  /**
   * Initialize database connection
   */
  private initDatabase(): void {
    if (existsSync(this.dbPath)) {
      this.db = new Database(this.dbPath);
      this.db.pragma("journal_mode = WAL");
    } else {
      this.runInitialScan();
      this.db = new Database(this.dbPath);
      this.db.pragma("journal_mode = WAL");
    }
  }

  /**
   * Run consilium scan to build initial graph
   */
  private runInitialScan(): void {
    const analysisType = this.semantic
      ? "semantic + syntactic"
      : "syntactic only";
    console.log(
      `Building code graph for ${this.repoPath} (${analysisType})...`
    );

    const cmd = this.semantic
      ? `cargo run -- --repo ${this.repoPath} scan --semantic`
      : `cargo run -- --repo ${this.repoPath} scan`;

    try {
      execSync(cmd, {
        cwd: join(__dirname, "..", "..", "crates", "core"),
        stdio: "pipe",
      });
    } catch (error: any) {
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
  getSymbol(fqn: string): Symbol | null {
    if (this.symbolCache.has(fqn)) {
      return this.symbolCache.get(fqn)!;
    }

    const stmt = this.db.prepare(`
      SELECT s.*, s.file_path
      FROM symbol s
      
      WHERE s.fqn = ?
    `);

    const row = stmt.get(fqn) as any;
    if (!row) return null;

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
  findSymbols(
    pattern: string,
    kind?: SymbolKind,
    limit: number = 100
  ): Symbol[] {
    let query = `
      SELECT s.*, s.file_path
      FROM symbol s

      WHERE s.name LIKE ?
    `;
    const params: any[] = [`%${pattern}%`];

    if (kind) {
      // Handle both quoted ("Function") and unquoted (Function) kind values in database
      query += " AND (REPLACE(s.kind, '\"', '') = ? OR s.kind = ?)";
      params.push(kind, kind);
    }

    query += ` LIMIT ${limit}`;

    const stmt = this.db.prepare(query);
    const rows = stmt.all(...params) as any[];

    return rows.map((row) => this.rowToSymbol(row));
  }

  /**
   * Get all symbols defined in a file.
   *
   * @param filepath - Path to the file (relative to repo root)
   * @returns List of symbols in the file
   */
  getFileSymbols(filepath: string): Symbol[] {
    const stmt = this.db.prepare(`
      SELECT s.*, s.file_path
      FROM symbol s
      
      WHERE s.file_path = ?
      ORDER BY s.span_start_line
    `);

    const rows = stmt.all(filepath) as any[];
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
  getCallers(symbol: string, maxDepth: number = 1): CallPath[] {
    return this.traverseCalls(symbol, "callers", maxDepth);
  }

  /**
   * Find all functions called by this symbol.
   *
   * @param symbol - FQN of the symbol
   * @param maxDepth - Maximum call chain depth to explore (default: 1)
   * @returns List of call paths from this symbol
   */
  getCallees(symbol: string, maxDepth: number = 1): CallPath[] {
    return this.traverseCalls(symbol, "callees", maxDepth);
  }

  /**
   * Get all dependencies of a symbol.
   *
   * @param symbol - FQN of the symbol
   * @returns DependencyGraph showing all dependencies
   */
  getDependencies(symbol: string): DependencyGraph {
    const dependencies: Record<string, string[]> = {};
    const dependents: Record<string, string[]> = {};

    // Get direct dependencies
    const depStmt = this.db.prepare(`
      SELECT DISTINCT e.dst_symbol
      FROM edge e
      WHERE e.src_symbol = ? AND e.edge_type IN ('calls', 'imports', 'uses')
    `);
    const depRows = depStmt.all(symbol) as any[];
    dependencies[symbol] = depRows.map((row) => row.dst_symbol);

    // Get dependents
    const depsStmt = this.db.prepare(`
      SELECT DISTINCT e.src_symbol
      FROM edge e
      WHERE e.dst_symbol = ? AND e.edge_type IN ('calls', 'imports', 'uses')
    `);
    const depsRows = depsStmt.all(symbol) as any[];
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
  findPath(
    fromSymbol: string,
    toSymbol: string,
    maxDepth: number = 10
  ): Symbol[][] {
    const paths: Symbol[][] = [];
    const visited = new Set<string>();

    const dfs = (
      current: string,
      target: string,
      path: string[],
      depth: number
    ) => {
      if (depth > maxDepth) return;

      if (current === target) {
        // Found a path, convert to symbols
        const symbolPath = path
          .map((fqn) => this.getSymbol(fqn))
          .filter((s): s is Symbol => s !== null);
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
      const rows = stmt.all(current) as any[];

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
  getStatistics(): GraphStats {
    const stats: GraphStats = {
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
    const symbolRows = symbolStmt.all() as any[];
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
    const edgeRows = edgeStmt.all() as any[];
    for (const row of edgeRows) {
      // Strip quotes from edge_type values if present (database stores as "Calls" not Calls)
      const normalizedType = row.edge_type.replace(/^"|"$/g, '');
      stats.edgesByType[normalizedType] = row.count;
    }

    // File statistics
    stats.totalFiles = (
      this.db.prepare("SELECT COUNT(*) as count FROM file").get() as any
    ).count;
    stats.totalSymbols = (
      this.db.prepare("SELECT COUNT(*) as count FROM symbol").get() as any
    ).count;
    stats.totalEdges = (
      this.db.prepare("SELECT COUNT(*) as count FROM edge").get() as any
    ).count;

    return stats;
  }

  // ========== Helper Methods ==========

  /**
   * Convert database row to Symbol object
   */
  private rowToSymbol(row: any): Symbol {
    // Strip quotes from kind values if present (database stores as "Function" not Function)
    const normalizedKind = row.kind.replace(/^"|"$/g, '') as SymbolKind;

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
  private traverseCalls(
    symbol: string,
    direction: "callers" | "callees",
    maxDepth: number
  ): CallPath[] {
    const paths: CallPath[] = [];

    const traverse = (current: string, path: string[], depth: number) => {
      if (depth >= maxDepth) return;

      let stmt: Database.Statement;
      if (direction === "callers") {
        stmt = this.db.prepare(`
          SELECT DISTINCT src_symbol FROM edge
          WHERE dst_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
        `);
      } else {
        stmt = this.db.prepare(`
          SELECT DISTINCT dst_symbol FROM edge
          WHERE src_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
        `);
      }

      const rows = stmt.all(current) as any[];
      const nextSymbols =
        direction === "callers"
          ? rows.map((row) => row.src_symbol)
          : rows.map((row) => row.dst_symbol);

      for (const nextSym of nextSymbols) {
        const newPath = [...path, nextSym];

        // Check for recursion
        const isRecursive = path.includes(nextSym);

        // Convert to symbols and add to results
        const symbolPath = newPath
          .map((fqn) => this.getSymbol(fqn))
          .filter((s): s is Symbol => s !== null);

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
  private findCyclesFrom(
    start: string,
    graph: Record<string, string[]>
  ): string[][] {
    const cycles: string[][] = [];
    const visited = new Set<string>();
    const recStack: string[] = [];

    const dfs = (node: string) => {
      visited.add(node);
      recStack.push(node);

      const neighbors = graph[node] || [];
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          dfs(neighbor);
        } else if (recStack.includes(neighbor)) {
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
  refreshCache(): void {
    this.symbolCache.clear();
    this.callgraphCache.clear();
  }

  /**
   * Close database connection
   */
  close(): void {
    this.db.close();
  }
}
