use anyhow::Result;
use protocol::{EdgeIR, EdgeType, Language, OccurrenceIR, Span, SymbolIR, SymbolKind};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::info;

mod graph;
mod connection_manager;
pub use graph::{CodeGraph, GraphStats};
pub use connection_manager::{get_shared_connection, execute_with_retry, execute_batch_transaction};

pub struct GraphStore {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl GraphStore {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let db_dir = repo_path.join(".reviewbot");
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("graph.db");

        let db_exists = db_path.exists();

        // Get shared connection instead of creating a new one
        let conn = get_shared_connection(&db_path)?;

        let store = Self { db_path, conn };

        // If database already exists, verify integrity before using
        if db_exists {
            store.verify_database_integrity()?;
        }

        store.init_schema()?;
        Ok(store)
    }

    /// Verify database integrity to detect corruption
    fn verify_database_integrity(&self) -> Result<()> {
        execute_with_retry(&self.conn, |connection| {
            // Run SQLite integrity check
            let result: String = connection.query_row(
                "PRAGMA integrity_check",
                [],
                |row| row.get(0)
            )?;

            if result != "ok" {
                return Err(anyhow::anyhow!(
                    "Database integrity check failed: {}. The database may be corrupted. \
                     Consider deleting .reviewbot/graph.db and re-running the scan.",
                    result
                ));
            }

            Ok(())
        }, 3)
    }
    
    fn init_schema(&self) -> Result<()> {
        execute_with_retry(&self.conn, |connection| {
            connection.execute_batch(
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
            connection.execute_batch(
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
        }, 3)
    }
    
    pub fn get_or_create_commit(&self, commit_sha: &str) -> Result<i64> {
        execute_with_retry(&self.conn, |connection| {
            // First, try to get existing commit
            if let Some(id) = connection.query_row(
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

            connection.execute(
                "INSERT INTO commit_snapshot (commit_sha, timestamp) VALUES (?1, ?2)",
                params![commit_sha, timestamp],
            )?;

            Ok(connection.last_insert_rowid())
        }, 3)
    }
    
    pub fn insert_file(&self, commit_id: i64, path: &str, content_hash: &str, size: usize) -> Result<()> {
        execute_with_retry(&self.conn, |connection| {
            connection.execute(
                "INSERT OR REPLACE INTO file (commit_id, path, content_hash, size_bytes)
                 VALUES (?1, ?2, ?3, ?4)",
                params![commit_id, path, content_hash, size as i64],
            )?;
            Ok(())
        }, 3)
    }
    
    pub fn insert_symbol(&self, commit_id: i64, symbol: &SymbolIR) -> Result<()> {
        // Validate symbol has non-empty name and FQN
        if symbol.name.is_empty() || symbol.fqn.is_empty() {
            return Err(anyhow::anyhow!("Symbol name and FQN cannot be empty"));
        }

        let lang_str = serde_json::to_string(&symbol.lang)?;
        let kind_str = serde_json::to_string(&symbol.kind)?;
        let visibility_str = symbol.visibility.as_ref().map(serde_json::to_string).transpose()?;
        
        execute_with_retry(&self.conn, |connection| {
            connection.execute(
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
            ])?;
            Ok(())
        }, 3)
    }
    
    pub fn insert_edge(&self, commit_id: i64, edge: &EdgeIR) -> Result<()> {
        let edge_type_str = serde_json::to_string(&edge.edge_type)?;
        let resolution_str = serde_json::to_string(&edge.resolution)?;

        execute_with_retry(&self.conn, |connection| {
            connection.execute(
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
        }, 3)
    }
    
    pub fn insert_occurrence(&self, commit_id: i64, occurrence: &OccurrenceIR) -> Result<()> {
        let role_str = serde_json::to_string(&occurrence.role)?;

        execute_with_retry(&self.conn, |connection| {
            connection.execute(
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
        }, 3)
    }

    /// Batch insert symbols in a single transaction for better performance
    pub fn batch_insert_symbols(&self, commit_id: i64, symbols: &[SymbolIR]) -> Result<()> {
        if symbols.is_empty() {
            return Ok(());
        }

        let connection = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire connection lock: {}", e)
        })?;

        let tx = connection.unchecked_transaction()?;

        for symbol in symbols {
            // Validate symbol has non-empty name and FQN
            if symbol.name.is_empty() || symbol.fqn.is_empty() {
                return Err(anyhow::anyhow!("Symbol name and FQN cannot be empty"));
            }

            let lang_str = serde_json::to_string(&symbol.lang)?;
            let kind_str = serde_json::to_string(&symbol.kind)?;
            let visibility_str = symbol.visibility.as_ref().map(serde_json::to_string).transpose()?;

            tx.execute(
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
                ]
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Batch insert edges in a single transaction for better performance
    pub fn batch_insert_edges(&self, commit_id: i64, edges: &[EdgeIR]) -> Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        let connection = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire connection lock: {}", e)
        })?;

        let tx = connection.unchecked_transaction()?;

        for edge in edges {
            let edge_type_str = serde_json::to_string(&edge.edge_type)?;
            let resolution_str = serde_json::to_string(&edge.resolution)?;

            tx.execute(
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
                ]
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Batch insert occurrences in a single transaction for better performance
    pub fn batch_insert_occurrences(&self, commit_id: i64, occurrences: &[OccurrenceIR]) -> Result<()> {
        if occurrences.is_empty() {
            return Ok(());
        }

        let connection = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire connection lock: {}", e)
        })?;

        let tx = connection.unchecked_transaction()?;

        for occurrence in occurrences {
            let role_str = serde_json::to_string(&occurrence.role)?;

            tx.execute(
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
                ]
            )?;
        }

        tx.commit()?;
        Ok(())
    }
    
    pub fn get_latest_commit(&self) -> Result<Option<String>> {
        execute_with_retry(&self.conn, |connection| {
            let commit = connection.query_row(
                "SELECT commit_sha FROM commit_snapshot ORDER BY timestamp DESC, id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            ).optional()?;
            Ok(commit)
        }, 3)
    }
    
    pub fn get_file_hash(&self, commit_sha: &str, file_path: &str) -> Result<Option<String>> {
        execute_with_retry(&self.conn, |connection| {
            let hash = connection.query_row(
                r#"SELECT f.content_hash
                   FROM file f
                   JOIN commit_snapshot c ON f.commit_id = c.id
                   WHERE c.commit_sha = ?1 AND f.path = ?2"#,
                params![commit_sha, file_path],
                |row| row.get::<_, String>(0),
            ).optional()?;
            Ok(hash)
        }, 3)
    }
    
    pub fn get_files_in_commit(&self, commit_sha: &str) -> Result<Vec<(String, String)>> {
        execute_with_retry(&self.conn, |connection| {
            let mut stmt = connection.prepare(
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
        }, 3)
    }
    
    pub fn clear_file_data(&self, commit_id: i64, file_path: &str) -> Result<()> {
        execute_with_retry(&self.conn, |connection| {
            // Delete symbols
            connection.execute(
                "DELETE FROM symbol WHERE commit_id = ?1 AND file_path = ?2",
                params![commit_id, file_path],
            )?;

            // Delete occurrences
            connection.execute(
                "DELETE FROM occurrence WHERE commit_id = ?1 AND file_path = ?2",
                params![commit_id, file_path],
            )?;

            // Delete edges related to this file
            connection.execute(
                "DELETE FROM edge WHERE commit_id = ?1 AND (file_src = ?2 OR file_dst = ?2)",
                params![commit_id, file_path],
            )?;

            Ok(())
        }, 3)
    }
    
    pub fn build_graph(&self) -> Result<CodeGraph> {
        execute_with_retry(&self.conn, |connection| {
            // Get all symbols
            let mut stmt = connection.prepare(
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
            let mut stmt = connection.prepare(
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
        }, 3)
    }
    
    pub fn get_symbol(&self, symbol_id: &str) -> Result<Option<SymbolIR>> {
        execute_with_retry(&self.conn, |connection| {
            let symbol = connection.query_row(
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
                        lang_version: None,
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
        }, 3)
    }
    
    pub fn get_edges(&self, symbol_id: &str) -> Result<Vec<EdgeIR>> {
        execute_with_retry(&self.conn, |connection| {
            let mut edges = Vec::new();

            // Get outgoing edges
            let mut stmt = connection.prepare(
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
            let mut stmt = connection.prepare(
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
        }, 3)
    }
    
    pub fn get_symbol_by_fqn(&self, fqn: &str) -> Result<Option<SymbolIR>> {
        execute_with_retry(&self.conn, |connection| {
            let symbol = connection.query_row(
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
                        lang_version: None,
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
        }, 3)
    }

    /// Search symbols using FTS5 full-text search for fast fuzzy matching
    pub fn search_symbols_fts(&self, query: &str, limit: usize) -> Result<Vec<SymbolIR>> {
        execute_with_retry(&self.conn, |connection| {
            let mut symbols = Vec::new();

            // Use FTS5 MATCH for fast full-text searching with ranking
            let mut stmt = connection.prepare(
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

            // For FTS5, only append * if the query doesn't already contain FTS5 operators
            let fts_query = if query.contains('*') || query.contains('"') || query.contains(" OR ") || query.contains(" AND ") {
                // Query already contains FTS5 operators, use as-is
                query.to_string()
            } else {
                // Simple query, append * for prefix matching
                format!("{}*", query)
            };
            let symbol_iter = stmt.query_map(params![fts_query, limit], |row| {
                Ok(SymbolIR {
                    id: row.get(0)?,
                    lang: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(Language::Unknown),
                    lang_version: None,
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
        }, 3)
    }
    
    pub fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<SymbolIR>> {
        // Try FTS5 first for better performance
        if let Ok(results) = self.search_symbols_fts(query, limit) {
            if !results.is_empty() {
                return Ok(results);
            }
        }

        execute_with_retry(&self.conn, |connection| {
            let mut symbols = Vec::new();

            // Fall back to LIKE search
            let pattern = format!("%{}%", query);

            let mut stmt = connection.prepare(
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
                    lang_version: None,
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
        }, 3)
    }
    
    pub fn get_symbols_in_file(&self, file_path: &str) -> Result<Vec<SymbolIR>> {
        execute_with_retry(&self.conn, |connection| {
            let mut symbols = Vec::new();

            let mut stmt = connection.prepare(
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
                    lang_version: None,
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
        }, 3)
    }
    
    pub fn get_symbol_count(&self) -> Result<usize> {
        execute_with_retry(&self.conn, |connection| {
            let count = connection.query_row(
                "SELECT COUNT(*) FROM symbol",
                [],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count as usize)
        }, 3)
    }

    /// Get symbol count for a specific commit
    pub fn get_symbol_count_for_commit(&self, commit_id: i64) -> Result<usize> {
        execute_with_retry(&self.conn, |connection| {
            let count = connection.query_row(
                "SELECT COUNT(*) FROM symbol WHERE commit_id = ?1",
                params![commit_id],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count as usize)
        }, 3)
    }

    pub fn get_edge_count(&self) -> Result<usize> {
        execute_with_retry(&self.conn, |connection| {
            let count = connection.query_row(
                "SELECT COUNT(*) FROM edge",
                [],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count as usize)
        }, 3)
    }

    /// Get edge count for a specific commit
    pub fn get_edge_count_for_commit(&self, commit_id: i64) -> Result<usize> {
        execute_with_retry(&self.conn, |connection| {
            let count = connection.query_row(
                "SELECT COUNT(*) FROM edge WHERE commit_id = ?1",
                params![commit_id],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count as usize)
        }, 3)
    }

    pub fn get_file_count(&self) -> Result<usize> {
        execute_with_retry(&self.conn, |connection| {
            let count = connection.query_row(
                "SELECT COUNT(DISTINCT path) FROM file",
                [],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count as usize)
        }, 3)
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
        execute_with_retry(&self.conn, |connection| {
            // Find files that import/depend on this file
            let mut stmt = connection.prepare(
                "SELECT DISTINCT file_src FROM edge
                 WHERE file_dst = ?1 AND file_src IS NOT NULL
                 AND edge_type IN ('\"Imports\"', '\"Reads\"', '\"Calls\"', '\"Contains\"', '\"Implements\"')"
            )?;

            let dependents = stmt.query_map([file_path], |row| {
                row.get::<_, String>(0)
            })?
            .filter_map(Result::ok)
            .collect();

            Ok(dependents)
        }, 3)
    }
}

