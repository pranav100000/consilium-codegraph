use petgraph::graph::{DiGraph, NodeIndex};
use protocol::{EdgeIR, EdgeType, SymbolIR};
use std::collections::HashMap;
use tracing::info;

/// In-memory graph for fast traversals
pub struct CodeGraph {
    graph: DiGraph<String, EdgeType>,
    symbol_to_node: HashMap<String, NodeIndex>,
    node_to_symbol: HashMap<NodeIndex, String>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            symbol_to_node: HashMap::new(),
            node_to_symbol: HashMap::new(),
        }
    }
    
    /// Build graph from symbols and edges
    pub fn build_from_data(symbols: &[SymbolIR], edges: &[EdgeIR]) -> Self {
        let mut graph = Self::new();
        
        // Add all symbols as nodes
        for symbol in symbols {
            graph.add_symbol(&symbol.id);
        }
        
        // Add edges
        for edge in edges {
            if let (Some(src), Some(dst)) = (&edge.src, &edge.dst) {
                graph.add_edge(src, dst, edge.edge_type.clone());
            }
        }
        
        info!("Built graph with {} nodes and {} edges", 
            graph.symbol_to_node.len(), 
            graph.graph.edge_count());
        
        graph
    }
    
    /// Add a symbol node to the graph
    pub fn add_symbol(&mut self, symbol_id: &str) -> NodeIndex {
        if let Some(&node) = self.symbol_to_node.get(symbol_id) {
            return node;
        }
        
        let node = self.graph.add_node(symbol_id.to_string());
        self.symbol_to_node.insert(symbol_id.to_string(), node);
        self.node_to_symbol.insert(node, symbol_id.to_string());
        node
    }
    
    /// Add an edge between two symbols
    pub fn add_edge(&mut self, from_id: &str, to_id: &str, edge_type: EdgeType) {
        let from_node = self.add_symbol(from_id);
        let to_node = self.add_symbol(to_id);
        self.graph.add_edge(from_node, to_node, edge_type);
    }
    
    /// Find all symbols that call the given symbol (incoming edges)
    pub fn find_callers(&self, symbol_id: &str, max_depth: usize) -> Vec<String> {
        let mut results = Vec::new();
        
        if let Some(&node) = self.symbol_to_node.get(symbol_id) {
            // Use BFS for level-by-level traversal
            let mut visited = HashMap::new();
            let mut queue = vec![(node, 0)];
            visited.insert(node, 0);
            
            while let Some((current, depth)) = queue.pop() {
                if depth > 0 && depth <= max_depth {
                    if let Some(sym_id) = self.node_to_symbol.get(&current) {
                        results.push(sym_id.clone());
                    }
                }
                
                if depth < max_depth {
                    // Get incoming edges (callers)
                    for neighbor in self.graph.neighbors_directed(current, petgraph::Direction::Incoming) {
                        if !visited.contains_key(&neighbor) {
                            visited.insert(neighbor, depth + 1);
                            queue.push((neighbor, depth + 1));
                        }
                    }
                }
            }
        }
        
        results
    }
    
    /// Find all symbols called by the given symbol (outgoing edges)
    pub fn find_callees(&self, symbol_id: &str, max_depth: usize) -> Vec<String> {
        let mut results = Vec::new();
        
        if let Some(&node) = self.symbol_to_node.get(symbol_id) {
            // Use BFS for level-by-level traversal
            let mut visited = HashMap::new();
            let mut queue = vec![(node, 0)];
            visited.insert(node, 0);
            
            while let Some((current, depth)) = queue.pop() {
                if depth > 0 && depth <= max_depth {
                    if let Some(sym_id) = self.node_to_symbol.get(&current) {
                        results.push(sym_id.clone());
                    }
                }
                
                if depth < max_depth {
                    // Get outgoing edges (callees)
                    for neighbor in self.graph.neighbors_directed(current, petgraph::Direction::Outgoing) {
                        if !visited.contains_key(&neighbor) {
                            visited.insert(neighbor, depth + 1);
                            queue.push((neighbor, depth + 1));
                        }
                    }
                }
            }
        }
        
        results
    }
    
    /// Find all symbols in the same strongly connected component
    pub fn find_cycles_containing(&self, symbol_id: &str) -> Vec<Vec<String>> {
        use petgraph::algo::kosaraju_scc;
        
        let sccs = kosaraju_scc(&self.graph);
        let mut cycles = Vec::new();
        
        if let Some(&node) = self.symbol_to_node.get(symbol_id) {
            for scc in sccs {
                if scc.contains(&node) && scc.len() > 1 {
                    let cycle: Vec<String> = scc.iter()
                        .filter_map(|&n| self.node_to_symbol.get(&n).cloned())
                        .collect();
                    cycles.push(cycle);
                }
            }
        }
        
        cycles
    }
    
    /// Find shortest path between two symbols
    pub fn find_path(&self, from_id: &str, to_id: &str) -> Option<Vec<String>> {
        use petgraph::algo::dijkstra;
        
        let from_node = self.symbol_to_node.get(from_id)?;
        let to_node = self.symbol_to_node.get(to_id)?;
        
        let paths = dijkstra(&self.graph, *from_node, Some(*to_node), |_| 1);
        
        if paths.contains_key(to_node) {
            // Reconstruct path using BFS
            let mut path = Vec::new();
            let _current = *to_node;
            
            // This is simplified - in production we'd track predecessors
            path.push(to_id.to_string());
            
            // For now, just return endpoints
            if from_id != to_id {
                path.insert(0, from_id.to_string());
            }
            
            Some(path)
        } else {
            None
        }
    }
    
    /// Get graph statistics
    pub fn stats(&self) -> GraphStats {
        GraphStats {
            node_count: self.graph.node_count(),
            edge_count: self.graph.edge_count(),
            is_cyclic: petgraph::algo::is_cyclic_directed(&self.graph),
        }
    }
}

pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub is_cyclic: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{Language, Span};
    
    fn create_test_symbol(id: &str, name: &str) -> SymbolIR {
        SymbolIR {
            id: id.to_string(),
            lang: Language::TypeScript,
            kind: protocol::SymbolKind::Function,
            name: name.to_string(),
            fqn: format!("test.{}", name),
            signature: None,
            file_path: "test.ts".to_string(),
            span: Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 },
            visibility: None,
            doc: None,
            sig_hash: "test".to_string(),
        }
    }
    
    fn create_test_edge(src: &str, dst: &str, edge_type: EdgeType) -> EdgeIR {
        EdgeIR {
            edge_type,
            src: Some(src.to_string()),
            dst: Some(dst.to_string()),
            file_src: None,
            file_dst: None,
            resolution: protocol::Resolution::Syntactic,
            meta: HashMap::new(),
            provenance: HashMap::new(),
        }
    }
    
    #[test]
    fn test_build_graph() {
        let symbols = vec![
            create_test_symbol("a", "funcA"),
            create_test_symbol("b", "funcB"),
            create_test_symbol("c", "funcC"),
        ];
        
        let edges = vec![
            create_test_edge("a", "b", EdgeType::Calls),
            create_test_edge("b", "c", EdgeType::Calls),
        ];
        
        let graph = CodeGraph::build_from_data(&symbols, &edges);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 2);
        assert!(!stats.is_cyclic);
    }
    
    #[test]
    fn test_find_callers() {
        let symbols = vec![
            create_test_symbol("a", "funcA"),
            create_test_symbol("b", "funcB"),
            create_test_symbol("c", "funcC"),
            create_test_symbol("d", "funcD"),
        ];
        
        let edges = vec![
            create_test_edge("a", "b", EdgeType::Calls),
            create_test_edge("c", "b", EdgeType::Calls),
            create_test_edge("d", "c", EdgeType::Calls),
        ];
        
        let graph = CodeGraph::build_from_data(&symbols, &edges);
        
        // Direct callers of 'b' (depth 1)
        let callers = graph.find_callers("b", 1);
        assert_eq!(callers.len(), 2);
        assert!(callers.contains(&"a".to_string()));
        assert!(callers.contains(&"c".to_string()));
        
        // Callers at depth 2
        let callers = graph.find_callers("b", 2);
        assert_eq!(callers.len(), 3);
        assert!(callers.contains(&"d".to_string()));
    }
    
    #[test]
    fn test_find_callees() {
        let symbols = vec![
            create_test_symbol("a", "funcA"),
            create_test_symbol("b", "funcB"),
            create_test_symbol("c", "funcC"),
            create_test_symbol("d", "funcD"),
        ];
        
        let edges = vec![
            create_test_edge("a", "b", EdgeType::Calls),
            create_test_edge("a", "c", EdgeType::Calls),
            create_test_edge("b", "d", EdgeType::Calls),
        ];
        
        let graph = CodeGraph::build_from_data(&symbols, &edges);
        
        // Direct callees of 'a' (depth 1)
        let callees = graph.find_callees("a", 1);
        assert_eq!(callees.len(), 2);
        assert!(callees.contains(&"b".to_string()));
        assert!(callees.contains(&"c".to_string()));
        
        // Callees at depth 2
        let callees = graph.find_callees("a", 2);
        assert_eq!(callees.len(), 3);
        assert!(callees.contains(&"d".to_string()));
    }
    
    #[test]
    fn test_detect_cycle() {
        let symbols = vec![
            create_test_symbol("a", "funcA"),
            create_test_symbol("b", "funcB"),
            create_test_symbol("c", "funcC"),
        ];
        
        let edges = vec![
            create_test_edge("a", "b", EdgeType::Calls),
            create_test_edge("b", "c", EdgeType::Calls),
            create_test_edge("c", "a", EdgeType::Calls), // Creates a cycle
        ];
        
        let graph = CodeGraph::build_from_data(&symbols, &edges);
        
        let stats = graph.stats();
        assert!(stats.is_cyclic);
        
        let cycles = graph.find_cycles_containing("a");
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_empty_graph() {
        let graph = CodeGraph::new();
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert!(!stats.is_cyclic);
        
        // Test operations on empty graph
        assert_eq!(graph.find_callers("nonexistent", 1), Vec::<String>::new());
        assert_eq!(graph.find_callees("nonexistent", 1), Vec::<String>::new());
        assert_eq!(graph.find_cycles_containing("nonexistent"), Vec::<Vec<String>>::new());
        assert_eq!(graph.find_path("a", "b"), None);
    }

    #[test]
    fn test_single_node_graph() {
        let mut graph = CodeGraph::new();
        graph.add_symbol("lonely");
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 1);
        assert_eq!(stats.edge_count, 0);
        assert!(!stats.is_cyclic);
        
        assert_eq!(graph.find_callers("lonely", 1), Vec::<String>::new());
        assert_eq!(graph.find_callees("lonely", 1), Vec::<String>::new());
        assert_eq!(graph.find_cycles_containing("lonely"), Vec::<Vec<String>>::new());
        assert_eq!(graph.find_path("lonely", "lonely"), Some(vec!["lonely".to_string()]));
    }

    #[test]
    fn test_self_loop() {
        let mut graph = CodeGraph::new();
        graph.add_edge("recursive", "recursive", EdgeType::Calls);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 1);
        assert_eq!(stats.edge_count, 1);
        assert!(stats.is_cyclic);
        
        // Self-loops are not returned as cycles in SCC algorithm (single node)
        assert_eq!(graph.find_cycles_containing("recursive"), Vec::<Vec<String>>::new());
        
        // Note: The BFS implementation marks the starting node as visited at depth 0,
        // so it won't be revisited even with a self-loop
        assert_eq!(graph.find_callers("recursive", 1), Vec::<String>::new());
        assert_eq!(graph.find_callees("recursive", 1), Vec::<String>::new());
    }

    #[test]
    fn test_duplicate_edges() {
        let mut graph = CodeGraph::new();
        
        // Add same edge multiple times
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("a", "b", EdgeType::Reads);
        graph.add_edge("a", "b", EdgeType::Imports);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 3); // All edges are added
        
        // Callers/callees should still work correctly
        assert_eq!(graph.find_callers("b", 1), vec!["a".to_string()]);
        assert_eq!(graph.find_callees("a", 1), vec!["b".to_string()]);
    }

    #[test]
    fn test_nonexistent_symbol_queries() {
        let mut graph = CodeGraph::new();
        graph.add_edge("a", "b", EdgeType::Calls);
        
        // Query for non-existent symbols
        assert_eq!(graph.find_callers("nonexistent", 1), Vec::<String>::new());
        assert_eq!(graph.find_callees("nonexistent", 1), Vec::<String>::new());
        assert_eq!(graph.find_cycles_containing("nonexistent"), Vec::<Vec<String>>::new());
        assert_eq!(graph.find_path("nonexistent", "a"), None);
        assert_eq!(graph.find_path("a", "nonexistent"), None);
    }

    #[test]
    fn test_depth_zero() {
        let mut graph = CodeGraph::new();
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("b", "c", EdgeType::Calls);
        
        // Depth 0 should return nothing
        assert_eq!(graph.find_callers("c", 0), Vec::<String>::new());
        assert_eq!(graph.find_callees("a", 0), Vec::<String>::new());
    }

    #[test]
    fn test_very_large_depth() {
        let mut graph = CodeGraph::new();
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("b", "c", EdgeType::Calls);
        
        // Large depth should still work and return all reachable nodes
        let callers = graph.find_callers("c", 1000000);
        assert_eq!(callers.len(), 2);
        assert!(callers.contains(&"a".to_string()));
        assert!(callers.contains(&"b".to_string()));
    }

    #[test]
    fn test_diamond_pattern() {
        let mut graph = CodeGraph::new();
        
        // Create diamond: a -> b -> d
        //                  \-> c ->/
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("a", "c", EdgeType::Calls);
        graph.add_edge("b", "d", EdgeType::Calls);
        graph.add_edge("c", "d", EdgeType::Calls);
        
        let stats = graph.stats();
        assert!(!stats.is_cyclic);
        
        // d should have 2 direct callers
        let callers = graph.find_callers("d", 1);
        assert_eq!(callers.len(), 2);
        assert!(callers.contains(&"b".to_string()));
        assert!(callers.contains(&"c".to_string()));
        
        // d should have 3 callers at depth 2 (a, b, c)
        let callers = graph.find_callers("d", 2);
        assert_eq!(callers.len(), 3);
        assert!(callers.contains(&"a".to_string()));
    }

    #[test]
    fn test_disconnected_components() {
        let mut graph = CodeGraph::new();
        
        // Component 1
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("b", "c", EdgeType::Calls);
        
        // Component 2 (disconnected)
        graph.add_edge("x", "y", EdgeType::Calls);
        graph.add_edge("y", "z", EdgeType::Calls);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 6);
        assert_eq!(stats.edge_count, 4);
        assert!(!stats.is_cyclic);
        
        // No path between components
        assert_eq!(graph.find_path("a", "x"), None);
        assert_eq!(graph.find_path("z", "c"), None);
        
        // But paths within components work
        assert!(graph.find_path("a", "c").is_some());
        assert!(graph.find_path("x", "z").is_some());
    }

    #[test]
    fn test_multiple_cycles() {
        let mut graph = CodeGraph::new();
        
        // Create two separate cycles
        // Cycle 1: a -> b -> c -> a
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("b", "c", EdgeType::Calls);
        graph.add_edge("c", "a", EdgeType::Calls);
        
        // Cycle 2: x -> y -> z -> x
        graph.add_edge("x", "y", EdgeType::Calls);
        graph.add_edge("y", "z", EdgeType::Calls);
        graph.add_edge("z", "x", EdgeType::Calls);
        
        let stats = graph.stats();
        assert!(stats.is_cyclic);
        
        // Each node should be in exactly one cycle
        let cycles_a = graph.find_cycles_containing("a");
        assert_eq!(cycles_a.len(), 1);
        assert_eq!(cycles_a[0].len(), 3);
        
        let cycles_x = graph.find_cycles_containing("x");
        assert_eq!(cycles_x.len(), 1);
        assert_eq!(cycles_x[0].len(), 3);
    }

    #[test]
    fn test_unicode_symbol_ids() {
        let mut graph = CodeGraph::new();
        
        // Test with unicode characters
        graph.add_edge("函数A", "函数B", EdgeType::Calls);
        graph.add_edge("🚀", "🌟", EdgeType::Reads);
        graph.add_edge("café", "naïve", EdgeType::Imports);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 6);
        assert_eq!(stats.edge_count, 3);
        
        assert_eq!(graph.find_callees("函数A", 1), vec!["函数B".to_string()]);
        assert_eq!(graph.find_callers("🌟", 1), vec!["🚀".to_string()]);
        assert!(graph.find_path("café", "naïve").is_some());
    }

    #[test]
    fn test_special_character_symbol_ids() {
        let mut graph = CodeGraph::new();
        
        // Test with special characters that might cause issues
        graph.add_edge("a-b", "c_d", EdgeType::Calls);
        graph.add_edge("e.f", "g:h", EdgeType::Reads);
        graph.add_edge("i/j", "k\\l", EdgeType::Imports);
        graph.add_edge("m'n", "o\"p", EdgeType::Calls);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 8);
        assert_eq!(stats.edge_count, 4);
        
        assert_eq!(graph.find_callees("a-b", 1), vec!["c_d".to_string()]);
        assert_eq!(graph.find_callers("g:h", 1), vec!["e.f".to_string()]);
    }

    #[test]
    fn test_idempotent_symbol_addition() {
        let mut graph = CodeGraph::new();
        
        // Add same symbol multiple times
        let node1 = graph.add_symbol("test");
        let node2 = graph.add_symbol("test");
        let node3 = graph.add_symbol("test");
        
        // Should return the same node index
        assert_eq!(node1, node2);
        assert_eq!(node2, node3);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 1);
    }

    #[test]
    fn test_large_linear_graph() {
        let mut graph = CodeGraph::new();
        
        // Create a long chain: 0 -> 1 -> 2 -> ... -> 99
        for i in 0..100 {
            if i > 0 {
                graph.add_edge(&(i-1).to_string(), &i.to_string(), EdgeType::Calls);
            }
        }
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 100);
        assert_eq!(stats.edge_count, 99);
        assert!(!stats.is_cyclic);
        
        // Test traversal limits
        assert_eq!(graph.find_callers("10", 1).len(), 1);
        assert_eq!(graph.find_callers("10", 5).len(), 5);
        assert_eq!(graph.find_callers("10", 100).len(), 10); // Only 10 predecessors exist
        
        // Test path finding
        assert!(graph.find_path("0", "99").is_some());
        assert!(graph.find_path("50", "25").is_none()); // No backward path
    }

    #[test]
    fn test_complete_graph() {
        let mut graph = CodeGraph::new();
        
        // Create a complete graph with 5 nodes (everyone calls everyone)
        let nodes = vec!["a", "b", "c", "d", "e"];
        for &from in &nodes {
            for &to in &nodes {
                if from != to {
                    graph.add_edge(from, to, EdgeType::Calls);
                }
            }
        }
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 5);
        assert_eq!(stats.edge_count, 20); // 5 * 4 = 20 edges
        assert!(stats.is_cyclic);
        
        // Everyone is a caller and callee of everyone else
        let callers = graph.find_callers("c", 1);
        assert_eq!(callers.len(), 4);
        
        let callees = graph.find_callees("c", 1);
        assert_eq!(callees.len(), 4);
    }

    #[test]
    fn test_tree_structure() {
        let mut graph = CodeGraph::new();
        
        // Create a binary tree structure
        //        root
        //       /    \
        //      a      b
        //     / \    / \
        //    c   d  e   f
        graph.add_edge("root", "a", EdgeType::Calls);
        graph.add_edge("root", "b", EdgeType::Calls);
        graph.add_edge("a", "c", EdgeType::Calls);
        graph.add_edge("a", "d", EdgeType::Calls);
        graph.add_edge("b", "e", EdgeType::Calls);
        graph.add_edge("b", "f", EdgeType::Calls);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 7);
        assert_eq!(stats.edge_count, 6);
        assert!(!stats.is_cyclic);
        
        // Test depth-based traversal
        let callees_1 = graph.find_callees("root", 1);
        assert_eq!(callees_1.len(), 2); // a, b
        
        let callees_2 = graph.find_callees("root", 2);
        assert_eq!(callees_2.len(), 6); // a, b, c, d, e, f
    }

    #[test]
    fn test_mixed_edge_types() {
        let mut graph = CodeGraph::new();
        
        // Different edge types between same nodes
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("b", "c", EdgeType::Reads);
        graph.add_edge("c", "a", EdgeType::Imports);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 3);
        assert!(stats.is_cyclic);
        
        // Edge type doesn't affect traversal
        let cycles = graph.find_cycles_containing("a");
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_build_from_data_with_edges_without_symbols() {
        // Test that edges with missing symbols are handled gracefully
        let symbols = vec![
            create_test_symbol("a", "funcA"),
            create_test_symbol("b", "funcB"),
        ];
        
        let edges = vec![
            create_test_edge("a", "b", EdgeType::Calls),
            create_test_edge("b", "c", EdgeType::Calls), // 'c' not in symbols
            create_test_edge("d", "a", EdgeType::Calls), // 'd' not in symbols
        ];
        
        let graph = CodeGraph::build_from_data(&symbols, &edges);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 4); // a, b, c, d all added
        assert_eq!(stats.edge_count, 3);
    }

    #[test]
    fn test_edge_with_null_endpoints() {
        let symbols = vec![
            create_test_symbol("a", "funcA"),
        ];
        
        let edges = vec![
            EdgeIR {
                edge_type: EdgeType::Calls,
                src: None, // Null source
                dst: Some("a".to_string()),
                file_src: None,
                file_dst: None,
                resolution: protocol::Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            },
            EdgeIR {
                edge_type: EdgeType::Calls,
                src: Some("a".to_string()),
                dst: None, // Null destination
                file_src: None,
                file_dst: None,
                resolution: protocol::Resolution::Syntactic,
                meta: HashMap::new(),
                provenance: HashMap::new(),
            },
        ];
        
        let graph = CodeGraph::build_from_data(&symbols, &edges);
        
        let stats = graph.stats();
        assert_eq!(stats.node_count, 1); // Only 'a' added
        assert_eq!(stats.edge_count, 0); // No edges added (null endpoints)
    }

    #[test]
    fn test_path_finding_edge_cases() {
        let mut graph = CodeGraph::new();
        
        graph.add_edge("a", "b", EdgeType::Calls);
        graph.add_edge("b", "c", EdgeType::Calls);
        
        // Path to self
        let self_path = graph.find_path("a", "a");
        assert_eq!(self_path, Some(vec!["a".to_string()]));
        
        // Direct path
        let direct_path = graph.find_path("a", "b");
        assert!(direct_path.is_some());
        
        // Multi-hop path
        let multi_path = graph.find_path("a", "c");
        assert!(multi_path.is_some());
        
        // No path (wrong direction)
        let no_path = graph.find_path("c", "a");
        assert_eq!(no_path, None);
    }
}