/**
 * Database adapter that works with both Node.js (better-sqlite3) and Bun (bun:sqlite)
 */

// Detect runtime
const isBun = typeof (globalThis as any).Bun !== "undefined";

// Type definitions for unified interface
export interface DatabaseAdapter {
  prepare(sql: string): StatementAdapter;
  pragma(pragma: string): void;
  exec(sql: string): void;
  close(): void;
}

export interface StatementAdapter {
  get(...params: any[]): any;
  all(...params: any[]): any[];
  run(...params: any[]): any;
}

/**
 * Create a database connection using the appropriate library
 */
export function createDatabase(path: string): DatabaseAdapter {
  if (isBun) {
    // Use Bun's built-in SQLite
    // @ts-ignore - bun:sqlite is not in TypeScript definitions
    const { Database } = require("bun:sqlite");
    const db = new Database(path);

    return {
      prepare(sql: string): StatementAdapter {
        const stmt = db.prepare(sql);
        return {
          get(...params: any[]) {
            return stmt.get(...params);
          },
          all(...params: any[]) {
            return stmt.all(...params);
          },
          run(...params: any[]) {
            return stmt.run(...params);
          },
        };
      },
      pragma(pragma: string) {
        db.run(`PRAGMA ${pragma}`);
      },
      exec(sql: string) {
        db.run(sql);
      },
      close() {
        db.close();
      },
    };
  } else {
    // Use better-sqlite3 for Node.js
    const Database = require("better-sqlite3");
    const db = new Database(path);

    return {
      prepare(sql: string): StatementAdapter {
        const stmt = db.prepare(sql);
        return {
          get(...params: any[]) {
            return stmt.get(...params);
          },
          all(...params: any[]) {
            return stmt.all(...params);
          },
          run(...params: any[]) {
            return stmt.run(...params);
          },
        };
      },
      pragma(pragma: string) {
        db.pragma(pragma);
      },
      exec(sql: string) {
        db.exec(sql);
      },
      close() {
        db.close();
      },
    };
  }
}

/**
 * Get information about the database runtime
 */
export function getDatabaseRuntime(): { runtime: "bun" | "node"; version?: string } {
  if (isBun) {
    return {
      runtime: "bun",
      // @ts-ignore
      version: Bun.version,
    };
  } else {
    return {
      runtime: "node",
      version: process.version,
    };
  }
}
