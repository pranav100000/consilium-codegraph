use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, Span, SymbolIR, SymbolKind};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

mod graph;
pub use graph::{CodeGraph, GraphStats};

pub struct GraphStore {
    db_path: PathBuf,
    conn: Connection,
}

impl GraphStore {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let db_dir = repo_path.join(".reviewbot");
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("graph.db");
        
        let conn = Connection::open(&db_path)?;
        
        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        
        let store = Self { db_path, conn };
        store.init_schema()?;
        Ok(store)
    }
    
    fn get_connection(&self) -> Result<&Connection> {
        Ok(&self.conn)
    }
    
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE IF NOT EXISTS commit_snapshot (
                id INTEGER PRIMARY KEY,
                commit_sha TEXT UNIQUE NOT NULL,
                timestamp INTEGER NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS file (
                id INTEGER PRIMARY KEY,
                commit_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                size_bytes INTEGER,
                FOREIGN KEY (commit_id) REFERENCES commit_snapshot(id),
                UNIQUE(commit_id, path)
            );
            
            CREATE TABLE IF NOT EXISTS symbol (
                id INTEGER PRIMARY KEY,
                commit_id INTEGER NOT NULL,
                symbol_id TEXT NOT NULL,
                lang TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                fqn TEXT NOT NULL,
                signature TEXT,
                file_path TEXT NOT NULL,
                span_start_line INTEGER NOT NULL,
                span_start_col INTEGER NOT NULL,
                span_end_line INTEGER NOT NULL,
                span_end_col INTEGER NOT NULL,
                visibility TEXT,
                doc TEXT,
                sig_hash TEXT NOT NULL,
                FOREIGN KEY (commit_id) REFERENCES commit_snapshot(id),
                UNIQUE(commit_id, symbol_id)
            );
            
            CREATE TABLE IF NOT EXISTS edge (
                id INTEGER PRIMARY KEY,
                commit_id INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                src_symbol TEXT,
                dst_symbol TEXT,
                file_src TEXT,
                file_dst TEXT,
                resolution TEXT NOT NULL,
                FOREIGN KEY (commit_id) REFERENCES commit_snapshot(id)
            );
            
            CREATE TABLE IF NOT EXISTS occurrence (
                id INTEGER PRIMARY KEY,
                commit_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                symbol_id TEXT,
                role TEXT NOT NULL,
                span_start_line INTEGER NOT NULL,
                span_start_col INTEGER NOT NULL,
                span_end_line INTEGER NOT NULL,
                span_end_col INTEGER NOT NULL,
                token TEXT NOT NULL,
                FOREIGN KEY (commit_id) REFERENCES commit_snapshot(id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_symbol_fqn ON symbol(fqn);
            CREATE INDEX IF NOT EXISTS idx_symbol_commit_fqn ON symbol(commit_id, fqn);
            CREATE INDEX IF NOT EXISTS idx_edge_src ON edge(src_symbol);
            CREATE INDEX IF NOT EXISTS idx_edge_dst ON edge(dst_symbol);
            CREATE INDEX IF NOT EXISTS idx_edge_type ON edge(edge_type);
            CREATE INDEX IF NOT EXISTS idx_edge_resolution ON edge(resolution);
            CREATE INDEX IF NOT EXISTS idx_occurrence_file ON occurrence(file_path);
            CREATE INDEX IF NOT EXISTS idx_occurrence_symbol ON occurrence(symbol_id);
            
            -- FTS5 virtual table for full-text search on symbols
            CREATE VIRTUAL TABLE IF NOT EXISTS symbol_fts USING fts5(
                symbol_id UNINDEXED,
                name,
                fqn,
                doc,
                file_path,
                content=symbol,
                content_rowid=id,
                tokenize='porter unicode61'
            );
            
            -- Triggers to keep FTS index in sync
            CREATE TRIGGER IF NOT EXISTS symbol_fts_insert AFTER INSERT ON symbol BEGIN
                INSERT INTO symbol_fts(rowid, symbol_id, name, fqn, doc, file_path)
                VALUES (new.id, new.symbol_id, new.name, new.fqn, new.doc, new.file_path);
            END;
            
            CREATE TRIGGER IF NOT EXISTS symbol_fts_delete AFTER DELETE ON symbol BEGIN
                DELETE FROM symbol_fts WHERE rowid = old.id;
            END;
            
            CREATE TRIGGER IF NOT EXISTS symbol_fts_update AFTER UPDATE ON symbol BEGIN
                DELETE FROM symbol_fts WHERE rowid = old.id;
                INSERT INTO symbol_fts(rowid, symbol_id, name, fqn, doc, file_path)
                VALUES (new.id, new.symbol_id, new.name, new.fqn, new.doc, new.file_path);
            END;
            
            COMMIT;
            "#,
        )?;
        
        // Add indexes for better query performance
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_symbol_fqn ON symbol(fqn);
             CREATE INDEX IF NOT EXISTS idx_symbol_file ON symbol(file_path);
             CREATE INDEX IF NOT EXISTS idx_edge_src ON edge(src);
             CREATE INDEX IF NOT EXISTS idx_edge_dst ON edge(dst);
             CREATE INDEX IF NOT EXISTS idx_edge_type ON edge(edge_type);
             CREATE INDEX IF NOT EXISTS idx_occurrence_symbol ON occurrence(symbol_id);
             CREATE INDEX IF NOT EXISTS idx_file_commit ON file(commit_id, path);"
        )?;
        
        info!("Database schema initialized at {:?}", self.db_path);
        Ok(())
    }
    
    pub fn get_or_create_commit(&self, commit_sha: &str) -> Result<i64> {
        // First, try to get existing commit
        if let Some(id) = self.conn.query_row(
            "SELECT id FROM commit_snapshot WHERE commit_sha = ?1",
            params![commit_sha],
            |row| row.get::<_, i64>(0),
        ).optional()? {
            return Ok(id);
        }
        
        // Create new commit
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        
        self.conn.execute(
            "INSERT INTO commit_snapshot (commit_sha, timestamp) VALUES (?1, ?2)",
            params![commit_sha, timestamp],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    pub fn insert_file(&self, commit_id: i64, path: &str, content_hash: &str, size: usize) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO file (commit_id, path, content_hash, size_bytes) 
             VALUES (?1, ?2, ?3, ?4)",
            params![commit_id, path, content_hash, size as i64],
        )?;
        Ok(())
    }
    
    pub fn insert_symbol(&self, commit_id: i64, symbol: &SymbolIR) -> Result<()> {
        let lang_str = serde_json::to_string(&symbol.lang)?;
        let kind_str = serde_json::to_string(&symbol.kind)?;
        let visibility_str = symbol.visibility.as_ref().map(|v| serde_json::to_string(v)).transpose()?;
        
        self.conn.execute(
            r#"INSERT OR REPLACE INTO symbol 
            (commit_id, symbol_id, lang, kind, name, fqn, signature, 
             file_path, span_start_line, span_start_col, span_end_line, 
             span_end_col, visibility, doc, sig_hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"#,
            params![
                commit_id,
                symbol.id,
                lang_str,
                kind_str,
                symbol.name,
                symbol.fqn,
                symbol.signature,
                symbol.file_path,
                symbol.span.start_line,
                symbol.span.start_col,
                symbol.span.end_line,
                symbol.span.end_col,
                visibility_str,
                symbol.doc,
                symbol.sig_hash,
            ],
        )?;
        
        Ok(())
    }
    
    pub fn insert_edge(&self, commit_id: i64, edge: &EdgeIR) -> Result<()> {
        let edge_type_str = serde_json::to_string(&edge.edge_type)?;
        let resolution_str = serde_json::to_string(&edge.resolution)?;
        
        self.conn.execute(
            r#"INSERT INTO edge 
            (commit_id, edge_type, src_symbol, dst_symbol, file_src, file_dst, resolution)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                commit_id,
                edge_type_str,
                edge.src,
                edge.dst,
                edge.file_src,
                edge.file_dst,
                resolution_str,
            ],
        )?;
        
        Ok(())
    }
    
    pub fn insert_occurrence(&self, commit_id: i64, occurrence: &OccurrenceIR) -> Result<()> {
        let role_str = serde_json::to_string(&occurrence.role)?;
        
        self.conn.execute(
            r#"INSERT INTO occurrence 
            (commit_id, file_path, symbol_id, role, span_start_line, 
             span_start_col, span_end_line, span_end_col, token)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                commit_id,
                occurrence.file_path,
                occurrence.symbol_id,
                role_str,
                occurrence.span.start_line,
                occurrence.span.start_col,
                occurrence.span.end_line,
                occurrence.span.end_col,
                occurrence.token,
            ],
        )?;
        
        Ok(())
    }
    
    pub fn get_latest_commit(&self) -> Result<Option<String>> {
        let commit = self.conn.query_row(
            "SELECT commit_sha FROM commit_snapshot ORDER BY timestamp DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        ).optional()?;
        
        Ok(commit)
    }
    
    pub fn get_file_hash(&self, commit_sha: &str, file_path: &str) -> Result<Option<String>> {
        let hash = self.conn.query_row(
            r#"SELECT f.content_hash 
               FROM file f
               JOIN commit_snapshot c ON f.commit_id = c.id
               WHERE c.commit_sha = ?1 AND f.path = ?2"#,
            params![commit_sha, file_path],
            |row| row.get::<_, String>(0),
        ).optional()?;
        
        Ok(hash)
    }
    
    pub fn get_files_in_commit(&self, commit_sha: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT f.path, f.content_hash
               FROM file f
               JOIN commit_snapshot c ON f.commit_id = c.id
               WHERE c.commit_sha = ?1"#
        )?;
        
        let files = stmt.query_map(params![commit_sha], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(files)
    }
    
    pub fn clear_file_data(&self, commit_id: i64, file_path: &str) -> Result<()> {
        // Delete symbols
        self.conn.execute(
            "DELETE FROM symbol WHERE commit_id = ?1 AND file_path = ?2",
            params![commit_id, file_path],
        )?;
        
        // Delete occurrences
        self.conn.execute(
            "DELETE FROM occurrence WHERE commit_id = ?1 AND file_path = ?2",
            params![commit_id, file_path],
        )?;
        
        // Delete edges related to this file
        self.conn.execute(
            "DELETE FROM edge WHERE commit_id = ?1 AND (file_src = ?2 OR file_dst = ?2)",
            params![commit_id, file_path],
        )?;
        
        Ok(())
    }
    
    pub fn build_graph(&self) -> Result<CodeGraph> {
        // Get all symbols
        let mut stmt = self.conn.prepare(
            "SELECT symbol_id, name, kind FROM symbol"
        )?;
        
        let symbols: Vec<(String, String, String)> = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        // Get all edges
        let mut stmt = self.conn.prepare(
            "SELECT edge_type, src_symbol, dst_symbol FROM edge WHERE src_symbol IS NOT NULL AND dst_symbol IS NOT NULL"
        )?;
        
        let edges: Vec<(String, String, String)> = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        // Build the graph
        let mut graph = CodeGraph::new();
        
        // Add symbols as nodes
        for (id, _name, _kind) in symbols {
            graph.add_symbol(&id);
        }
        
        // Add edges
        for (edge_type_str, src, dst) in edges {
            let edge_type: EdgeType = serde_json::from_str(&edge_type_str)?;
            graph.add_edge(&src, &dst, edge_type);
        }
        
        Ok(graph)
    }
    
    pub fn get_symbol(&self, symbol_id: &str) -> Result<Option<SymbolIR>> {
        let symbol = self.conn.query_row(
            r#"SELECT symbol_id, lang, kind, name, fqn, signature, file_path,
                     span_start_line, span_start_col, span_end_line, span_end_col,
                     visibility, doc, sig_hash
               FROM symbol 
               WHERE symbol_id = ?1
               LIMIT 1"#,
            params![symbol_id],
            |row| {
                Ok(SymbolIR {
                    id: row.get(0)?,
                    lang: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(Language::Unknown),
                    kind: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or(SymbolKind::Variable),
                    name: row.get(3)?,
                    fqn: row.get(4)?,
                    signature: row.get(5)?,
                    file_path: row.get(6)?,
                    span: Span {
                        start_line: row.get(7)?,
                        start_col: row.get(8)?,
                        end_line: row.get(9)?,
                        end_col: row.get(10)?,
                    },
                    visibility: row.get::<_, Option<String>>(11)?
                        .and_then(|v| serde_json::from_str(&v).ok()),
                    doc: row.get(12)?,
                    sig_hash: row.get(13)?,
                })
            }
        ).optional()?;
        
        Ok(symbol)
    }
    
    pub fn get_edges(&self, symbol_id: &str) -> Result<Vec<EdgeIR>> {
        let mut edges = Vec::new();
        
        // Get outgoing edges
        let mut stmt = self.conn.prepare(
            r#"SELECT edge_type, src_symbol, dst_symbol, file_src, file_dst, resolution
               FROM edge 
               WHERE src_symbol = ?1"#
        )?;
        
        let edge_iter = stmt.query_map(params![symbol_id], |row| {
            Ok(EdgeIR {
                edge_type: serde_json::from_str(&row.get::<_, String>(0)?).unwrap_or(EdgeType::Contains),
                src: row.get(1)?,
                dst: row.get(2)?,
                file_src: row.get(3)?,
                file_dst: row.get(4)?,
                resolution: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(protocol::Resolution::Syntactic),
                meta: std::collections::HashMap::new(),
                provenance: std::collections::HashMap::new(),
            })
        })?;
        
        for edge in edge_iter {
            edges.push(edge?);
        }
        
        // Get incoming edges
        let mut stmt = self.conn.prepare(
            r#"SELECT edge_type, src_symbol, dst_symbol, file_src, file_dst, resolution
               FROM edge 
               WHERE dst_symbol = ?1"#
        )?;
        
        let edge_iter = stmt.query_map(params![symbol_id], |row| {
            Ok(EdgeIR {
                edge_type: serde_json::from_str(&row.get::<_, String>(0)?).unwrap_or(EdgeType::Contains),
                src: row.get(1)?,
                dst: row.get(2)?,
                file_src: row.get(3)?,
                file_dst: row.get(4)?,
                resolution: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(protocol::Resolution::Syntactic),
                meta: std::collections::HashMap::new(),
                provenance: std::collections::HashMap::new(),
            })
        })?;
        
        for edge in edge_iter {
            edges.push(edge?);
        }
        
        Ok(edges)
    }
    
    pub fn get_symbol_by_fqn(&self, fqn: &str) -> Result<Option<SymbolIR>> {
        let symbol = self.conn.query_row(
            r#"SELECT symbol_id, lang, kind, name, fqn, signature, file_path,
                     span_start_line, span_start_col, span_end_line, span_end_col,
                     visibility, doc, sig_hash
               FROM symbol 
               WHERE fqn = ?1
               ORDER BY id DESC
               LIMIT 1"#,
            params![fqn],
            |row| {
                Ok(SymbolIR {
                    id: row.get(0)?,
                    lang: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(Language::Unknown),
                    kind: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or(SymbolKind::Variable),
                    name: row.get(3)?,
                    fqn: row.get(4)?,
                    signature: row.get(5)?,
                    file_path: row.get(6)?,
                    span: Span {
                        start_line: row.get(7)?,
                        start_col: row.get(8)?,
                        end_line: row.get(9)?,
                        end_col: row.get(10)?,
                    },
                    visibility: row.get::<_, Option<String>>(11)?
                        .and_then(|v| serde_json::from_str(&v).ok()),
                    doc: row.get(12)?,
                    sig_hash: row.get(13)?,
                })
            }
        ).optional()?;
        
        Ok(symbol)
    }

    /// Search symbols using FTS5 full-text search for fast fuzzy matching
    pub fn search_symbols_fts(&self, query: &str, limit: usize) -> Result<Vec<SymbolIR>> {
        let mut symbols = Vec::new();
        
        // Use FTS5 MATCH for fast full-text searching with ranking
        let mut stmt = self.conn.prepare(
            r#"
            SELECT s.symbol_id, s.lang, s.kind, s.name, s.fqn, s.signature, s.file_path,
                   s.span_start_line, s.span_start_col, s.span_end_line, s.span_end_col,
                   s.visibility, s.doc, s.sig_hash
            FROM symbol_fts
            JOIN symbol s ON symbol_fts.rowid = s.id
            WHERE symbol_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        )?;
        
        let symbol_iter = stmt.query_map(params![query, limit], |row| {
            Ok(SymbolIR {
                id: row.get(0)?,
                lang: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(Language::Unknown),
                kind: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or(SymbolKind::Variable),
                name: row.get(3)?,
                fqn: row.get(4)?,
                signature: row.get(5)?,
                file_path: row.get(6)?,
                span: Span {
                    start_line: row.get(7)?,
                    start_col: row.get(8)?,
                    end_line: row.get(9)?,
                    end_col: row.get(10)?,
                },
                visibility: row.get::<_, Option<String>>(11)?
                    .and_then(|v| serde_json::from_str(&v).ok()),
                doc: row.get(12)?,
                sig_hash: row.get(13)?,
            })
        })?;
        
        for symbol in symbol_iter {
            symbols.push(symbol?);
        }
        
        Ok(symbols)
    }
    
    pub fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<SymbolIR>> {
        // Try FTS5 first for better performance
        if let Ok(results) = self.search_symbols_fts(query, limit) {
            if !results.is_empty() {
                return Ok(results);
            }
        }
        
        let mut symbols = Vec::new();
        
        // Fall back to LIKE search
        let pattern = format!("%{}%", query);
        
        let mut stmt = self.conn.prepare(
            r#"
            SELECT symbol_id, lang, kind, name, fqn, signature, file_path,
                   span_start_line, span_start_col, span_end_line, span_end_col,
                   visibility, doc, sig_hash
            FROM symbol 
            WHERE name LIKE ?1 OR fqn LIKE ?1
            ORDER BY 
                CASE WHEN name = ?2 THEN 0
                     WHEN name LIKE ?3 THEN 1
                     ELSE 2 END,
                length(name)
            LIMIT ?4
            "#,
        )?;
        
        let exact = query;
        let prefix = format!("{}%", query);
        
        let symbol_iter = stmt.query_map(params![pattern, exact, prefix, limit], |row| {
            Ok(SymbolIR {
                id: row.get(0)?,
                lang: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(Language::Unknown),
                kind: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or(SymbolKind::Variable),
                name: row.get(3)?,
                fqn: row.get(4)?,
                signature: row.get(5)?,
                file_path: row.get(6)?,
                span: Span {
                    start_line: row.get(7)?,
                    start_col: row.get(8)?,
                    end_line: row.get(9)?,
                    end_col: row.get(10)?,
                },
                visibility: row.get::<_, Option<String>>(11)?
                    .and_then(|v| serde_json::from_str(&v).ok()),
                doc: row.get(12)?,
                sig_hash: row.get(13)?,
            })
        })?;
        
        for symbol in symbol_iter {
            symbols.push(symbol?);
        }
        
        Ok(symbols)
    }
    
    pub fn get_symbols_in_file(&self, file_path: &str) -> Result<Vec<SymbolIR>> {
        let mut symbols = Vec::new();
        
        let mut stmt = self.conn.prepare(
            r#"
            SELECT symbol_id, lang, kind, name, fqn, signature, file_path,
                   span_start_line, span_start_col, span_end_line, span_end_col,
                   visibility, doc, sig_hash
            FROM symbol 
            WHERE file_path = ?1
            ORDER BY span_start_line, span_start_col
            "#,
        )?;
        
        let symbol_iter = stmt.query_map(params![file_path], |row| {
            Ok(SymbolIR {
                id: row.get(0)?,
                lang: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(Language::Unknown),
                kind: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or(SymbolKind::Variable),
                name: row.get(3)?,
                fqn: row.get(4)?,
                signature: row.get(5)?,
                file_path: row.get(6)?,
                span: Span {
                    start_line: row.get(7)?,
                    start_col: row.get(8)?,
                    end_line: row.get(9)?,
                    end_col: row.get(10)?,
                },
                visibility: row.get::<_, Option<String>>(11)?
                    .and_then(|v| serde_json::from_str(&v).ok()),
                doc: row.get(12)?,
                sig_hash: row.get(13)?,
            })
        })?;
        
        for symbol in symbol_iter {
            symbols.push(symbol?);
        }
        
        Ok(symbols)
    }
    
    pub fn get_symbol_count(&self) -> Result<usize> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM symbol",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        
        Ok(count as usize)
    }
    
    pub fn get_edge_count(&self) -> Result<usize> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM edge",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        
        Ok(count as usize)
    }
    
    pub fn get_file_count(&self) -> Result<usize> {
        let count = self.conn.query_row(
            "SELECT COUNT(DISTINCT path) FROM file",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        
        Ok(count as usize)
    }
    
    // Additional methods needed by the main binary
    
    pub fn get_last_scanned_commit(&self) -> Result<Option<String>> {
        // Same as get_latest_commit
        self.get_latest_commit()
    }
    
    pub fn create_commit_snapshot(&self, commit_sha: &str) -> Result<i64> {
        // Same as get_or_create_commit
        self.get_or_create_commit(commit_sha)
    }
    
    pub fn delete_file_data(&self, commit_id: i64, file_path: &str) -> Result<()> {
        // Same as clear_file_data
        self.clear_file_data(commit_id, file_path)
    }
    
    pub fn find_symbol_by_fqn(&self, fqn: &str) -> Result<Option<SymbolIR>> {
        // Same as get_symbol_by_fqn
        self.get_symbol_by_fqn(fqn)
    }
    
    pub fn find_symbol_by_id(&self, symbol_id: &str) -> Result<Option<SymbolIR>> {
        // Same as get_symbol
        self.get_symbol(symbol_id)
    }
    
    pub fn get_callers(&self, symbol_id: &str, max_depth: usize) -> Result<Vec<SymbolIR>> {
        // Build graph and find callers
        let graph = self.build_graph()?;
        let caller_ids = graph.find_callers(symbol_id, max_depth);
        
        let mut callers = Vec::new();
        for id in caller_ids {
            if let Some(symbol) = self.get_symbol(&id)? {
                callers.push(symbol);
            }
        }
        Ok(callers)
    }
    
    pub fn get_callees(&self, symbol_id: &str, max_depth: usize) -> Result<Vec<SymbolIR>> {
        // Build graph and find callees
        let graph = self.build_graph()?;
        let callee_ids = graph.find_callees(symbol_id, max_depth);
        
        let mut callees = Vec::new();
        for id in callee_ids {
            if let Some(symbol) = self.get_symbol(&id)? {
                callees.push(symbol);
            }
        }
        Ok(callees)
    }
    
    pub fn get_file_dependents(&self, file_path: &str) -> Result<Vec<String>> {
        // Find files that import/depend on this file
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT file_src FROM edge 
             WHERE file_dst = ?1 AND edge_type = 'Imports'"
        )?;
        
        let dependents = stmt.query_map([file_path], |row| {
            row.get::<_, String>(0)
        })?
        .filter_map(Result::ok)
        .collect();
        
        Ok(dependents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use protocol::{EdgeType, Language, OccurrenceRole, Resolution, SymbolKind};
    use std::collections::HashMap;
    
    fn create_test_store() -> Result<(GraphStore, TempDir)> {
        let temp_dir = TempDir::new()?;
        let store = GraphStore::new(temp_dir.path())?;
        Ok((store, temp_dir))
    }
    
    fn create_test_symbol(id: &str, name: &str) -> SymbolIR {
        SymbolIR {
            id: id.to_string(),
            lang: Language::TypeScript,
            kind: SymbolKind::Function,
            name: name.to_string(),
            fqn: format!("test.{}", name),
            signature: Some(format!("function {}()", name)),
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 10,
            },
            visibility: Some("public".to_string()),
            doc: Some("Test function".to_string()),
            sig_hash: format!("hash_{}", id),
        }
    }
    
    #[test]
    fn test_store_creation() -> Result<()> {
        let (_store, _temp_dir) = create_test_store()?;
        Ok(())
    }
    
    #[test]
    fn test_commit_operations() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        
        // Test creating a commit
        let commit_id = store.get_or_create_commit("abc123")?;
        assert!(commit_id > 0);
        
        // Test getting the same commit (should not create new)
        let commit_id2 = store.get_or_create_commit("abc123")?;
        assert_eq!(commit_id, commit_id2);
        
        // Sleep to ensure different timestamp (SQLite timestamps are in seconds)
        std::thread::sleep(std::time::Duration::from_secs(1));
        
        // Test creating different commit
        let commit_id3 = store.get_or_create_commit("def456")?;
        assert_ne!(commit_id, commit_id3);
        
        // Test getting latest commit
        let latest = store.get_latest_commit()?;
        assert_eq!(latest, Some("def456".to_string()));
        
        Ok(())
    }
    
    #[test]
    fn test_file_operations() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Insert a file
        store.insert_file(commit_id, "src/main.rs", "hash123", 1024)?;
        
        // Get file hash
        let hash = store.get_file_hash("test_commit", "src/main.rs")?;
        assert_eq!(hash, Some("hash123".to_string()));
        
        // Get files in commit
        let files = store.get_files_in_commit("test_commit")?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "src/main.rs");
        assert_eq!(files[0].1, "hash123");
        
        // Test non-existent file
        let hash = store.get_file_hash("test_commit", "nonexistent.rs")?;
        assert_eq!(hash, None);
        
        Ok(())
    }
    
    #[test]
    fn test_symbol_operations() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        let symbol = create_test_symbol("sym1", "testFunc");
        store.insert_symbol(commit_id, &symbol)?;
        
        // Get symbol by ID
        let retrieved = store.get_symbol("sym1")?;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "testFunc");
        assert_eq!(retrieved.id, "sym1");
        
        // Get symbol by FQN
        let by_fqn = store.get_symbol_by_fqn("test.testFunc")?;
        assert!(by_fqn.is_some());
        assert_eq!(by_fqn.unwrap().id, "sym1");
        
        // Get symbols in file
        let in_file = store.get_symbols_in_file("test.ts")?;
        assert_eq!(in_file.len(), 1);
        assert_eq!(in_file[0].id, "sym1");
        
        // Test symbol count
        let count = store.get_symbol_count()?;
        assert_eq!(count, 1);
        
        // Test non-existent symbol
        let missing = store.get_symbol("nonexistent")?;
        assert!(missing.is_none());
        
        Ok(())
    }
    
    #[test]
    fn test_edge_operations() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Insert symbols
        let sym1 = create_test_symbol("sym1", "func1");
        let sym2 = create_test_symbol("sym2", "func2");
        store.insert_symbol(commit_id, &sym1)?;
        store.insert_symbol(commit_id, &sym2)?;
        
        // Insert edge
        let edge = EdgeIR {
            edge_type: EdgeType::Calls,
            src: Some("sym1".to_string()),
            dst: Some("sym2".to_string()),
            file_src: Some("test.ts".to_string()),
            file_dst: Some("test.ts".to_string()),
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        };
        store.insert_edge(commit_id, &edge)?;
        
        // Get edges for symbol
        let edges = store.get_edges("sym1")?;
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].src, Some("sym1".to_string()));
        assert_eq!(edges[0].dst, Some("sym2".to_string()));
        
        // Test edge count
        let count = store.get_edge_count()?;
        assert_eq!(count, 1);
        
        Ok(())
    }
    
    #[test]
    fn test_occurrence_operations() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        let occurrence = OccurrenceIR {
            file_path: "test.ts".to_string(),
            symbol_id: Some("sym1".to_string()),
            role: OccurrenceRole::Definition,
            span: Span {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 10,
            },
            token: "testFunc".to_string(),
        };
        
        store.insert_occurrence(commit_id, &occurrence)?;
        
        // Verify insertion (would need to add a getter method)
        Ok(())
    }
    
    #[test]
    fn test_search_operations() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Insert various symbols
        let symbols = vec![
            create_test_symbol("s1", "getUserById"),
            create_test_symbol("s2", "setUserName"),
            create_test_symbol("s3", "deleteUser"),
            create_test_symbol("s4", "AdminUser"),
            create_test_symbol("s5", "normalFunction"),
        ];
        
        for sym in &symbols {
            store.insert_symbol(commit_id, sym)?;
        }
        
        // Search for "User"
        let results = store.search_symbols("User", 10)?;
        assert_eq!(results.len(), 4); // Should find getUserById, setUserName, deleteUser, AdminUser
        
        // Search with limit
        let results = store.search_symbols("User", 2)?;
        assert_eq!(results.len(), 2);
        
        // Search for exact match
        let results = store.search_symbols("normalFunction", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "normalFunction");
        
        // Search for non-existent
        let results = store.search_symbols("nonexistent", 10)?;
        assert_eq!(results.len(), 0);
        
        Ok(())
    }
    
    #[test]
    fn test_fts5_search() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Insert symbols with documentation
        let mut sym1 = create_test_symbol("s1", "processUserData");
        sym1.doc = Some("Process user data and validate inputs".to_string());
        
        let mut sym2 = create_test_symbol("s2", "validateEmail");
        sym2.doc = Some("Validate email format according to RFC".to_string());
        
        let mut sym3 = create_test_symbol("s3", "sendNotification");
        sym3.doc = Some("Send notification to user via email".to_string());
        
        store.insert_symbol(commit_id, &sym1)?;
        store.insert_symbol(commit_id, &sym2)?;
        store.insert_symbol(commit_id, &sym3)?;
        
        // FTS5 search should match on documentation too
        let results = store.search_symbols_fts("validate", 10)?;
        assert_eq!(results.len(), 2); // Should find both processUserData and validateEmail
        
        // Test prefix matching (FTS5 does prefix, not fuzzy)
        let results = store.search_symbols_fts("send*", 10)?;
        assert!(results.len() > 0); // Should find sendNotification
        
        Ok(())
    }
    
    #[test]
    fn test_clear_file_data() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Insert symbol and occurrence
        let symbol = create_test_symbol("sym1", "testFunc");
        store.insert_symbol(commit_id, &symbol)?;
        
        let occurrence = OccurrenceIR {
            file_path: "test.ts".to_string(),
            symbol_id: Some("sym1".to_string()),
            role: OccurrenceRole::Definition,
            span: Span {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 10,
            },
            token: "testFunc".to_string(),
        };
        store.insert_occurrence(commit_id, &occurrence)?;
        
        // Clear file data
        store.clear_file_data(commit_id, "test.ts")?;
        
        // Symbol should be gone
        let symbols = store.get_symbols_in_file("test.ts")?;
        assert_eq!(symbols.len(), 0);
        
        Ok(())
    }
    
    #[test]
    fn test_graph_building() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Create a small graph
        let sym1 = create_test_symbol("s1", "main");
        let sym2 = create_test_symbol("s2", "helper");
        let sym3 = create_test_symbol("s3", "util");
        
        store.insert_symbol(commit_id, &sym1)?;
        store.insert_symbol(commit_id, &sym2)?;
        store.insert_symbol(commit_id, &sym3)?;
        
        // main calls helper
        store.insert_edge(commit_id, &EdgeIR {
            edge_type: EdgeType::Calls,
            src: Some("s1".to_string()),
            dst: Some("s2".to_string()),
            file_src: None,
            file_dst: None,
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        })?;
        
        // helper calls util
        store.insert_edge(commit_id, &EdgeIR {
            edge_type: EdgeType::Calls,
            src: Some("s2".to_string()),
            dst: Some("s3".to_string()),
            file_src: None,
            file_dst: None,
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        })?;
        
        let graph = store.build_graph()?;
        let stats = graph.stats();
        
        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 2);
        assert!(!stats.is_cyclic);
        
        Ok(())
    }
    
    #[test]
    fn test_idempotency() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        let symbol = create_test_symbol("sym1", "testFunc");
        
        // Insert same symbol twice
        store.insert_symbol(commit_id, &symbol)?;
        store.insert_symbol(commit_id, &symbol)?;
        
        // Should only have one symbol
        let count = store.get_symbol_count()?;
        assert_eq!(count, 1);
        
        Ok(())
    }
    
    #[test]
    fn test_unicode_symbols() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Test with unicode characters
        let mut symbol = create_test_symbol("sym_unicode", "测试函数");
        symbol.doc = Some("这是一个测试函数 with émojis 😀".to_string());
        
        store.insert_symbol(commit_id, &symbol)?;
        
        let retrieved = store.get_symbol("sym_unicode")?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "测试函数");
        
        // Search for unicode
        let results = store.search_symbols("测试", 10)?;
        assert_eq!(results.len(), 1);
        
        Ok(())
    }
    
    #[test]
    fn test_very_long_names() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Create symbol with very long name
        let long_name = "a".repeat(1000);
        let mut symbol = create_test_symbol("sym_long", &long_name);
        symbol.fqn = format!("test.{}", long_name);
        
        store.insert_symbol(commit_id, &symbol)?;
        
        let retrieved = store.get_symbol("sym_long")?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name.len(), 1000);
        
        Ok(())
    }
    
    #[test]
    fn test_empty_values() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Symbol with minimal/empty values
        let symbol = SymbolIR {
            id: "empty".to_string(),
            lang: Language::Unknown,
            kind: SymbolKind::Variable,
            name: "".to_string(), // Empty name
            fqn: "".to_string(),  // Empty FQN
            signature: None,
            file_path: "".to_string(), // Empty path
            span: Span {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            },
            visibility: None,
            doc: None,
            sig_hash: "".to_string(),
        };
        
        store.insert_symbol(commit_id, &symbol)?;
        
        let retrieved = store.get_symbol("empty")?;
        assert!(retrieved.is_some());
        
        Ok(())
    }
    
    #[test]
    fn test_special_characters_in_paths() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Test with special characters in file path
        let mut symbol = create_test_symbol("sym1", "test");
        symbol.file_path = "src/with spaces/and-dashes/under_scores/file.ts".to_string();
        
        store.insert_symbol(commit_id, &symbol)?;
        
        let in_file = store.get_symbols_in_file("src/with spaces/and-dashes/under_scores/file.ts")?;
        assert_eq!(in_file.len(), 1);
        
        Ok(())
    }
    
    #[test]
    fn test_sql_injection_protection() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Try to inject SQL
        let mut symbol = create_test_symbol("sym1", "test');DROP TABLE symbol;--");
        symbol.fqn = "'; DROP TABLE symbol; --".to_string();
        
        store.insert_symbol(commit_id, &symbol)?;
        
        // Table should still exist
        let count = store.get_symbol_count()?;
        assert_eq!(count, 1);
        
        // Search with injection attempt
        let results = store.search_symbols("'; DROP TABLE symbol; --", 10)?;
        assert_eq!(results.len(), 1);
        
        Ok(())
    }
    
    // Note: Concurrent test removed because SQLite connections are not thread-safe (not Send)
    // In production, you'd use a connection pool or separate connections per thread
    
    #[test]
    fn test_cycle_detection_in_graph() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        let commit_id = store.get_or_create_commit("test_commit")?;
        
        // Create symbols
        for i in 1..=3 {
            let sym = create_test_symbol(&format!("s{}", i), &format!("func{}", i));
            store.insert_symbol(commit_id, &sym)?;
        }
        
        // Create a cycle: s1 -> s2 -> s3 -> s1
        let edges = vec![
            ("s1", "s2"),
            ("s2", "s3"),
            ("s3", "s1"), // Creates cycle
        ];
        
        for (src, dst) in edges {
            store.insert_edge(commit_id, &EdgeIR {
                edge_type: EdgeType::Calls,
                src: Some(src.to_string()),
                dst: Some(dst.to_string()),
                file_src: None,
                file_dst: None,
                resolution: Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            })?;
        }
        
        let graph = store.build_graph()?;
        let stats = graph.stats();
        
        assert!(stats.is_cyclic, "Should detect cycle");
        
        // Test finding cycles
        let cycles = graph.find_cycles_containing("s1");
        assert!(!cycles.is_empty(), "Should find at least one cycle");
        
        Ok(())
    }
    
    #[test]
    fn test_file_count_distinct() -> Result<()> {
        let (store, _temp_dir) = create_test_store()?;
        
        // Insert same file in multiple commits
        let commit1 = store.get_or_create_commit("commit1")?;
        let commit2 = store.get_or_create_commit("commit2")?;
        
        store.insert_file(commit1, "file.rs", "hash1", 100)?;
        store.insert_file(commit2, "file.rs", "hash2", 200)?;
        store.insert_file(commit1, "other.rs", "hash3", 300)?;
        
        // Should count distinct paths
        let count = store.get_file_count()?;
        assert_eq!(count, 2); // file.rs and other.rs
        
        Ok(())
    }
}