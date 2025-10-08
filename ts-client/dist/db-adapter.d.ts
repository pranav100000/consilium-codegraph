/**
 * Database adapter that works with both Node.js (better-sqlite3) and Bun (bun:sqlite)
 */
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
export declare function createDatabase(path: string): DatabaseAdapter;
/**
 * Get information about the database runtime
 */
export declare function getDatabaseRuntime(): {
    runtime: "bun" | "node";
    version?: string;
};
//# sourceMappingURL=db-adapter.d.ts.map