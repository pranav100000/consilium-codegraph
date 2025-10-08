"use strict";
/**
 * Database adapter that works with both Node.js (better-sqlite3) and Bun (bun:sqlite)
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.createDatabase = createDatabase;
exports.getDatabaseRuntime = getDatabaseRuntime;
// Detect runtime
const isBun = typeof globalThis.Bun !== "undefined";
/**
 * Create a database connection using the appropriate library
 */
function createDatabase(path) {
    if (isBun) {
        // Use Bun's built-in SQLite
        // @ts-ignore - bun:sqlite is not in TypeScript definitions
        const { Database } = require("bun:sqlite");
        const db = new Database(path);
        return {
            prepare(sql) {
                const stmt = db.prepare(sql);
                return {
                    get(...params) {
                        return stmt.get(...params);
                    },
                    all(...params) {
                        return stmt.all(...params);
                    },
                    run(...params) {
                        return stmt.run(...params);
                    },
                };
            },
            pragma(pragma) {
                db.run(`PRAGMA ${pragma}`);
            },
            exec(sql) {
                db.run(sql);
            },
            close() {
                db.close();
            },
        };
    }
    else {
        // Use better-sqlite3 for Node.js
        const Database = require("better-sqlite3");
        const db = new Database(path);
        return {
            prepare(sql) {
                const stmt = db.prepare(sql);
                return {
                    get(...params) {
                        return stmt.get(...params);
                    },
                    all(...params) {
                        return stmt.all(...params);
                    },
                    run(...params) {
                        return stmt.run(...params);
                    },
                };
            },
            pragma(pragma) {
                db.pragma(pragma);
            },
            exec(sql) {
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
function getDatabaseRuntime() {
    if (isBun) {
        return {
            runtime: "bun",
            // @ts-ignore
            version: Bun.version,
        };
    }
    else {
        return {
            runtime: "node",
            version: process.version,
        };
    }
}
//# sourceMappingURL=db-adapter.js.map