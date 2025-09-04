"""
Core CodeGraph class for querying the unified code graph
"""

import sqlite3
import subprocess
import json
from pathlib import Path
from typing import List, Optional, Dict, Set, Any
from functools import lru_cache

from .models import (
    Symbol, SymbolKind, Location, CallPath, 
    DependencyGraph, EdgeType, AnalysisQuality
)


class CodeGraph:
    """
    Main interface for querying the code graph.
    Integrates multiple analyzers and provides unified access.
    """
    
    def __init__(self, repo_path: str, db_path: Optional[str] = None):
        """
        Initialize the code graph for a repository.
        
        Args:
            repo_path: Path to the repository root
            db_path: Path to the graph database (default: .reviewbot/graph.db)
        """
        self.repo_path = Path(repo_path)
        if db_path is None:
            db_path = self.repo_path / ".reviewbot" / "graph.db"
        self.db_path = Path(db_path)
        
        # Initialize connections
        self._init_database()
        self._init_cache()
        
    def _init_database(self):
        """Initialize database connection"""
        if self.db_path.exists():
            self.conn = sqlite3.connect(self.db_path)
            self.conn.row_factory = sqlite3.Row
        else:
            # Run consilium scan if database doesn't exist
            self._run_initial_scan()
    
    def _init_cache(self):
        """Initialize caching layer"""
        self._symbol_cache = {}
        self._callgraph_cache = {}
        
    def _run_initial_scan(self):
        """Run consilium scan to build initial graph"""
        print(f"Building code graph for {self.repo_path}...")
        result = subprocess.run(
            ["cargo", "run", "--", "--repo", str(self.repo_path), "scan"],
            cwd=Path(__file__).parent.parent / "crates" / "core",
            capture_output=True,
            text=True
        )
        if result.returncode != 0:
            raise RuntimeError(f"Failed to build code graph: {result.stderr}")
        
        # Reconnect to newly created database
        self.conn = sqlite3.connect(self.db_path)
        self.conn.row_factory = sqlite3.Row
    
    # ========== Symbol Lookups ==========
    
    def get_symbol(self, fqn: str) -> Optional[Symbol]:
        """
        Get symbol details by fully qualified name.
        
        Args:
            fqn: Fully qualified name (e.g., "MyClass::myMethod")
            
        Returns:
            Symbol object or None if not found
        """
        if fqn in self._symbol_cache:
            return self._symbol_cache[fqn]
        
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT s.*, f.path 
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE s.fqn = ?
        """, (fqn,))
        
        row = cursor.fetchone()
        if row:
            symbol = self._row_to_symbol(row)
            self._symbol_cache[fqn] = symbol
            return symbol
        return None
    
    def find_symbols(self, 
                    pattern: str, 
                    kind: Optional[SymbolKind] = None,
                    limit: int = 100) -> List[Symbol]:
        """
        Search symbols by pattern and optional type filter.
        
        Args:
            pattern: Search pattern (supports SQL wildcards)
            kind: Optional symbol type filter
            limit: Maximum results to return
            
        Returns:
            List of matching symbols
        """
        query = """
            SELECT s.*, f.path 
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE s.name LIKE ?
        """
        params = [f"%{pattern}%"]
        
        if kind:
            query += " AND s.kind = ?"
            params.append(kind.value)
        
        query += f" LIMIT {limit}"
        
        cursor = self.conn.cursor()
        cursor.execute(query, params)
        
        return [self._row_to_symbol(row) for row in cursor.fetchall()]
    
    def get_file_symbols(self, filepath: str) -> List[Symbol]:
        """
        Get all symbols defined in a file.
        
        Args:
            filepath: Path to the file (relative to repo root)
            
        Returns:
            List of symbols in the file
        """
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT s.*, f.path 
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE f.path = ?
            ORDER BY s.line
        """, (filepath,))
        
        return [self._row_to_symbol(row) for row in cursor.fetchall()]
    
    # ========== Relationship Traversal ==========
    
    def get_callers(self, symbol: str, max_depth: int = 1) -> List[CallPath]:
        """
        Find all functions that call this symbol.
        
        Args:
            symbol: FQN of the symbol
            max_depth: Maximum call chain depth to explore
            
        Returns:
            List of call paths leading to this symbol
        """
        return self._traverse_calls(symbol, direction="callers", max_depth=max_depth)
    
    def get_callees(self, symbol: str, max_depth: int = 1) -> List[CallPath]:
        """
        Find all functions called by this symbol.
        
        Args:
            symbol: FQN of the symbol
            max_depth: Maximum call chain depth to explore
            
        Returns:
            List of call paths from this symbol
        """
        return self._traverse_calls(symbol, direction="callees", max_depth=max_depth)
    
    def get_dependencies(self, symbol: str) -> DependencyGraph:
        """
        Get all dependencies of a symbol.
        
        Args:
            symbol: FQN of the symbol
            
        Returns:
            DependencyGraph showing all dependencies
        """
        dependencies = {}
        dependents = {}
        
        # Get direct dependencies
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT DISTINCT e.dst 
            FROM edges e
            WHERE e.src = ? AND e.edge_type IN ('calls', 'imports', 'uses')
        """, (symbol,))
        
        dependencies[symbol] = [row["dst"] for row in cursor.fetchall()]
        
        # Get dependents
        cursor.execute("""
            SELECT DISTINCT e.src 
            FROM edges e
            WHERE e.dst = ? AND e.edge_type IN ('calls', 'imports', 'uses')
        """, (symbol,))
        
        dependents[symbol] = [row["src"] for row in cursor.fetchall()]
        
        # Check for cycles
        cycles = self._find_cycles(symbol, dependencies)
        
        return DependencyGraph(
            root=self.get_symbol(symbol),
            dependencies=dependencies,
            dependents=dependents,
            cycles=cycles
        )
    
    def find_path(self, from_symbol: str, to_symbol: str, 
                  max_depth: int = 10) -> List[List[Symbol]]:
        """
        Find execution paths between two symbols.
        
        Args:
            from_symbol: Starting symbol FQN
            to_symbol: Target symbol FQN
            max_depth: Maximum path length
            
        Returns:
            List of possible paths (each path is a list of symbols)
        """
        paths = []
        visited = set()
        
        def dfs(current: str, target: str, path: List[str], depth: int):
            if depth > max_depth:
                return
            
            if current == target:
                # Found a path, convert to symbols
                symbol_path = [self.get_symbol(fqn) for fqn in path]
                paths.append(symbol_path)
                return
            
            visited.add(current)
            
            # Get next symbols
            cursor = self.conn.cursor()
            cursor.execute("""
                SELECT DISTINCT dst 
                FROM edges
                WHERE src = ? AND edge_type = 'calls'
            """, (current,))
            
            for row in cursor.fetchall():
                next_sym = row["dst"]
                if next_sym not in visited:
                    dfs(next_sym, target, path + [next_sym], depth + 1)
            
            visited.remove(current)
        
        dfs(from_symbol, to_symbol, [from_symbol], 0)
        return paths
    
    # ========== Graph Statistics ==========
    
    def get_statistics(self) -> Dict[str, Any]:
        """Get overall graph statistics"""
        cursor = self.conn.cursor()
        
        stats = {}
        
        # Symbol counts by type
        cursor.execute("""
            SELECT kind, COUNT(*) as count
            FROM symbols
            GROUP BY kind
        """)
        stats["symbols_by_kind"] = dict(cursor.fetchall())
        
        # Edge counts by type
        cursor.execute("""
            SELECT edge_type, COUNT(*) as count
            FROM edges
            GROUP BY edge_type
        """)
        stats["edges_by_type"] = dict(cursor.fetchall())
        
        # File statistics
        cursor.execute("SELECT COUNT(*) FROM files")
        stats["total_files"] = cursor.fetchone()[0]
        
        cursor.execute("SELECT COUNT(*) FROM symbols")
        stats["total_symbols"] = cursor.fetchone()[0]
        
        cursor.execute("SELECT COUNT(*) FROM edges")
        stats["total_edges"] = cursor.fetchone()[0]
        
        return stats
    
    # ========== Helper Methods ==========
    
    def _row_to_symbol(self, row: sqlite3.Row) -> Symbol:
        """Convert database row to Symbol object"""
        return Symbol(
            fqn=row["fqn"],
            name=row["name"],
            kind=SymbolKind(row["kind"]),
            location=Location(
                file=row["path"],
                line=row["line"],
                column=row.get("column")
            ),
            signature=row.get("signature"),
            docstring=row.get("docstring"),
            analyzer="consilium",
            confidence=1.0
        )
    
    def _traverse_calls(self, symbol: str, direction: str, 
                       max_depth: int) -> List[CallPath]:
        """Traverse call graph in either direction"""
        paths = []
        
        def traverse(current: str, path: List[str], depth: int):
            if depth >= max_depth:
                return
            
            cursor = self.conn.cursor()
            if direction == "callers":
                cursor.execute("""
                    SELECT DISTINCT src FROM edges
                    WHERE dst = ? AND edge_type = 'calls'
                """, (current,))
                next_symbols = [row["src"] for row in cursor.fetchall()]
            else:  # callees
                cursor.execute("""
                    SELECT DISTINCT dst FROM edges
                    WHERE src = ? AND edge_type = 'calls'
                """, (current,))
                next_symbols = [row["dst"] for row in cursor.fetchall()]
            
            for next_sym in next_symbols:
                new_path = path + [next_sym]
                
                # Check for recursion
                is_recursive = next_sym in path
                
                # Convert to symbols and add to results
                symbol_path = [self.get_symbol(fqn) for fqn in new_path]
                paths.append(CallPath(
                    path=symbol_path,
                    depth=len(new_path),
                    is_recursive=is_recursive
                ))
                
                # Continue traversing if not recursive
                if not is_recursive:
                    traverse(next_sym, new_path, depth + 1)
        
        traverse(symbol, [symbol], 0)
        return paths
    
    def _find_cycles(self, start: str, graph: Dict[str, List[str]]) -> List[List[str]]:
        """Find cycles in dependency graph using DFS"""
        cycles = []
        visited = set()
        rec_stack = []
        
        def dfs(node: str):
            visited.add(node)
            rec_stack.append(node)
            
            for neighbor in graph.get(node, []):
                if neighbor not in visited:
                    dfs(neighbor)
                elif neighbor in rec_stack:
                    # Found cycle
                    cycle_start = rec_stack.index(neighbor)
                    cycles.append(rec_stack[cycle_start:])
            
            rec_stack.pop()
        
        dfs(start)
        return cycles
    
    def refresh_cache(self):
        """Clear all caches"""
        self._symbol_cache.clear()
        self._callgraph_cache.clear()
        
    def close(self):
        """Close database connection"""
        if hasattr(self, 'conn'):
            self.conn.close()