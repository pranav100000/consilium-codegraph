"""
Code analysis functions for security, quality, and architecture review
"""

import re
from typing import List, Optional, Dict, Set, Any
from collections import defaultdict
from pathlib import Path

from .models import (
    Symbol, SymbolKind, SecurityIssue, Severity, Location,
    ComplexityMetrics, CodeSmell, DuplicateCode, DataFlow,
    AnalysisQuality, LayerViolation, RefactoringSuggestion
)
from .code_graph import CodeGraph


class CodeAnalyzer:
    """
    Performs various analyses on the code graph for agents.
    """
    
    def __init__(self, graph: CodeGraph):
        """
        Initialize analyzer with a code graph.
        
        Args:
            graph: CodeGraph instance to analyze
        """
        self.graph = graph
        self._init_patterns()
    
    def _init_patterns(self):
        """Initialize security and code smell patterns"""
        # SQL injection patterns
        self.sql_patterns = [
            r"execute\s*\(",
            r"query\s*\(",
            r"raw\s*\(",
            r"\.format\s*\(",
            r"\+\s*['\"].*SELECT",
            r"\+\s*['\"].*INSERT",
            r"\+\s*['\"].*UPDATE",
            r"\+\s*['\"].*DELETE"
        ]
        
        # Authentication patterns
        self.auth_patterns = [
            r"authenticate",
            r"authorize",
            r"check_permission",
            r"verify_token",
            r"@auth_required",
            r"@login_required"
        ]
        
        # Input validation patterns
        self.input_sources = [
            r"request\.",
            r"req\.",
            r"body\.",
            r"params\.",
            r"query\.",
            r"args\.",
            r"input\(",
            r"stdin"
        ]
    
    # ========== Data Flow Analysis ==========
    
    def trace_data_flow(self, 
                       source: str, 
                       sink: Optional[str] = None,
                       max_depth: int = 10) -> List[DataFlow]:
        """
        Track data flow from source to optional sink.
        
        Args:
            source: Source pattern or symbol
            sink: Optional sink pattern or symbol
            max_depth: Maximum depth to trace
            
        Returns:
            List of data flow paths
        """
        flows = []
        
        # Find source symbols
        source_symbols = self.graph.find_symbols(source)
        
        for src_symbol in source_symbols:
            if sink:
                # Find specific sink
                paths = self.graph.find_path(src_symbol.fqn, sink, max_depth)
                for path in paths:
                    flows.append(DataFlow(
                        source=src_symbol,
                        sink=path[-1] if path else src_symbol,
                        path=path,
                        is_tainted=self._is_tainted_path(path),
                        is_sanitized=self._is_sanitized_path(path),
                        confidence=0.8,
                        analysis_quality=AnalysisQuality.SYNTACTIC
                    ))
            else:
                # Find all flows from source
                callees = self.graph.get_callees(src_symbol.fqn, max_depth=max_depth)
                for call_path in callees:
                    if self._is_sensitive_sink(call_path.path[-1]):
                        flows.append(DataFlow(
                            source=src_symbol,
                            sink=call_path.path[-1],
                            path=call_path.path,
                            is_tainted=self._is_tainted_path(call_path.path),
                            is_sanitized=self._is_sanitized_path(call_path.path),
                            confidence=0.7,
                            analysis_quality=AnalysisQuality.HEURISTIC
                        ))
        
        return flows
    
    def find_tainted_paths(self, source_pattern: str) -> List[DataFlow]:
        """
        Find all potentially tainted data flows from untrusted sources.
        
        Args:
            source_pattern: Pattern matching untrusted input sources
            
        Returns:
            List of tainted data flows
        """
        tainted_flows = []
        
        # Find functions that handle user input
        for pattern in self.input_sources:
            if pattern in source_pattern.lower():
                flows = self.trace_data_flow(source_pattern)
                tainted_flows.extend([f for f in flows if f.is_tainted])
        
        return tainted_flows
    
    # ========== Security Analysis ==========
    
    def find_sql_injections(self) -> List[SecurityIssue]:
        """Find potential SQL injection vulnerabilities"""
        issues = []
        
        # Find all database query functions
        db_functions = self.graph.find_symbols("query", SymbolKind.FUNCTION)
        db_functions.extend(self.graph.find_symbols("execute", SymbolKind.FUNCTION))
        
        for func in db_functions:
            # Check if function receives user input
            callers = self.graph.get_callers(func.fqn, max_depth=3)
            
            for caller_path in callers:
                if self._handles_user_input(caller_path.path[0]):
                    # Check if input is sanitized
                    if not self._has_sanitization(caller_path.path):
                        issues.append(SecurityIssue(
                            issue_id=f"sql_injection_{func.fqn}_{len(issues)}",
                            type="sql_injection",
                            severity=Severity.CRITICAL,
                            location=func.location,
                            description=f"Potential SQL injection in {func.name}. User input may flow to database query without sanitization.",
                            evidence=[f"Call path: {' -> '.join([s.name for s in caller_path.path])}"],
                            fix_suggestion="Use parameterized queries or prepared statements",
                            confidence=0.8,
                            cwe_id="CWE-89"
                        ))
        
        return issues
    
    def find_auth_bypasses(self) -> List[SecurityIssue]:
        """Find missing authentication checks"""
        issues = []
        
        # Find API endpoints or route handlers
        endpoints = self._find_api_endpoints()
        
        for endpoint in endpoints:
            # Check if endpoint has auth check
            if not self._has_auth_check(endpoint):
                issues.append(SecurityIssue(
                    issue_id=f"auth_bypass_{endpoint.fqn}",
                    type="missing_authentication",
                    severity=Severity.HIGH,
                    location=endpoint.location,
                    description=f"Endpoint {endpoint.name} lacks authentication check",
                    evidence=[f"No auth decorator or check found in {endpoint.name}"],
                    fix_suggestion="Add authentication middleware or decorator",
                    confidence=0.7,
                    cwe_id="CWE-306",
                    owasp_category="A01:2021"
                ))
        
        return issues
    
    def find_unsafe_operations(self, operation_type: str = "all") -> List[SecurityIssue]:
        """
        Find unsafe operations like eval, exec, deserialize, etc.
        
        Args:
            operation_type: Type of operation to check ("eval", "exec", "deserialize", "all")
            
        Returns:
            List of security issues
        """
        issues = []
        unsafe_functions = {
            "eval": ["eval", "exec", "compile"],
            "deserialize": ["pickle.loads", "yaml.load", "json.loads"],
            "command": ["os.system", "subprocess.call", "popen"]
        }
        
        patterns = unsafe_functions.get(operation_type, [])
        if operation_type == "all":
            patterns = sum(unsafe_functions.values(), [])
        
        for pattern in patterns:
            symbols = self.graph.find_symbols(pattern)
            for symbol in symbols:
                # Check if it handles external input
                callers = self.graph.get_callers(symbol.fqn, max_depth=2)
                for caller_path in callers:
                    if self._handles_user_input(caller_path.path[0]):
                        issues.append(SecurityIssue(
                            issue_id=f"unsafe_op_{symbol.fqn}_{len(issues)}",
                            type=f"unsafe_{operation_type}",
                            severity=Severity.HIGH,
                            location=symbol.location,
                            description=f"Unsafe operation {symbol.name} with potential user input",
                            evidence=[f"Function {symbol.name} called with external data"],
                            fix_suggestion=f"Use safe alternatives or validate/sanitize input",
                            confidence=0.75,
                            cwe_id="CWE-94"
                        ))
        
        return issues
    
    # ========== Code Quality Analysis ==========
    
    def get_complexity(self, symbol: str) -> ComplexityMetrics:
        """
        Calculate complexity metrics for a symbol.
        
        Args:
            symbol: FQN of the symbol
            
        Returns:
            ComplexityMetrics object
        """
        sym = self.graph.get_symbol(symbol)
        if not sym:
            return ComplexityMetrics(0, 0, 0, 0, 0, 0)
        
        # Get callees to estimate complexity
        callees = self.graph.get_callees(symbol, max_depth=1)
        
        # Estimate metrics (simplified - would need AST for accurate calculation)
        return ComplexityMetrics(
            cyclomatic=len(callees) + 1,  # Simplified estimation
            cognitive=len(callees) * 2,  # Simplified estimation
            lines_of_code=50,  # Would need file parsing
            nesting_depth=3,  # Would need AST
            parameter_count=self._count_parameters(sym),
            return_points=1  # Would need AST
        )
    
    def find_duplicates(self, min_lines: int = 10) -> List[DuplicateCode]:
        """
        Find duplicate code blocks.
        
        Args:
            min_lines: Minimum lines for duplicate detection
            
        Returns:
            List of duplicate code blocks
        """
        duplicates = []
        
        # Group symbols by similar names
        symbol_groups = defaultdict(list)
        all_symbols = self.graph.find_symbols("", limit=1000)
        
        for symbol in all_symbols:
            # Simple similarity: same prefix
            prefix = symbol.name.split("_")[0] if "_" in symbol.name else symbol.name[:5]
            symbol_groups[prefix].append(symbol)
        
        # Check for duplicates in each group
        for prefix, symbols in symbol_groups.items():
            if len(symbols) > 1:
                # Check if symbols have similar structure
                for i, sym1 in enumerate(symbols):
                    for sym2 in symbols[i+1:]:
                        similarity = self._calculate_similarity(sym1, sym2)
                        if similarity > 0.8:
                            duplicates.append(DuplicateCode(
                                locations=[sym1.location, sym2.location],
                                lines=min_lines,  # Estimated
                                tokens=100,  # Estimated
                                similarity=similarity,
                                code_snippet=f"Similar functions: {sym1.name} and {sym2.name}"
                            ))
        
        return duplicates
    
    def find_code_smells(self) -> List[CodeSmell]:
        """Find various code quality issues"""
        smells = []
        
        all_symbols = self.graph.find_symbols("", limit=1000)
        
        for symbol in all_symbols:
            # Long function names
            if len(symbol.name) > 50:
                smells.append(CodeSmell(
                    type="long_identifier",
                    location=symbol.location,
                    description=f"Function name '{symbol.name}' is too long",
                    impact="Reduces readability",
                    refactoring_suggestion="Use a shorter, more descriptive name",
                    severity=Severity.LOW
                ))
            
            # God functions (too many callees)
            if symbol.kind == SymbolKind.FUNCTION:
                callees = self.graph.get_callees(symbol.fqn)
                if len(callees) > 20:
                    smells.append(CodeSmell(
                        type="god_function",
                        location=symbol.location,
                        description=f"Function {symbol.name} does too much (calls {len(callees)} functions)",
                        impact="Hard to maintain and test",
                        refactoring_suggestion="Extract into smaller, focused functions",
                        severity=Severity.MEDIUM,
                        metrics={"callee_count": len(callees)}
                    ))
            
            # Unused symbols (no callers)
            if symbol.kind in [SymbolKind.FUNCTION, SymbolKind.METHOD]:
                callers = self.graph.get_callers(symbol.fqn)
                if len(callers) == 0 and not self._is_entry_point(symbol):
                    smells.append(CodeSmell(
                        type="dead_code",
                        location=symbol.location,
                        description=f"Function {symbol.name} appears to be unused",
                        impact="Increases maintenance burden",
                        refactoring_suggestion="Remove if truly unused",
                        severity=Severity.LOW
                    ))
        
        return smells
    
    # ========== Architecture Analysis ==========
    
    def get_module_dependencies(self) -> Dict[str, Set[str]]:
        """Get module-level dependency graph"""
        dependencies = defaultdict(set)
        
        # Get all edges with import type
        conn = self.graph.conn
        cursor = conn.cursor()
        cursor.execute("""
            SELECT e.src, e.dst 
            FROM edges e
            WHERE e.edge_type = 'imports'
        """)
        
        for row in cursor.fetchall():
            # Extract module from FQN
            src_module = self._get_module(row["src"])
            dst_module = self._get_module(row["dst"])
            
            if src_module != dst_module:
                dependencies[src_module].add(dst_module)
        
        return dict(dependencies)
    
    def find_circular_dependencies(self) -> List[List[str]]:
        """Find dependency cycles between modules"""
        dependencies = self.get_module_dependencies()
        cycles = []
        visited = set()
        rec_stack = []
        
        def dfs(module: str, path: List[str]):
            if module in rec_stack:
                # Found cycle
                cycle_start = rec_stack.index(module)
                cycles.append(rec_stack[cycle_start:] + [module])
                return
            
            if module in visited:
                return
            
            visited.add(module)
            rec_stack.append(module)
            
            for dep in dependencies.get(module, []):
                dfs(dep, path + [dep])
            
            rec_stack.pop()
        
        for module in dependencies:
            if module not in visited:
                dfs(module, [module])
        
        return cycles
    
    def check_layer_violations(self, rules: Dict[str, List[str]]) -> List[LayerViolation]:
        """
        Check for architecture layer violations.
        
        Args:
            rules: Dict mapping layer names to allowed dependency layers
                  e.g., {"ui": ["service"], "service": ["data"], "data": []}
        
        Returns:
            List of violations
        """
        violations = []
        
        # Get all cross-module dependencies
        dependencies = self.get_module_dependencies()
        
        for src_module, dst_modules in dependencies.items():
            src_layer = self._get_layer(src_module)
            
            for dst_module in dst_modules:
                dst_layer = self._get_layer(dst_module)
                
                # Check if dependency is allowed
                allowed_layers = rules.get(src_layer, [])
                if dst_layer not in allowed_layers and src_layer != dst_layer:
                    violations.append(LayerViolation(
                        from_layer=src_layer,
                        to_layer=dst_layer,
                        from_symbol=src_module,
                        to_symbol=dst_module,
                        violation_type="illegal_dependency",
                        suggested_path=[src_layer] + allowed_layers + [dst_layer]
                    ))
        
        return violations
    
    # ========== Helper Methods ==========
    
    def _is_tainted_path(self, path: List[Symbol]) -> bool:
        """Check if a path involves tainted (user) input"""
        for symbol in path:
            if self._handles_user_input(symbol):
                return True
        return False
    
    def _is_sanitized_path(self, path: List[Symbol]) -> bool:
        """Check if a path includes sanitization"""
        sanitization_keywords = ["sanitize", "validate", "escape", "clean", "filter"]
        for symbol in path:
            if any(keyword in symbol.name.lower() for keyword in sanitization_keywords):
                return True
        return False
    
    def _is_sensitive_sink(self, symbol: Symbol) -> bool:
        """Check if symbol is a sensitive operation"""
        sensitive_keywords = ["database", "file", "network", "exec", "eval", "system"]
        return any(keyword in symbol.name.lower() for keyword in sensitive_keywords)
    
    def _handles_user_input(self, symbol: Symbol) -> bool:
        """Check if symbol handles user input"""
        input_keywords = ["request", "input", "param", "arg", "query", "body", "form"]
        return any(keyword in symbol.name.lower() for keyword in input_keywords)
    
    def _has_sanitization(self, path: List[Symbol]) -> bool:
        """Check if path includes input sanitization"""
        return self._is_sanitized_path(path)
    
    def _find_api_endpoints(self) -> List[Symbol]:
        """Find API endpoint functions"""
        endpoints = []
        
        # Common endpoint patterns
        patterns = ["route", "api", "endpoint", "handler", "controller"]
        
        for pattern in patterns:
            endpoints.extend(self.graph.find_symbols(pattern, SymbolKind.FUNCTION))
        
        return endpoints
    
    def _has_auth_check(self, symbol: Symbol) -> bool:
        """Check if symbol has authentication"""
        # Check symbol name
        for pattern in self.auth_patterns:
            if re.search(pattern, symbol.name, re.IGNORECASE):
                return True
        
        # Check callees
        callees = self.graph.get_callees(symbol.fqn, max_depth=2)
        for callee_path in callees:
            for callee in callee_path.path:
                for pattern in self.auth_patterns:
                    if re.search(pattern, callee.name, re.IGNORECASE):
                        return True
        
        return False
    
    def _count_parameters(self, symbol: Symbol) -> int:
        """Count function parameters from signature"""
        if not symbol.signature:
            return 0
        
        # Simple parameter counting (would need proper parsing)
        params = symbol.signature.count(",") + 1 if "(" in symbol.signature else 0
        return params
    
    def _calculate_similarity(self, sym1: Symbol, sym2: Symbol) -> float:
        """Calculate similarity between two symbols"""
        # Simple name similarity
        name_sim = self._string_similarity(sym1.name, sym2.name)
        
        # Similar callees
        callees1 = set(c.path[-1].name for c in self.graph.get_callees(sym1.fqn))
        callees2 = set(c.path[-1].name for c in self.graph.get_callees(sym2.fqn))
        
        if callees1 or callees2:
            callee_sim = len(callees1 & callees2) / len(callees1 | callees2)
        else:
            callee_sim = 0
        
        return (name_sim + callee_sim) / 2
    
    def _string_similarity(self, s1: str, s2: str) -> float:
        """Calculate string similarity (simplified Jaccard)"""
        set1 = set(s1.lower())
        set2 = set(s2.lower())
        
        if not set1 and not set2:
            return 1.0
        
        return len(set1 & set2) / len(set1 | set2)
    
    def _is_entry_point(self, symbol: Symbol) -> bool:
        """Check if symbol is an entry point"""
        entry_patterns = ["main", "start", "init", "setup", "__init__"]
        return any(pattern in symbol.name.lower() for pattern in entry_patterns)
    
    def _get_module(self, fqn: str) -> str:
        """Extract module from FQN"""
        parts = fqn.split("::")
        return parts[0] if parts else fqn
    
    def _get_layer(self, module: str) -> str:
        """Determine architecture layer from module name"""
        if any(x in module.lower() for x in ["ui", "view", "controller", "route"]):
            return "presentation"
        elif any(x in module.lower() for x in ["service", "business", "logic"]):
            return "business"
        elif any(x in module.lower() for x in ["data", "repository", "model", "db"]):
            return "data"
        else:
            return "unknown"