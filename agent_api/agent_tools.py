"""
Simple, flexible tools for code review agents.
No prescriptive logic - let the agents be intelligent.
"""

from typing import List, Dict, Any, Optional, Set
from pathlib import Path
import sqlite3


class CodeTools:
    """
    Minimal tools for agents to explore and understand code.
    The agent decides what's important, not us.
    """
    
    def __init__(self, repo_path: str, db_path: Optional[str] = None):
        self.repo_path = Path(repo_path)
        if db_path is None:
            db_path = self.repo_path / ".reviewbot" / "graph.db"
        self.db_path = Path(db_path)
        
        if not self.db_path.exists():
            raise FileNotFoundError(f"Database not found at {self.db_path}")
        
        self.conn = sqlite3.connect(self.db_path)
        self.conn.row_factory = sqlite3.Row
    
    # ========== Basic Queries - Let agent interpret ==========
    
    def query(self, sql: str, params: tuple = ()) -> List[Dict[str, Any]]:
        """
        Run any SQL query on the code graph database.
        Agent can craft queries based on what it needs.
        
        Tables available:
        - files: id, path, hash, language
        - symbols: id, file_id, fqn, name, kind, line, signature
        - edges: id, src, dst, edge_type
        """
        cursor = self.conn.cursor()
        cursor.execute(sql, params)
        return [dict(row) for row in cursor.fetchall()]
    
    def get_symbol(self, fqn: str) -> Optional[Dict[str, Any]]:
        """Get symbol information by fully qualified name."""
        result = self.query("""
            SELECT s.*, f.path as file_path
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE s.fqn = ?
        """, (fqn,))
        return result[0] if result else None
    
    def find_symbols(self, pattern: str = "", kind: Optional[str] = None) -> List[Dict[str, Any]]:
        """Find symbols matching a pattern."""
        if kind:
            return self.query("""
                SELECT s.*, f.path as file_path
                FROM symbols s
                JOIN files f ON s.file_id = f.id
                WHERE s.name LIKE ? AND s.kind = ?
            """, (f"%{pattern}%", kind))
        else:
            return self.query("""
                SELECT s.*, f.path as file_path
                FROM symbols s
                JOIN files f ON s.file_id = f.id
                WHERE s.name LIKE ?
            """, (f"%{pattern}%",))
    
    def get_relationships(self, symbol: str, direction: str = "both") -> Dict[str, List[str]]:
        """
        Get all relationships for a symbol.
        
        Args:
            symbol: FQN of the symbol
            direction: "from" (outgoing), "to" (incoming), or "both"
            
        Returns:
            Dictionary with edge_type -> list of connected symbols
        """
        relationships = {}
        
        if direction in ["from", "both"]:
            outgoing = self.query("""
                SELECT edge_type, dst
                FROM edges
                WHERE src = ?
            """, (symbol,))
            
            for edge in outgoing:
                edge_type = f"outgoing_{edge['edge_type']}"
                if edge_type not in relationships:
                    relationships[edge_type] = []
                relationships[edge_type].append(edge['dst'])
        
        if direction in ["to", "both"]:
            incoming = self.query("""
                SELECT edge_type, src
                FROM edges
                WHERE dst = ?
            """, (symbol,))
            
            for edge in incoming:
                edge_type = f"incoming_{edge['edge_type']}"
                if edge_type not in relationships:
                    relationships[edge_type] = []
                relationships[edge_type].append(edge['src'])
        
        return relationships
    
    def trace_paths(self, start: str, end: Optional[str] = None, 
                   max_depth: int = 5, edge_type: Optional[str] = None) -> List[List[str]]:
        """
        Find paths between symbols.
        
        Args:
            start: Starting symbol FQN
            end: Optional ending symbol FQN (if None, returns all reachable)
            max_depth: Maximum path length
            edge_type: Optional filter for edge type
            
        Returns:
            List of paths (each path is a list of symbol FQNs)
        """
        paths = []
        visited = set()
        
        def dfs(current: str, target: Optional[str], path: List[str], depth: int):
            if depth > max_depth:
                return
            
            if target and current == target:
                paths.append(path[:])
                return
            elif not target and depth > 0:
                # No specific target, collect all paths
                paths.append(path[:])
            
            visited.add(current)
            
            # Get next nodes
            if edge_type:
                edges = self.query("""
                    SELECT dst FROM edges
                    WHERE src = ? AND edge_type = ?
                """, (current, edge_type))
            else:
                edges = self.query("""
                    SELECT dst FROM edges
                    WHERE src = ?
                """, (current,))
            
            for edge in edges:
                next_node = edge['dst']
                if next_node and next_node not in visited:
                    path.append(next_node)
                    dfs(next_node, target, path, depth + 1)
                    path.pop()
            
            visited.remove(current)
        
        dfs(start, end, [start], 0)
        return paths
    
    def get_neighborhood(self, symbol: str, radius: int = 2) -> Dict[str, Any]:
        """
        Get the neighborhood of a symbol (all connected symbols within radius).
        
        Args:
            symbol: FQN of the symbol
            radius: How many hops to explore
            
        Returns:
            Dictionary with symbols and their distances
        """
        neighborhood = {symbol: 0}
        to_explore = [(symbol, 0)]
        
        while to_explore:
            current, distance = to_explore.pop(0)
            
            if distance >= radius:
                continue
            
            # Get all connected symbols
            edges = self.query("""
                SELECT dst FROM edges WHERE src = ?
                UNION
                SELECT src FROM edges WHERE dst = ?
            """, (current, current))
            
            for edge in edges:
                neighbor = edge['dst'] if edge['dst'] else edge['src']
                if neighbor and neighbor not in neighborhood:
                    neighborhood[neighbor] = distance + 1
                    to_explore.append((neighbor, distance + 1))
        
        return neighborhood
    
    def find_patterns(self, pattern_query: str) -> List[Dict[str, Any]]:
        """
        Let agents define their own patterns with SQL.
        
        Example patterns agent might use:
        - Functions that call both auth and database functions
        - Classes with more than N methods
        - Circular dependencies
        - Whatever the agent thinks is relevant
        
        Args:
            pattern_query: SQL query defining the pattern
            
        Returns:
            Query results
        """
        return self.query(pattern_query)
    
    def get_context(self, symbol: str, context_size: int = 5) -> Dict[str, Any]:
        """
        Get contextual information about a symbol.
        Agent decides what context means and how to use it.
        
        Args:
            symbol: FQN of the symbol
            context_size: How much context to gather
            
        Returns:
            Dictionary with various contextual information
        """
        sym = self.get_symbol(symbol)
        if not sym:
            return {}
        
        # Gather various context - agent decides what's relevant
        context = {
            "symbol": sym,
            "file_symbols": self.query("""
                SELECT * FROM symbols 
                WHERE file_id = (SELECT file_id FROM symbols WHERE fqn = ?)
            """, (symbol,)),
            "relationships": self.get_relationships(symbol),
            "callers": self.query("SELECT src FROM edges WHERE dst = ? AND edge_type = 'calls'", (symbol,)),
            "callees": self.query("SELECT dst FROM edges WHERE src = ? AND edge_type = 'calls'", (symbol,)),
            "imports": self.query("SELECT dst FROM edges WHERE src = ? AND edge_type = 'imports'", (symbol,)),
            "neighborhood": self.get_neighborhood(symbol, radius=context_size)
        }
        
        return context
    
    def compare_symbols(self, symbol1: str, symbol2: str) -> Dict[str, Any]:
        """
        Compare two symbols - agent decides what comparison means.
        
        Args:
            symbol1: First symbol FQN
            symbol2: Second symbol FQN
            
        Returns:
            Dictionary with comparison data
        """
        sym1 = self.get_symbol(symbol1)
        sym2 = self.get_symbol(symbol2)
        
        if not sym1 or not sym2:
            return {"error": "One or both symbols not found"}
        
        rel1 = self.get_relationships(symbol1)
        rel2 = self.get_relationships(symbol2)
        
        return {
            "symbol1": sym1,
            "symbol2": sym2,
            "relationships1": rel1,
            "relationships2": rel2,
            "shared_callees": set(rel1.get("outgoing_calls", [])) & set(rel2.get("outgoing_calls", [])),
            "shared_callers": set(rel1.get("incoming_calls", [])) & set(rel2.get("incoming_calls", [])),
        }
    
    def get_file_summary(self, file_path: str) -> Dict[str, Any]:
        """
        Get summary of a file - agent interprets what's important.
        
        Args:
            file_path: Path to the file
            
        Returns:
            Raw data about the file
        """
        symbols = self.query("""
            SELECT s.*, f.path
            FROM symbols s
            JOIN files f ON s.file_id = f.id
            WHERE f.path = ?
        """, (file_path,))
        
        edges = self.query("""
            SELECT e.*
            FROM edges e
            WHERE e.src IN (SELECT fqn FROM symbols s JOIN files f ON s.file_id = f.id WHERE f.path = ?)
               OR e.dst IN (SELECT fqn FROM symbols s JOIN files f ON s.file_id = f.id WHERE f.path = ?)
        """, (file_path, file_path))
        
        return {
            "file": file_path,
            "symbols": symbols,
            "edges": edges,
            "symbol_count": len(symbols),
            "edge_count": len(edges),
            "symbol_kinds": self._count_by_key(symbols, "kind"),
            "edge_types": self._count_by_key(edges, "edge_type")
        }
    
    def explore(self, start_point: str = "", strategy: str = "breadth") -> List[str]:
        """
        Explore the codebase from a starting point.
        Agent decides how to interpret results.
        
        Args:
            start_point: Where to start (empty = find entry points)
            strategy: "breadth", "depth", or "random"
            
        Returns:
            List of discovered symbols
        """
        if not start_point:
            # Find potential entry points (symbols with few/no callers)
            candidates = self.query("""
                SELECT s.fqn
                FROM symbols s
                WHERE s.fqn NOT IN (SELECT dst FROM edges WHERE edge_type = 'calls')
                   OR s.name LIKE '%main%'
                   OR s.name LIKE '%start%'
                   OR s.name LIKE '%init%'
                LIMIT 10
            """)
            return [c['fqn'] for c in candidates]
        
        # Explore from starting point
        if strategy == "breadth":
            neighborhood = self.get_neighborhood(start_point, radius=3)
            return list(neighborhood.keys())
        elif strategy == "depth":
            paths = self.trace_paths(start_point, max_depth=10)
            symbols = set()
            for path in paths:
                symbols.update(path)
            return list(symbols)
        else:
            # Random exploration
            edges = self.query("SELECT DISTINCT src, dst FROM edges ORDER BY RANDOM() LIMIT 50")
            symbols = set()
            for edge in edges:
                if edge['src']:
                    symbols.add(edge['src'])
                if edge['dst']:
                    symbols.add(edge['dst'])
            return list(symbols)
    
    def _count_by_key(self, items: List[Dict], key: str) -> Dict[str, int]:
        """Count items by a specific key"""
        counts = {}
        for item in items:
            value = item.get(key)
            if value:
                counts[value] = counts.get(value, 0) + 1
        return counts
    
    def close(self):
        """Close database connection"""
        if self.conn:
            self.conn.close()


# ========== Simple Usage Example ==========

def example_agent_usage(repo_path: str):
    """
    Example of how an agent might use these tools.
    The agent decides what's important, not the tools.
    """
    tools = CodeTools(repo_path)
    
    # Agent might ask: "What are the main entry points?"
    entry_points = tools.explore(start_point="")
    print(f"Found entry points: {entry_points[:5]}")
    
    # Agent might ask: "What does this function do?"
    if entry_points:
        context = tools.get_context(entry_points[0])
        # Agent interprets the context
        
    # Agent might ask: "Find functions that handle user input and access database"
    pattern = tools.find_patterns("""
        SELECT DISTINCT s1.fqn, s1.name
        FROM symbols s1
        WHERE (s1.name LIKE '%request%' OR s1.name LIKE '%input%')
          AND EXISTS (
              SELECT 1 FROM edges e
              WHERE e.src = s1.fqn
                AND e.dst IN (
                    SELECT fqn FROM symbols
                    WHERE name LIKE '%query%' OR name LIKE '%database%'
                )
          )
    """)
    
    # Agent interprets results and decides what matters
    
    tools.close()


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        example_agent_usage(sys.argv[1])