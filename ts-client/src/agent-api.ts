/**
 * Agent-focused API for code navigation
 *
 * Designed to complement existing agentic tools (Read, Grep, Glob)
 * by adding the graph/relationship layer.
 */

import { join } from "path";
import { existsSync } from "fs";
import { createDatabase, DatabaseAdapter } from "./db-adapter";

// ========== Types ==========

export interface Location {
  file: string;
  line: number;
  column?: number;
}

export interface SymbolInfo {
  // Identity
  fqn: string;
  name: string;
  kind: string;

  // Location
  location: Location;

  // Type information
  signature?: string;

  // Context
  containedIn?: string;      // Parent class/module FQN
  contains?: string[];       // Child symbols (methods, fields, etc.)

  // Relationships (optional, based on what was requested)
  callers?: string[];        // FQNs of symbols that call this
  callees?: string[];        // FQNs of symbols this calls
  references?: Location[];   // All locations where this is referenced
  imports?: string[];        // Symbols this imports
  importedBy?: string[];     // Symbols that import this
}

export interface SymbolSearchResult {
  fqn: string;
  name: string;
  kind: string;
  location: Location;
  signature?: string;
}

export interface RelationshipNode {
  symbol: string;            // FQN
  kind: string;              // Symbol kind
  location: Location;
  depth: number;             // Distance from starting symbol
  path?: string[];           // Path from start to this symbol (for deep queries)
}

export type RelationshipType =
  | "calls"          // Functions/methods this calls
  | "called-by"      // Functions/methods that call this
  | "imports"        // Symbols this imports
  | "imported-by"    // Symbols that import this
  | "implements"     // Interfaces this implements
  | "implemented-by" // Classes that implement this interface
  | "contains"       // Symbols contained in this (methods in class, etc.)
  | "contained-in";  // Symbol that contains this

// ========== Agent API ==========

export class AgentCodeGraph {
  private db: DatabaseAdapter;
  private dbPath: string;

  /**
   * Initialize the agent API for a repository.
   *
   * @param repoPath - Path to the repository root
   * @param dbPath - Path to the graph database (default: .reviewbot/graph.db)
   */
  constructor(repoPath: string, dbPath?: string) {
    this.dbPath = dbPath || join(repoPath, ".reviewbot", "graph.db");

    if (!existsSync(this.dbPath)) {
      throw new Error(
        `Database not found at ${this.dbPath}. Run 'cargo run -- scan' first.`
      );
    }

    this.db = createDatabase(this.dbPath);
    this.db.pragma("journal_mode = WAL");
  }

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
  getSymbol(
    identifier: string,
    options?: {
      includeCallers?: boolean;
      includeCallees?: boolean;
      includeReferences?: boolean;
      includeImports?: boolean;
      includeImportedBy?: boolean;
      includeContains?: boolean;
    }
  ): SymbolInfo | null {
    // First, try to find the symbol by FQN or name
    const stmt = this.db.prepare(`
      SELECT fqn, name, kind, file_path, span_start_line, span_start_col, signature
      FROM symbol
      WHERE fqn = ? OR name = ?
      LIMIT 1
    `);

    const row = stmt.get(identifier, identifier) as any;
    if (!row) return null;

    // Build base symbol info
    const symbolInfo: SymbolInfo = {
      fqn: row.fqn,
      name: row.name,
      kind: this.normalizeValue(row.kind),
      location: {
        file: row.file_path,
        line: row.span_start_line,
        column: row.span_start_col,
      },
      signature: row.signature || undefined,
    };

    // Add relationships based on options
    if (options?.includeCallers) {
      symbolInfo.callers = this.getCallers(row.fqn);
    }

    if (options?.includeCallees) {
      symbolInfo.callees = this.getCallees(row.fqn);
    }

    if (options?.includeReferences) {
      symbolInfo.references = this.getReferences(row.fqn);
    }

    if (options?.includeImports) {
      symbolInfo.imports = this.getImports(row.fqn);
    }

    if (options?.includeImportedBy) {
      symbolInfo.importedBy = this.getImportedBy(row.fqn);
    }

    if (options?.includeContains) {
      symbolInfo.contains = this.getContains(row.fqn);
    }

    // Get parent container if it exists
    symbolInfo.containedIn = this.getContainer(row.fqn);

    return symbolInfo;
  }

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
  findSymbols(
    query: string,
    filters?: {
      kind?: string[];
      inFile?: string;
      limit?: number;
    }
  ): SymbolSearchResult[] {
    let sql = `
      SELECT fqn, name, kind, file_path, span_start_line, signature
      FROM symbol
      WHERE name LIKE ?
    `;
    const params: any[] = [`%${query}%`];

    if (filters?.kind && filters.kind.length > 0) {
      const kindPlaceholders = filters.kind.map(() => "?").join(",");
      sql += ` AND (REPLACE(kind, '"', '') IN (${kindPlaceholders}) OR kind IN (${kindPlaceholders}))`;
      params.push(...filters.kind, ...filters.kind);
    }

    if (filters?.inFile) {
      sql += ` AND file_path = ?`;
      params.push(filters.inFile);
    }

    sql += ` LIMIT ${filters?.limit || 100}`;

    const stmt = this.db.prepare(sql);
    const rows = stmt.all(...params) as any[];

    return rows.map((row) => ({
      fqn: row.fqn,
      name: row.name,
      kind: this.normalizeValue(row.kind),
      location: {
        file: row.file_path,
        line: row.span_start_line,
      },
      signature: row.signature || undefined,
    }));
  }

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
  queryRelationships(
    symbolFqn: string,
    relationship: RelationshipType,
    options?: {
      depth?: number;
      limit?: number;
    }
  ): RelationshipNode[] {
    const depth = options?.depth || 1;
    const limit = options?.limit || 100;

    switch (relationship) {
      case "calls":
        return this.traverseCalls(symbolFqn, "outgoing", depth, limit);
      case "called-by":
        return this.traverseCalls(symbolFqn, "incoming", depth, limit);
      case "imports":
        return this.traverseImports(symbolFqn, "outgoing", depth, limit);
      case "imported-by":
        return this.traverseImports(symbolFqn, "incoming", depth, limit);
      case "implements":
        return this.traverseImplements(symbolFqn, "outgoing", limit);
      case "implemented-by":
        return this.traverseImplements(symbolFqn, "incoming", limit);
      case "contains":
        return this.traverseContains(symbolFqn, "outgoing", limit);
      case "contained-in":
        return this.traverseContains(symbolFqn, "incoming", limit);
      default:
        throw new Error(`Unknown relationship type: ${relationship}`);
    }
  }

  // ========== Helper Methods ==========

  private getCallers(fqn: string): string[] {
    const stmt = this.db.prepare(`
      SELECT DISTINCT src_symbol
      FROM edge
      WHERE dst_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
    `);
    const rows = stmt.all(fqn) as any[];
    return rows.map((row) => row.src_symbol);
  }

  private getCallees(fqn: string): string[] {
    const stmt = this.db.prepare(`
      SELECT DISTINCT dst_symbol
      FROM edge
      WHERE src_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
    `);
    const rows = stmt.all(fqn) as any[];
    return rows.map((row) => row.dst_symbol);
  }

  private getReferences(fqn: string): Location[] {
    const stmt = this.db.prepare(`
      SELECT file_path, span_start_line, span_start_col
      FROM occurrence
      WHERE symbol_id = (SELECT symbol_id FROM symbol WHERE fqn = ? LIMIT 1)
    `);
    const rows = stmt.all(fqn) as any[];
    return rows.map((row) => ({
      file: row.file_path,
      line: row.span_start_line,
      column: row.span_start_col,
    }));
  }

  private getImports(fqn: string): string[] {
    const stmt = this.db.prepare(`
      SELECT DISTINCT dst_symbol
      FROM edge
      WHERE src_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Imports' OR edge_type = 'Imports')
    `);
    const rows = stmt.all(fqn) as any[];
    return rows.map((row) => row.dst_symbol);
  }

  private getImportedBy(fqn: string): string[] {
    const stmt = this.db.prepare(`
      SELECT DISTINCT src_symbol
      FROM edge
      WHERE dst_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Imports' OR edge_type = 'Imports')
    `);
    const rows = stmt.all(fqn) as any[];
    return rows.map((row) => row.src_symbol);
  }

  private getContains(fqn: string): string[] {
    const stmt = this.db.prepare(`
      SELECT DISTINCT dst_symbol
      FROM edge
      WHERE src_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Contains' OR edge_type = 'Contains')
    `);
    const rows = stmt.all(fqn) as any[];
    return rows.map((row) => row.dst_symbol);
  }

  private getContainer(fqn: string): string | undefined {
    const stmt = this.db.prepare(`
      SELECT DISTINCT src_symbol
      FROM edge
      WHERE dst_symbol = ? AND (REPLACE(edge_type, '"', '') = 'Contains' OR edge_type = 'Contains')
      LIMIT 1
    `);
    const row = stmt.get(fqn) as any;
    return row?.src_symbol;
  }

  private traverseCalls(
    startFqn: string,
    direction: "incoming" | "outgoing",
    maxDepth: number,
    limit: number
  ): RelationshipNode[] {
    const results: RelationshipNode[] = [];
    const visited = new Set<string>();
    const queue: Array<{ fqn: string; depth: number; path: string[] }> = [
      { fqn: startFqn, depth: 0, path: [startFqn] },
    ];

    while (queue.length > 0 && results.length < limit) {
      const { fqn, depth, path } = queue.shift()!;

      if (depth >= maxDepth || visited.has(fqn)) continue;
      visited.add(fqn);

      // Get next symbols based on direction
      const edgeColumn = direction === "incoming" ? "src_symbol" : "dst_symbol";
      const whereColumn = direction === "incoming" ? "dst_symbol" : "src_symbol";

      const stmt = this.db.prepare(`
        SELECT DISTINCT e.${edgeColumn} as next_symbol, s.kind, s.file_path, s.span_start_line
        FROM edge e
        JOIN symbol s ON e.${edgeColumn} = s.fqn
        WHERE e.${whereColumn} = ?
          AND (REPLACE(e.edge_type, '"', '') = 'Calls' OR e.edge_type = 'Calls')
      `);

      const rows = stmt.all(fqn) as any[];

      for (const row of rows) {
        if (results.length >= limit) break;

        const nextFqn = row.next_symbol;
        if (!visited.has(nextFqn)) {
          results.push({
            symbol: nextFqn,
            kind: this.normalizeValue(row.kind),
            location: {
              file: row.file_path,
              line: row.span_start_line,
            },
            depth: depth + 1,
            path: depth > 0 ? [...path, nextFqn] : undefined,
          });

          queue.push({
            fqn: nextFqn,
            depth: depth + 1,
            path: [...path, nextFqn],
          });
        }
      }
    }

    return results;
  }

  private traverseImports(
    startFqn: string,
    direction: "incoming" | "outgoing",
    maxDepth: number,
    limit: number
  ): RelationshipNode[] {
    const results: RelationshipNode[] = [];
    const visited = new Set<string>();
    const queue: Array<{ fqn: string; depth: number }> = [
      { fqn: startFqn, depth: 0 },
    ];

    while (queue.length > 0 && results.length < limit) {
      const { fqn, depth } = queue.shift()!;

      if (depth >= maxDepth || visited.has(fqn)) continue;
      visited.add(fqn);

      const edgeColumn = direction === "incoming" ? "src_symbol" : "dst_symbol";
      const whereColumn = direction === "incoming" ? "dst_symbol" : "src_symbol";

      const stmt = this.db.prepare(`
        SELECT DISTINCT e.${edgeColumn} as next_symbol, s.kind, s.file_path, s.span_start_line
        FROM edge e
        JOIN symbol s ON e.${edgeColumn} = s.fqn
        WHERE e.${whereColumn} = ?
          AND (REPLACE(e.edge_type, '"', '') = 'Imports' OR e.edge_type = 'Imports')
      `);

      const rows = stmt.all(fqn) as any[];

      for (const row of rows) {
        if (results.length >= limit) break;

        const nextFqn = row.next_symbol;
        if (!visited.has(nextFqn)) {
          results.push({
            symbol: nextFqn,
            kind: this.normalizeValue(row.kind),
            location: {
              file: row.file_path,
              line: row.span_start_line,
            },
            depth: depth + 1,
          });

          queue.push({ fqn: nextFqn, depth: depth + 1 });
        }
      }
    }

    return results;
  }

  private traverseImplements(
    startFqn: string,
    direction: "incoming" | "outgoing",
    limit: number
  ): RelationshipNode[] {
    const edgeColumn = direction === "incoming" ? "src_symbol" : "dst_symbol";
    const whereColumn = direction === "incoming" ? "dst_symbol" : "src_symbol";

    const stmt = this.db.prepare(`
      SELECT DISTINCT e.${edgeColumn} as symbol, s.kind, s.file_path, s.span_start_line
      FROM edge e
      JOIN symbol s ON e.${edgeColumn} = s.fqn
      WHERE e.${whereColumn} = ?
        AND (REPLACE(e.edge_type, '"', '') = 'Implements' OR e.edge_type = 'Implements')
      LIMIT ${limit}
    `);

    const rows = stmt.all(startFqn) as any[];

    return rows.map((row) => ({
      symbol: row.symbol,
      kind: this.normalizeValue(row.kind),
      location: {
        file: row.file_path,
        line: row.span_start_line,
      },
      depth: 1,
    }));
  }

  private traverseContains(
    startFqn: string,
    direction: "incoming" | "outgoing",
    limit: number
  ): RelationshipNode[] {
    const edgeColumn = direction === "incoming" ? "src_symbol" : "dst_symbol";
    const whereColumn = direction === "incoming" ? "dst_symbol" : "src_symbol";

    const stmt = this.db.prepare(`
      SELECT DISTINCT e.${edgeColumn} as symbol, s.kind, s.file_path, s.span_start_line
      FROM edge e
      JOIN symbol s ON e.${edgeColumn} = s.fqn
      WHERE e.${whereColumn} = ?
        AND (REPLACE(e.edge_type, '"', '') = 'Contains' OR e.edge_type = 'Contains')
      LIMIT ${limit}
    `);

    const rows = stmt.all(startFqn) as any[];

    return rows.map((row) => ({
      symbol: row.symbol,
      kind: this.normalizeValue(row.kind),
      location: {
        file: row.file_path,
        line: row.span_start_line,
      },
      depth: 1,
    }));
  }

  /**
   * Strip quotes from database values (handles "Function" vs Function)
   */
  private normalizeValue(value: string): string {
    return value.replace(/^"|"$/g, "");
  }

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
    tokenScores: { [filePath: string]: { [token: string]: number } };
    tokenCallers: { [filePath: string]: { [token: string]: string[] } };
  } {
    const tokenScores: { [filePath: string]: { [token: string]: number } } = {};
    const tokenCallers: { [filePath: string]: { [token: string]: string[] } } = {};

    // Symbols to ignore (common boilerplate)
    const ignoredNames = new Set(["__init__", "__post_init__", "__call__", "constructor"]);

    // Get all files with symbols
    const filesStmt = this.db.prepare(`
      SELECT DISTINCT file_path FROM symbol
      ORDER BY file_path
    `);
    const files = filesStmt.all() as { file_path: string }[];

    // Build FQN -> file path mapping for fast lookups
    const fqnToFile = new Map<string, string>();
    const allSymbolsStmt = this.db.prepare(`
      SELECT fqn, file_path FROM symbol
    `);
    const allSymbols = allSymbolsStmt.all() as { fqn: string; file_path: string }[];
    allSymbols.forEach(({ fqn, file_path }) => {
      fqnToFile.set(fqn, file_path);
    });

    // Process each file
    for (const { file_path } of files) {
      // Get symbols in this file
      const symbolsStmt = this.db.prepare(`
        SELECT fqn, name, kind
        FROM symbol
        WHERE file_path = ?
      `);
      const symbols = symbolsStmt.all(file_path) as { fqn: string; name: string; kind: string }[];

      for (const symbol of symbols) {
        // Skip ignored symbols
        if (ignoredNames.has(symbol.name)) {
          continue;
        }

        // Get callers for this symbol
        const callersStmt = this.db.prepare(`
          SELECT DISTINCT src_symbol
          FROM edge
          WHERE dst_symbol = ?
            AND (REPLACE(edge_type, '"', '') = 'Calls' OR edge_type = 'Calls')
        `);
        const callerRows = callersStmt.all(symbol.fqn) as { src_symbol: string }[];

        // Calculate score based on number of callers
        // Formula: 1.0 + ln(1 + numCallers)
        const numCallers = callerRows.length;
        const score = 1.0 + Math.log(1 + numCallers);
        const roundedScore = Math.round(score * 1000) / 1000; // 3 decimal places

        // Initialize file entries if needed
        if (!tokenScores[file_path]) {
          tokenScores[file_path] = {};
        }
        if (!tokenCallers[file_path]) {
          tokenCallers[file_path] = {};
        }

        // Store score
        tokenScores[file_path][symbol.name] = roundedScore;

        // Convert caller FQNs to file paths
        const callerFiles = new Set<string>();
        for (const { src_symbol } of callerRows) {
          const callerFile = fqnToFile.get(src_symbol);
          if (callerFile) {
            callerFiles.add(callerFile);
          }

          // Limit to 25 callers max (Codebuff's MAX_CALLERS)
          if (callerFiles.size >= 25) {
            break;
          }
        }

        // Store caller files (deduplicated, limited to 25)
        tokenCallers[file_path][symbol.name] = Array.from(callerFiles);
      }
    }

    return { tokenScores, tokenCallers };
  }

  /**
   * Close the database connection
   */
  close(): void {
    this.db.close();
  }
}
