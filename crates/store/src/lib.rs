use anyhow::Result;
use protocol::{EdgeIR, OccurrenceIR, Span, SymbolIR};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub struct GraphStore {
    conn: Connection,
    db_path: PathBuf,
}

impl GraphStore {
    pub fn new(repo_root: &Path) -> Result<Self> {
        let db_dir = repo_root.join(".reviewbot");
        std::fs::create_dir_all(&db_dir)?;
        
        let db_path = db_dir.join("graph.db");
        let conn = Connection::open(&db_path)?;
        
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", 10000)?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        
        let mut store = Self { conn, db_path };
        store.init_schema()?;
        
        Ok(store)
    }
    
    fn init_schema(&mut self) -> Result<()> {
        self.conn.execute_batch(
            r#"BEGIN;
            CREATE TABLE IF NOT EXISTS commit_snapshot (
                id INTEGER PRIMARY KEY,
                sha TEXT NOT NULL UNIQUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                files_indexed INTEGER DEFAULT 0,
                symbols_found INTEGER DEFAULT 0
            );
            
            CREATE TABLE IF NOT EXISTS file (
                id INTEGER PRIMARY KEY,
                commit_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                language TEXT,
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
                meta TEXT,
                provenance TEXT,
                FOREIGN KEY (commit_id) REFERENCES commit_snapshot(id),
                UNIQUE(commit_id, edge_type, src_symbol, dst_symbol, file_src, file_dst)
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
            COMMIT;
            "#,
        )?;
        
        info!("Database schema initialized at {:?}", self.db_path);
        Ok(())
    }
    
    pub fn create_commit_snapshot(&self, sha: &str) -> Result<i64> {
        let existing: Option<i64> = self.conn
            .query_row(
                "SELECT id FROM commit_snapshot WHERE sha = ?1",
                params![sha],
                |row| row.get(0),
            )
            .optional()?;
        
        if let Some(id) = existing {
            debug!("Using existing commit snapshot {}", sha);
            return Ok(id);
        }
        
        self.conn.execute(
            "INSERT INTO commit_snapshot (sha) VALUES (?1)",
            params![sha],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    pub fn get_last_scanned_commit(&self) -> Result<Option<String>> {
        let result = self.conn
            .query_row(
                "SELECT sha FROM commit_snapshot ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result)
    }
    
    pub fn get_file_dependents(&self, file_path: &str) -> Result<Vec<String>> {
        let mut dependents = Vec::new();
        
        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT file_src 
            FROM edge 
            WHERE file_dst = ?1 AND edge_type = 'Imports'
            "#,
        )?;
        
        let rows = stmt.query_map(params![file_path], |row| row.get(0))?;
        
        for row in rows {
            dependents.push(row?);
        }
        
        Ok(dependents)
    }
    
    pub fn delete_file_data(&self, commit_id: i64, file_path: &str) -> Result<()> {
        // Delete symbols for this file
        self.conn.execute(
            "DELETE FROM symbol WHERE commit_id = ?1 AND file_path = ?2",
            params![commit_id, file_path],
        )?;
        
        // Delete edges originating from this file
        self.conn.execute(
            "DELETE FROM edge WHERE commit_id = ?1 AND (file_src = ?2 OR file_dst = ?2)",
            params![commit_id, file_path],
        )?;
        
        // Delete occurrences in this file
        self.conn.execute(
            "DELETE FROM occurrence WHERE commit_id = ?1 AND file_path = ?2",
            params![commit_id, file_path],
        )?;
        
        // Delete file record
        self.conn.execute(
            "DELETE FROM file WHERE commit_id = ?1 AND path = ?2",
            params![commit_id, file_path],
        )?;
        
        Ok(())
    }
    
    pub fn insert_file(&self, commit_id: i64, path: &str, content_hash: String, size: usize) -> Result<()> {
        // Detect language from extension
        let language = if path.ends_with(".ts") || path.ends_with(".tsx") {
            "TypeScript"
        } else if path.ends_with(".js") || path.ends_with(".jsx") || path.ends_with(".mjs") {
            "JavaScript"
        } else if path.ends_with(".py") || path.ends_with(".pyi") {
            "Python"
        } else if path.ends_with(".go") {
            "Go"
        } else {
            "Unknown"
        };
        
        self.conn.execute(
            "INSERT OR REPLACE INTO file (commit_id, path, language, content_hash, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![commit_id, path, language, content_hash, size as i64],
        )?;
        
        Ok(())
    }
    
    pub fn insert_symbol(&self, commit_id: i64, symbol: &SymbolIR) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO symbol (
                commit_id, symbol_id, lang, kind, name, fqn, signature,
                file_path, span_start_line, span_start_col, span_end_line, 
                span_end_col, visibility, doc, sig_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                commit_id,
                symbol.id,
                format!("{:?}", symbol.lang),
                format!("{:?}", symbol.kind),
                symbol.name,
                symbol.fqn,
                symbol.signature,
                symbol.file_path,
                symbol.span.start_line,
                symbol.span.start_col,
                symbol.span.end_line,
                symbol.span.end_col,
                symbol.visibility,
                symbol.doc,
                symbol.sig_hash,
            ],
        )?;
        Ok(())
    }
    
    pub fn insert_edge(&self, commit_id: i64, edge: &EdgeIR) -> Result<()> {
        let meta_json = serde_json::to_string(&edge.meta)?;
        let provenance_json = serde_json::to_string(&edge.provenance)?;
        
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO edge (
                commit_id, edge_type, src_symbol, dst_symbol, 
                file_src, file_dst, resolution, meta, provenance
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                commit_id,
                format!("{:?}", edge.edge_type),
                edge.src,
                edge.dst,
                edge.file_src,
                edge.file_dst,
                format!("{:?}", edge.resolution),
                meta_json,
                provenance_json,
            ],
        )?;
        Ok(())
    }
    
    pub fn insert_occurrence(&self, commit_id: i64, occurrence: &OccurrenceIR) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO occurrence (
                commit_id, file_path, symbol_id, role,
                span_start_line, span_start_col, span_end_line, span_end_col, token
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                commit_id,
                occurrence.file_path,
                occurrence.symbol_id,
                format!("{:?}", occurrence.role),
                occurrence.span.start_line,
                occurrence.span.start_col,
                occurrence.span.end_line,
                occurrence.span.end_col,
                occurrence.token,
            ],
        )?;
        Ok(())
    }
    
    pub fn get_mutation_count(&self) -> Result<usize> {
        Ok(self.conn.changes() as usize)
    }
    
    pub fn find_symbol_by_fqn(&self, fqn: &str) -> Result<Option<SymbolIR>> {
        let row = self.conn.query_row(
            r#"
            SELECT symbol_id, lang, kind, name, fqn, signature, file_path,
                   span_start_line, span_start_col, span_end_line, span_end_col,
                   visibility, doc, sig_hash
            FROM symbol 
            WHERE fqn = ?1
            ORDER BY commit_id DESC
            LIMIT 1
            "#,
            params![fqn],
            |row| {
                Ok(SymbolIR {
                    id: row.get(0)?,
                    lang: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(1)?)).unwrap(),
                    kind: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(2)?)).unwrap(),
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
                    visibility: row.get(11)?,
                    doc: row.get(12)?,
                    sig_hash: row.get(13)?,
                })
            },
        ).optional()?;
        
        Ok(row)
    }
    
    pub fn get_callers(&self, symbol_id: &str, depth: usize) -> Result<Vec<SymbolIR>> {
        let mut symbols = Vec::new();
        
        if depth > 0 {
            let mut stmt = self.conn.prepare(
                r#"
                WITH RECURSIVE callers AS (
                    SELECT DISTINCT s.*, 0 as depth
                    FROM edge e
                    JOIN symbol s ON s.symbol_id = e.src_symbol
                    WHERE e.dst_symbol = ?1 AND e.edge_type = 'Calls'
                    
                    UNION
                    
                    SELECT DISTINCT s.*, c.depth + 1
                    FROM edge e
                    JOIN symbol s ON s.symbol_id = e.src_symbol
                    JOIN callers c ON c.symbol_id = e.dst_symbol
                    WHERE e.edge_type = 'Calls' AND c.depth < ?2 - 1
                )
                SELECT DISTINCT symbol_id, lang, kind, name, fqn, signature, file_path,
                       span_start_line, span_start_col, span_end_line, span_end_col,
                       visibility, doc, sig_hash
                FROM callers
                "#,
            )?;
            
            let symbol_iter = stmt.query_map(params![symbol_id, depth], |row| {
                Ok(SymbolIR {
                    id: row.get(0)?,
                    lang: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(1)?)).unwrap(),
                    kind: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(2)?)).unwrap(),
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
                    visibility: row.get(11)?,
                    doc: row.get(12)?,
                    sig_hash: row.get(13)?,
                })
            })?;
            
            for symbol in symbol_iter {
                symbols.push(symbol?);
            }
        }
        
        Ok(symbols)
    }
    
    pub fn get_callees(&self, symbol_id: &str, depth: usize) -> Result<Vec<SymbolIR>> {
        let mut symbols = Vec::new();
        
        if depth > 0 {
            let mut stmt = self.conn.prepare(
                r#"
                WITH RECURSIVE callees AS (
                    SELECT DISTINCT s.*, 0 as depth
                    FROM edge e
                    JOIN symbol s ON s.symbol_id = e.dst_symbol
                    WHERE e.src_symbol = ?1 AND e.edge_type = 'Calls'
                    
                    UNION
                    
                    SELECT DISTINCT s.*, c.depth + 1
                    FROM edge e
                    JOIN symbol s ON s.symbol_id = e.dst_symbol
                    JOIN callees c ON c.symbol_id = e.src_symbol
                    WHERE e.edge_type = 'Calls' AND c.depth < ?2 - 1
                )
                SELECT DISTINCT symbol_id, lang, kind, name, fqn, signature, file_path,
                       span_start_line, span_start_col, span_end_line, span_end_col,
                       visibility, doc, sig_hash
                FROM callees
                "#,
            )?;
            
            let symbol_iter = stmt.query_map(params![symbol_id, depth], |row| {
                Ok(SymbolIR {
                    id: row.get(0)?,
                    lang: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(1)?)).unwrap(),
                    kind: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(2)?)).unwrap(),
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
                    visibility: row.get(11)?,
                    doc: row.get(12)?,
                    sig_hash: row.get(13)?,
                })
            })?;
            
            for symbol in symbol_iter {
                symbols.push(symbol?);
            }
        }
        
        Ok(symbols)
    }
    
    pub fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<SymbolIR>> {
        let mut symbols = Vec::new();
        
        // Simple LIKE search for now (FTS5 will come later)
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
                lang: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(1)?)).unwrap(),
                kind: serde_json::from_str(&format!(r#""{}""#, row.get::<_, String>(2)?)).unwrap(),
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
                visibility: row.get(11)?,
                doc: row.get(12)?,
                sig_hash: row.get(13)?,
            })
        })?;
        
        for symbol in symbol_iter {
            symbols.push(symbol?);
        }
        
        Ok(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_create_database() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let store = GraphStore::new(temp_dir.path())?;
        
        let tables_count: i32 = store.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0),
        )?;
        
        assert!(tables_count >= 5);
        Ok(())
    }
    
    #[test]
    fn test_commit_snapshot() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let store = GraphStore::new(temp_dir.path())?;
        
        let commit_id = store.create_commit_snapshot("abc123")?;
        assert!(commit_id > 0);
        
        let commit_id2 = store.create_commit_snapshot("abc123")?;
        assert_eq!(commit_id, commit_id2);
        
        Ok(())
    }
    
    #[test]
    fn test_insert_symbol() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let store = GraphStore::new(temp_dir.path())?;
        let commit_id = store.create_commit_snapshot("test123")?;
        
        let symbol = SymbolIR {
            id: "test_symbol".to_string(),
            lang: Language::TypeScript,
            kind: SymbolKind::Function,
            name: "testFunc".to_string(),
            fqn: "module.testFunc".to_string(),
            signature: Some("() => void".to_string()),
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: 1,
                start_col: 0,
                end_line: 3,
                end_col: 1,
            },
            visibility: Some("public".to_string()),
            doc: None,
            sig_hash: "hash123".to_string(),
        };
        
        store.insert_symbol(commit_id, &symbol)?;
        
        let count: i32 = store.conn.query_row(
            "SELECT COUNT(*) FROM symbol WHERE fqn = ?1",
            params!["module.testFunc"],
            |row| row.get(0),
        )?;
        
        assert_eq!(count, 1);
        Ok(())
    }
    
    #[test]
    fn test_find_symbol_by_fqn() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let store = GraphStore::new(temp_dir.path())?;
        let commit_id = store.create_commit_snapshot("test123")?;
        
        let symbol = SymbolIR {
            id: "test_id".to_string(),
            lang: Language::TypeScript,
            kind: SymbolKind::Class,
            name: "MyClass".to_string(),
            fqn: "module.MyClass".to_string(),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span {
                start_line: 5,
                start_col: 0,
                end_line: 10,
                end_col: 1,
            },
            visibility: Some("public".to_string()),
            doc: Some("Test class".to_string()),
            sig_hash: "hash456".to_string(),
        };
        
        store.insert_symbol(commit_id, &symbol)?;
        
        // Test finding existing symbol
        let found = store.find_symbol_by_fqn("module.MyClass")?;
        assert!(found.is_some());
        
        let found_symbol = found.unwrap();
        assert_eq!(found_symbol.name, "MyClass");
        assert_eq!(found_symbol.kind, SymbolKind::Class);
        assert_eq!(found_symbol.fqn, "module.MyClass");
        
        // Test finding non-existent symbol
        let not_found = store.find_symbol_by_fqn("module.NotExists")?;
        assert!(not_found.is_none());
        
        Ok(())
    }
    
    #[test]
    fn test_search_symbols() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let store = GraphStore::new(temp_dir.path())?;
        let commit_id = store.create_commit_snapshot("test123")?;
        
        // Insert test symbols
        let symbols = vec![
            ("Calculator", "math.Calculator"),
            ("calculate", "utils.calculate"),
            ("CalcHelper", "helpers.CalcHelper"),
            ("sum", "math.sum"),
            ("product", "math.product"),
        ];
        
        for (name, fqn) in symbols {
            let symbol = SymbolIR {
                id: format!("id_{}", name),
                lang: Language::TypeScript,
                kind: SymbolKind::Function,
                name: name.to_string(),
                fqn: fqn.to_string(),
                signature: None,
                file_path: "test.ts".to_string(),
                span: Span {
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                },
                visibility: None,
                doc: None,
                sig_hash: format!("hash_{}", name),
            };
            store.insert_symbol(commit_id, &symbol)?;
        }
        
        // Test search with prefix
        let results = store.search_symbols("Calc", 10)?;
        assert_eq!(results.len(), 3, "Should find 3 symbols starting with 'Calc'");
        
        // Test exact match ranking
        let results = store.search_symbols("sum", 10)?;
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "sum", "Exact match should be first");
        
        // Test limit
        let results = store.search_symbols("", 2)?; // Match all but limit to 2
        assert_eq!(results.len(), 2, "Should respect limit");
        
        Ok(())
    }
    
    #[test]
    fn test_get_callers_and_callees() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let store = GraphStore::new(temp_dir.path())?;
        let commit_id = store.create_commit_snapshot("test123")?;
        
        // Create a simple call graph: main -> helper -> util
        let main_sym = SymbolIR {
            id: "sym_main".to_string(),
            lang: Language::TypeScript,
            kind: SymbolKind::Function,
            name: "main".to_string(),
            fqn: "app.main".to_string(),
            signature: None,
            file_path: "main.ts".to_string(),
            span: Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "hash_main".to_string(),
        };
        
        let helper_sym = SymbolIR {
            id: "sym_helper".to_string(),
            lang: Language::TypeScript,
            kind: SymbolKind::Function,
            name: "helper".to_string(),
            fqn: "app.helper".to_string(),
            signature: None,
            file_path: "helper.ts".to_string(),
            span: Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "hash_helper".to_string(),
        };
        
        let util_sym = SymbolIR {
            id: "sym_util".to_string(),
            lang: Language::TypeScript,
            kind: SymbolKind::Function,
            name: "util".to_string(),
            fqn: "app.util".to_string(),
            signature: None,
            file_path: "util.ts".to_string(),
            span: Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "hash_util".to_string(),
        };
        
        store.insert_symbol(commit_id, &main_sym)?;
        store.insert_symbol(commit_id, &helper_sym)?;
        store.insert_symbol(commit_id, &util_sym)?;
        
        // Create edges: main -> helper -> util
        let edge1 = EdgeIR {
            edge_type: EdgeType::Calls,
            src: Some("sym_main".to_string()),
            dst: Some("sym_helper".to_string()),
            file_src: None,
            file_dst: None,
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        };
        
        let edge2 = EdgeIR {
            edge_type: EdgeType::Calls,
            src: Some("sym_helper".to_string()),
            dst: Some("sym_util".to_string()),
            file_src: None,
            file_dst: None,
            resolution: Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        };
        
        store.insert_edge(commit_id, &edge1)?;
        store.insert_edge(commit_id, &edge2)?;
        
        // Test get_callers
        let callers = store.get_callers("sym_helper", 1)?;
        assert_eq!(callers.len(), 1, "helper should have 1 direct caller");
        assert_eq!(callers[0].name, "main");
        
        // Test get_callees
        let callees = store.get_callees("sym_helper", 1)?;
        assert_eq!(callees.len(), 1, "helper should have 1 direct callee");
        assert_eq!(callees[0].name, "util");
        
        // Test recursive with depth 2
        let callees = store.get_callees("sym_main", 2)?;
        assert_eq!(callees.len(), 2, "main should have 2 callees at depth 2");
        
        Ok(())
    }
}