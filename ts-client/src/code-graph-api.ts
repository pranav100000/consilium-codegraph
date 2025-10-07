/**
 * Simplified Code Graph API for agents - focused on database queries
 */

import { join } from "path";
import { existsSync } from "fs";
import { createDatabase, DatabaseAdapter } from "./db-adapter";
import { Symbol, Edge, EdgeType, GraphStats } from "./models";

/**
 * Simple API for querying the code graph database.
 * Designed to run on the same server as agents - no auth needed.
 */
export class CodeGraphAPI {
  private db: DatabaseAdapter;
  private dbPath: string;

  /**
   * Initialize the API for a repository.
   *
   * @param repoPath - Path to the repository root
   * @param dbPath - Path to the graph database (default: .reviewbot/graph.db)
   * @param semantic - Whether to recommend semantic analysis (default: true)
   */
  constructor(
    repoPath: string,
    dbPath?: string,
    semantic: boolean = true
  ) {
    this.dbPath = dbPath || join(repoPath, ".reviewbot", "graph.db");

    if (!existsSync(this.dbPath)) {
      const scanCmd = semantic ? "reviewbot scan --semantic" : "reviewbot scan";
      throw new Error(
        `Database not found at ${this.dbPath}. Run '${scanCmd}' first.`
      );
    }

    this.db = createDatabase(this.dbPath);
    // Enable WAL mode for better concurrency
    this.db.pragma("journal_mode = WAL");
    this.db.pragma("foreign_keys = ON");
  }

  // ========== Core Queries ==========

  /**
   * Get a symbol by its fully qualified name.
   */
  getSymbol(fqn: string): Symbol | null {
    const stmt = this.db.prepare(`
      SELECT s.fqn, s.name, s.kind, s.file_path, s.span_start_line, s.signature
      FROM symbol s
      
      WHERE s.fqn = ?
    `);

    const row = stmt.get(fqn) as any;
    if (!row) return null;

    // Strip quotes from kind values if present (database stores as "Function" not Function)
    const normalizedKind = row.kind.replace(/^"|"$/g, '');

    return {
      fqn: row.fqn,
      name: row.name,
      kind: normalizedKind,
      location: {
        file: row.file_path,
        line: row.span_start_line,
      },
      signature: row.signature || undefined,
    };
  }

  /**
   * Search for symbols by name pattern.
   */
  findSymbols(pattern: string, kind?: string): Symbol[] {
    let query = `
      SELECT s.fqn, s.name, s.kind, s.file_path, s.span_start_line, s.signature
      FROM symbol s

      WHERE s.name LIKE ?
    `;
    const params: any[] = [`%${pattern}%`];

    if (kind) {
      // Handle both quoted ("Function") and unquoted (Function) kind values in database
      query += " AND (REPLACE(s.kind, '\"', '') = ? OR s.kind = ?)";
      params.push(kind, kind);
    }

    const stmt = this.db.prepare(query);
    const rows = stmt.all(...params) as any[];

    return rows.map((row) => {
      // Strip quotes from kind values if present (database stores as "Function" not Function)
      const normalizedKind = row.kind.replace(/^"|"$/g, '');

      return {
        fqn: row.fqn,
        name: row.name,
        kind: normalizedKind,
        location: {
          file: row.file_path,
          line: row.span_start_line,
        },
        signature: row.signature || undefined,
      };
    });
  }

  /**
   * Get all symbols in a file.
   */
  getFileSymbols(filePath: string): Symbol[] {
    const stmt = this.db.prepare(`
      SELECT s.fqn, s.name, s.kind, s.file_path, s.span_start_line, s.signature
      FROM symbol s
      
      WHERE s.file_path = ?
      ORDER BY s.span_start_line
    `);

    const rows = stmt.all(filePath) as any[];
    return rows.map((row) => {
      // Strip quotes from kind values if present (database stores as "Function" not Function)
      const normalizedKind = row.kind.replace(/^"|"$/g, '');

      return {
        fqn: row.fqn,
        name: row.name,
        kind: normalizedKind,
        location: {
          file: row.file_path,
          line: row.span_start_line,
        },
        signature: row.signature || undefined,
      };
    });
  }

  // ========== Relationship Queries ==========

  /**
   * Get all functions that call this symbol.
   */
  getCallers(symbol: string): string[] {
    const stmt = this.db.prepare(`
      SELECT DISTINCT src_symbol
      FROM edge
      WHERE dst_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
    `);

    const rows = stmt.all(symbol) as any[];
    return rows.map((row) => row.src_symbol);
  }

  /**
   * Get all functions called by this symbol.
   */
  getCallees(symbol: string): string[] {
    const stmt = this.db.prepare(`
      SELECT DISTINCT dst_symbol
      FROM edge
      WHERE src_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
    `);

    const rows = stmt.all(symbol) as any[];
    return rows.map((row) => row.dst_symbol);
  }

  /**
   * Get edges with optional filters.
   */
  getEdges(
    source?: string,
    target?: string,
    edgeType?: string
  ): Edge[] {
    let query = "SELECT src_symbol, dst_symbol, edge_type FROM edge WHERE 1=1";
    const params: any[] = [];

    if (source) {
      query += " AND src_symbol = ?";
      params.push(source);
    }

    if (target) {
      query += " AND dst_symbol = ?";
      params.push(target);
    }

    if (edgeType) {
      // Handle both quoted ("Calls") and unquoted (Calls) edge_type values in database
      query += " AND (REPLACE(edge_type, '\"', '') = ? OR edge_type = ?)";
      params.push(edgeType, edgeType);
    }

    const stmt = this.db.prepare(query);
    const rows = stmt.all(...params) as any[];

    return rows.map((row) => {
      // Strip quotes from edge_type values if present (database stores as "Calls" not Calls)
      const normalizedType = row.edge_type.replace(/^"|"$/g, '') as EdgeType;

      return {
        source: row.src_symbol,
        target: row.dst_symbol,
        edgeType: normalizedType,
      };
    });
  }

  // ========== Analysis Queries ==========

  /**
   * Find all paths between two symbols.
   */
  findPaths(start: string, end: string, maxDepth: number = 5): string[][] {
    const paths: string[][] = [];
    const visited = new Set<string>();

    const dfs = (
      current: string,
      target: string,
      path: string[],
      depth: number
    ) => {
      if (depth > maxDepth) return;

      if (current === target) {
        paths.push([...path]);
        return;
      }

      visited.add(current);

      const callees = this.getCallees(current);
      for (const nextNode of callees) {
        if (!visited.has(nextNode)) {
          path.push(nextNode);
          dfs(nextNode, target, path, depth + 1);
          path.pop();
        }
      }

      visited.delete(current);
    };

    dfs(start, end, [start], 0);
    return paths;
  }

  /**
   * Get all dependencies of a symbol.
   */
  getDependencies(symbol: string): Record<string, string[]> {
    const result: Record<string, string[]> = {
      imports: [],
      calls: [],
      uses: [],
    };

    const stmt = this.db.prepare(`
      SELECT DISTINCT dst_symbol, edge_type
      FROM edge
      WHERE src_symbol = ?
    `);

    const rows = stmt.all(symbol) as any[];
    for (const row of rows) {
      const edgeType = row.edge_type;
      if (edgeType in result) {
        result[edgeType].push(row.dst_symbol);
      } else if (edgeType === "imports") {
        result.imports.push(row.dst_symbol);
      } else if (edgeType === "calls") {
        result.calls.push(row.dst_symbol);
      } else {
        if (!result[edgeType]) {
          result[edgeType] = [];
        }
        result[edgeType].push(row.dst_symbol);
      }
    }

    return result;
  }

  /**
   * Get all symbols that would be affected if this symbol changes.
   */
  getImpactRadius(symbol: string, maxDepth: number = 3): Set<string> {
    const impacted = new Set<string>();
    const toProcess: Array<[string, number]> = [[symbol, 0]];

    while (toProcess.length > 0) {
      const [current, depth] = toProcess.shift()!;

      if (depth >= maxDepth) continue;

      const callers = this.getCallers(current);
      for (const caller of callers) {
        if (!impacted.has(caller)) {
          impacted.add(caller);
          toProcess.push([caller, depth + 1]);
        }
      }
    }

    return impacted;
  }

  // ========== Statistics ==========

  /**
   * Get overall statistics about the code graph.
   */
  getStats(): GraphStats {
    const stats: GraphStats = {
      symbolsByKind: {},
      edgesByType: {},
      totalFiles: 0,
      totalSymbols: 0,
      totalEdges: 0,
    };

    // Count symbols by type
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

    // Count edges by type
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

    // Total counts
    stats.totalFiles = (this.db.prepare("SELECT COUNT(*) as count FROM file").get() as any).count;
    stats.totalSymbols = (this.db.prepare("SELECT COUNT(*) as count FROM symbol").get() as any).count;
    stats.totalEdges = (this.db.prepare("SELECT COUNT(*) as count FROM edge").get() as any).count;

    return stats;
  }

  /**
   * Find all cycles in the call graph.
   */
  findCycles(): string[][] {
    const cycles: string[][] = [];
    const visited = new Set<string>();
    const recStack: string[] = [];

    // Get all symbols
    const stmt = this.db.prepare("SELECT DISTINCT fqn FROM symbol");
    const allSymbols = (stmt.all() as any[]).map((row) => row.fqn);

    const dfs = (node: string) => {
      visited.add(node);
      recStack.push(node);

      const callees = this.getCallees(node);
      for (const callee of callees) {
        if (!visited.has(callee)) {
          dfs(callee);
        } else if (recStack.includes(callee)) {
          // Found a cycle
          const cycleStart = recStack.indexOf(callee);
          const cycle = [...recStack.slice(cycleStart), callee];

          // Check if this cycle is already recorded
          const cycleStr = cycle.join(",");
          if (!cycles.some((c) => c.join(",") === cycleStr)) {
            cycles.push(cycle);
          }
        }
      }

      recStack.pop();
    };

    for (const symbol of allSymbols) {
      if (!visited.has(symbol)) {
        dfs(symbol);
      }
    }

    return cycles;
  }

  /**
   * Begin an explicit transaction.
   */
  beginTransaction(): void {
    this.db.exec("BEGIN TRANSACTION");
  }

  /**
   * Commit the current transaction.
   */
  commit(): void {
    this.db.exec("COMMIT");
  }

  /**
   * Rollback the current transaction.
   */
  rollback(): void {
    this.db.exec("ROLLBACK");
  }

  /**
   * Close the database connection.
   */
  close(): void {
    this.db.close();
  }
}

// ========== Convenience Functions for Agents ==========

/**
 * Quick analysis of a codebase for agents.
 *
 * @returns Dictionary with key metrics and insights
 */
export function analyzeCodebase(repoPath: string): {
  stats: GraphStats;
  cycles: string[][];
  entryPoints: string[];
  complexFunctions: Array<{ function: string; calleesCount: number }>;
} {
  const api = new CodeGraphAPI(repoPath);

  const stats = api.getStats();
  const cycles = api.findCycles();

  // Find entry points (functions with few/no callers)
  const entryPoints: string[] = [];
  const symbols = api.findSymbols("", "Function").slice(0, 100); // Sample
  for (const symbol of symbols) {
    const callers = api.getCallers(symbol.fqn);
    if (callers.length <= 1) {
      entryPoints.push(symbol.fqn);
    }
  }

  // Find complex functions (many callees)
  const complexFunctions: Array<{ function: string; calleesCount: number }> = [];
  for (const symbol of symbols) {
    const callees = api.getCallees(symbol.fqn);
    if (callees.length > 10) {
      complexFunctions.push({
        function: symbol.fqn,
        calleesCount: callees.length,
      });
    }
  }

  api.close();

  return {
    stats,
    cycles: cycles.slice(0, 10), // Limit to first 10
    entryPoints: entryPoints.slice(0, 20), // Limit to first 20
    complexFunctions: complexFunctions
      .sort((a, b) => b.calleesCount - a.calleesCount)
      .slice(0, 10),
  };
}

/**
 * Find all code related to a symbol.
 *
 * @returns Dictionary with callers, callees, and dependencies
 */
export function findRelatedCode(
  repoPath: string,
  symbol: string
): {
  symbol: string;
  callers: string[];
  callees: string[];
  dependencies: Record<string, string[]>;
  impact: string[];
} {
  const api = new CodeGraphAPI(repoPath);

  const result = {
    symbol,
    callers: api.getCallers(symbol),
    callees: api.getCallees(symbol),
    dependencies: api.getDependencies(symbol),
    impact: Array.from(api.getImpactRadius(symbol)),
  };

  api.close();

  return result;
}
