"""
Simplified Code Graph API for agents - focused on database queries
"""

import sqlite3
from pathlib import Path
from typing import List, Optional, Dict, Any, Set
from dataclasses import dataclass
from functools import lru_cache


@dataclass
class Symbol:
    """A symbol in the code graph"""
    fqn: str
    name: str
    kind: str
    file: str
    line: int
    signature: Optional[str] = None


@dataclass
class Edge:
    """An edge in the code graph"""
    source: str
    target: str
    edge_type: str


class CodeGraphAPI:
    """
    Simple API for querying the code graph database.
    Designed to run on the same server as agents - no auth needed.
    """
    
    def __init__(self, repo_path: str, db_path: Optional[str] = None, 
                 check_same_thread: bool = True, timeout: float = 10.0):
        """
        Initialize the API for a repository.
        
        Args:
            repo_path: Path to the repository root
            db_path: Path to the graph database (default: .reviewbot/graph.db)
            check_same_thread: If False, allows multi-threaded access (default: True)
            timeout: Database lock timeout in seconds (default: 10.0)
        """
        self.repo_path = Path(repo_path)
        if db_path is None:
            db_path = self.repo_path / ".reviewbot" / "graph.db"
        self.db_path = Path(db_path)
        
        if not self.db_path.exists():
            raise FileNotFoundError(f"Database not found at {self.db_path}. Run 'reviewbot scan' first.")
        
        # Support concurrent access with proper timeout
        self.conn = sqlite3.connect(
            self.db_path, 
            check_same_thread=check_same_thread,
            timeout=timeout
        )
        self.conn.row_factory = sqlite3.Row
        # Enable WAL mode for better concurrency
        self.conn.execute("PRAGMA journal_mode=WAL")
        self.conn.execute("PRAGMA foreign_keys=ON")
    
    # ========== Core Queries ==========
    
    def get_symbol(self, fqn: str) -> Optional[Symbol]:
        """Get a symbol by its fully qualified name."""
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT s.fqn, s.name, s.kind, f.path, s.line, s.signature
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE s.fqn = ?
        """, (fqn,))
        
        row = cursor.fetchone()
        if row:
            return Symbol(
                fqn=row["fqn"],
                name=row["name"],
                kind=row["kind"],
                file=row["path"],
                line=row["line"],
                signature=row["signature"] if "signature" in row.keys() else None
            )
        return None
    
    def find_symbols(self, pattern: str, kind: Optional[str] = None) -> List[Symbol]:
        """Search for symbols by name pattern."""
        query = """
            SELECT s.fqn, s.name, s.kind, f.path, s.line, s.signature
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE s.name LIKE ?
        """
        params = [f"%{pattern}%"]
        
        if kind:
            query += " AND s.kind = ?"
            params.append(kind)
        
        cursor = self.conn.cursor()
        cursor.execute(query, params)
        
        symbols = []
        for row in cursor.fetchall():
            symbols.append(Symbol(
                fqn=row["fqn"],
                name=row["name"],
                kind=row["kind"],
                file=row["path"],
                line=row["line"],
                signature=row["signature"] if "signature" in row.keys() else None
            ))
        
        return symbols
    
    def get_file_symbols(self, file_path: str) -> List[Symbol]:
        """Get all symbols in a file."""
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT s.fqn, s.name, s.kind, f.path, s.line, s.signature
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE f.path = ?
            ORDER BY s.line
        """, (file_path,))
        
        symbols = []
        for row in cursor.fetchall():
            symbols.append(Symbol(
                fqn=row["fqn"],
                name=row["name"],
                kind=row["kind"],
                file=row["path"],
                line=row["line"],
                signature=row["signature"] if "signature" in row.keys() else None
            ))
        
        return symbols
    
    # ========== Relationship Queries ==========
    
    def get_callers(self, symbol: str) -> List[str]:
        """Get all functions that call this symbol."""
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT DISTINCT src
            FROM edges
            WHERE dst = ? AND edge_type = 'calls'
        """, (symbol,))
        
        return [row["src"] for row in cursor.fetchall()]
    
    def get_callees(self, symbol: str) -> List[str]:
        """Get all functions called by this symbol."""
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT DISTINCT dst
            FROM edges
            WHERE src = ? AND edge_type = 'calls'
        """, (symbol,))
        
        return [row["dst"] for row in cursor.fetchall()]
    
    def get_edges(self, source: Optional[str] = None, target: Optional[str] = None, 
                  edge_type: Optional[str] = None) -> List[Edge]:
        """Get edges with optional filters."""
        query = "SELECT src, dst, edge_type FROM edges WHERE 1=1"
        params = []
        
        if source:
            query += " AND src = ?"
            params.append(source)
        
        if target:
            query += " AND dst = ?"
            params.append(target)
        
        if edge_type:
            query += " AND edge_type = ?"
            params.append(edge_type)
        
        cursor = self.conn.cursor()
        cursor.execute(query, params)
        
        edges = []
        for row in cursor.fetchall():
            edges.append(Edge(
                source=row["src"],
                target=row["dst"],
                edge_type=row["edge_type"]
            ))
        
        return edges
    
    # ========== Analysis Queries ==========
    
    def find_paths(self, start: str, end: str, max_depth: int = 5) -> List[List[str]]:
        """Find all paths between two symbols."""
        paths = []
        visited = set()
        
        def dfs(current: str, target: str, path: List[str], depth: int):
            if depth > max_depth:
                return
            
            if current == target:
                paths.append(path[:])
                return
            
            visited.add(current)
            
            # Get next nodes
            callees = self.get_callees(current)
            for next_node in callees:
                if next_node not in visited:
                    path.append(next_node)
                    dfs(next_node, target, path, depth + 1)
                    path.pop()
            
            visited.remove(current)
        
        dfs(start, end, [start], 0)
        return paths
    
    def get_dependencies(self, symbol: str) -> Dict[str, List[str]]:
        """Get all dependencies of a symbol."""
        result = {
            "imports": [],
            "calls": [],
            "uses": []
        }
        
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT DISTINCT dst, edge_type
            FROM edges
            WHERE src = ?
        """, (symbol,))
        
        for row in cursor.fetchall():
            edge_type = row["edge_type"]
            if edge_type in result:
                result[edge_type].append(row["dst"])
            elif edge_type == "imports":
                result["imports"].append(row["dst"])
            elif edge_type == "calls":
                result["calls"].append(row["dst"])
            else:
                result.setdefault(edge_type, []).append(row["dst"])
        
        return result
    
    def get_impact_radius(self, symbol: str, max_depth: int = 3) -> Set[str]:
        """Get all symbols that would be affected if this symbol changes."""
        impacted = set()
        to_process = [(symbol, 0)]
        
        while to_process:
            current, depth = to_process.pop(0)
            
            if depth >= max_depth:
                continue
            
            callers = self.get_callers(current)
            for caller in callers:
                if caller not in impacted:
                    impacted.add(caller)
                    to_process.append((caller, depth + 1))
        
        return impacted
    
    # ========== Statistics ==========
    
    def get_stats(self) -> Dict[str, Any]:
        """Get overall statistics about the code graph."""
        cursor = self.conn.cursor()
        
        stats = {}
        
        # Count symbols by type
        cursor.execute("""
            SELECT kind, COUNT(*) as count
            FROM symbols
            GROUP BY kind
        """)
        stats["symbols_by_kind"] = dict(cursor.fetchall())
        
        # Count edges by type
        cursor.execute("""
            SELECT edge_type, COUNT(*) as count
            FROM edges
            GROUP BY edge_type
        """)
        stats["edges_by_type"] = dict(cursor.fetchall())
        
        # Total counts
        cursor.execute("SELECT COUNT(*) FROM files")
        stats["total_files"] = cursor.fetchone()[0]
        
        cursor.execute("SELECT COUNT(*) FROM symbols")
        stats["total_symbols"] = cursor.fetchone()[0]
        
        cursor.execute("SELECT COUNT(*) FROM edges")
        stats["total_edges"] = cursor.fetchone()[0]
        
        return stats
    
    def find_cycles(self) -> List[List[str]]:
        """Find all cycles in the call graph."""
        cycles = []
        visited = set()
        rec_stack = []
        
        # Get all symbols
        cursor = self.conn.cursor()
        cursor.execute("SELECT DISTINCT fqn FROM symbols")
        all_symbols = [row["fqn"] for row in cursor.fetchall()]
        
        def dfs(node: str):
            visited.add(node)
            rec_stack.append(node)
            
            callees = self.get_callees(node)
            for callee in callees:
                if callee not in visited:
                    dfs(callee)
                elif callee in rec_stack:
                    # Found a cycle
                    cycle_start = rec_stack.index(callee)
                    cycle = rec_stack[cycle_start:] + [callee]
                    if cycle not in cycles:
                        cycles.append(cycle)
            
            rec_stack.pop()
        
        for symbol in all_symbols:
            if symbol not in visited:
                dfs(symbol)
        
        return cycles
    
    def close(self):
        """Close the database connection."""
        if self.conn:
            self.conn.close()
            self.conn = None
    
    def __enter__(self):
        """Context manager entry."""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - ensures connection is closed."""
        self.close()
        return False
    
    def begin_transaction(self):
        """Begin an explicit transaction."""
        self.conn.execute("BEGIN TRANSACTION")
    
    def commit(self):
        """Commit the current transaction."""
        self.conn.commit()
    
    def rollback(self):
        """Rollback the current transaction."""
        self.conn.rollback()


# ========== Convenience Functions for Agents ==========

def analyze_codebase(repo_path: str) -> Dict[str, Any]:
    """
    Quick analysis of a codebase for agents.
    
    Returns:
        Dictionary with key metrics and insights
    """
    api = CodeGraphAPI(repo_path)
    
    stats = api.get_stats()
    cycles = api.find_cycles()
    
    # Find entry points (functions with few/no callers)
    entry_points = []
    for symbol in api.find_symbols("", kind="function")[:100]:  # Sample
        callers = api.get_callers(symbol.fqn)
        if len(callers) <= 1:
            entry_points.append(symbol.fqn)
    
    # Find complex functions (many callees)
    complex_functions = []
    for symbol in api.find_symbols("", kind="function")[:100]:  # Sample
        callees = api.get_callees(symbol.fqn)
        if len(callees) > 10:
            complex_functions.append({
                "function": symbol.fqn,
                "callees_count": len(callees)
            })
    
    api.close()
    
    return {
        "stats": stats,
        "cycles": cycles[:10],  # Limit to first 10
        "entry_points": entry_points[:20],  # Limit to first 20
        "complex_functions": sorted(complex_functions, 
                                  key=lambda x: x["callees_count"], 
                                  reverse=True)[:10]
    }


def find_related_code(repo_path: str, symbol: str) -> Dict[str, List[str]]:
    """
    Find all code related to a symbol.
    
    Returns:
        Dictionary with callers, callees, and dependencies
    """
    api = CodeGraphAPI(repo_path)
    
    result = {
        "symbol": symbol,
        "callers": api.get_callers(symbol),
        "callees": api.get_callees(symbol),
        "dependencies": api.get_dependencies(symbol),
        "impact": list(api.get_impact_radius(symbol))
    }
    
    api.close()
    
    return result


if __name__ == "__main__":
    # Example usage
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python simple_api.py <repo_path>")
        sys.exit(1)
    
    repo = sys.argv[1]
    
    # Analyze the codebase
    print("Analyzing codebase...")
    analysis = analyze_codebase(repo)
    
    print(f"\nCodebase Statistics:")
    print(f"  Files: {analysis['stats']['total_files']}")
    print(f"  Symbols: {analysis['stats']['total_symbols']}")
    print(f"  Edges: {analysis['stats']['total_edges']}")
    
    print(f"\nFound {len(analysis['cycles'])} cycles")
    print(f"Found {len(analysis['entry_points'])} entry points")
    
    if analysis['complex_functions']:
        print(f"\nMost complex functions:")
        for func in analysis['complex_functions'][:5]:
            print(f"  {func['function']}: {func['callees_count']} callees")