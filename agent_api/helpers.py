"""
High-level helper functions for AI agents
"""

from typing import List, Optional, Dict, Any
from pathlib import Path

from .models import (
    Symbol, SymbolKind, FunctionExplanation, ImpactAnalysis,
    SecurityContext, RefactoringSuggestion, ComplexityMetrics,
    SecurityIssue
)
from .code_graph import CodeGraph
from .analyzer import CodeAnalyzer


class AgentHelpers:
    """
    High-level functions that combine multiple operations for agents.
    These are designed to be intuitive for LLM agents to use.
    """
    
    def __init__(self, repo_path: str):
        """
        Initialize helpers for a repository.
        
        Args:
            repo_path: Path to the repository root
        """
        self.graph = CodeGraph(repo_path)
        self.analyzer = CodeAnalyzer(self.graph)
    
    def explain_function(self, symbol: str) -> FunctionExplanation:
        """
        Get a comprehensive explanation of a function.
        
        Args:
            symbol: FQN of the function
            
        Returns:
            FunctionExplanation with purpose, parameters, complexity, etc.
        """
        sym = self.graph.get_symbol(symbol)
        if not sym:
            raise ValueError(f"Symbol {symbol} not found")
        
        # Get complexity
        complexity = self.analyzer.get_complexity(symbol)
        
        # Infer purpose from name and callees
        purpose = self._infer_purpose(sym)
        
        # Get parameters
        parameters = self._extract_parameters(sym)
        
        # Find side effects
        side_effects = self._find_side_effects(symbol)
        
        # Get test coverage (simplified - would need test mapping)
        test_coverage = self._estimate_test_coverage(symbol)
        
        # Get dependencies
        deps = self.graph.get_dependencies(symbol)
        dependencies = list(deps.dependencies.get(symbol, []))
        
        return FunctionExplanation(
            symbol=sym,
            purpose=purpose,
            parameters=parameters,
            returns=self._extract_return_type(sym),
            side_effects=side_effects,
            complexity=complexity,
            test_coverage=test_coverage,
            dependencies=dependencies
        )
    
    def analyze_change_impact(self, symbol: str) -> ImpactAnalysis:
        """
        Analyze what would be affected if this symbol changes.
        
        Args:
            symbol: FQN of the symbol
            
        Returns:
            ImpactAnalysis showing all affected code
        """
        # Get direct callers
        direct_callers = []
        for call_path in self.graph.get_callers(symbol, max_depth=1):
            if len(call_path.path) > 1:
                direct_callers.append(call_path.path[1])
        
        # Get transitive impact
        transitive_impact = set()
        for call_path in self.graph.get_callers(symbol, max_depth=5):
            for sym in call_path.path[1:]:
                transitive_impact.add(sym.fqn)
        
        # Find affected tests
        affected_tests = self._find_related_tests(symbol)
        
        # Determine affected features
        affected_features = self._identify_features(transitive_impact)
        
        # Calculate risk score
        risk_score = self._calculate_risk(
            len(direct_callers),
            len(transitive_impact),
            len(affected_tests)
        )
        
        return ImpactAnalysis(
            symbol=symbol,
            direct_callers=direct_callers,
            transitive_impact=transitive_impact,
            affected_tests=affected_tests,
            affected_features=affected_features,
            risk_score=risk_score
        )
    
    def find_similar_code(self, symbol: str, threshold: float = 0.7) -> List[Symbol]:
        """
        Find semantically similar functions.
        
        Args:
            symbol: FQN of the reference symbol
            threshold: Similarity threshold (0-1)
            
        Returns:
            List of similar symbols
        """
        reference = self.graph.get_symbol(symbol)
        if not reference:
            return []
        
        similar = []
        
        # Get all functions
        all_functions = self.graph.find_symbols("", SymbolKind.FUNCTION, limit=500)
        
        for func in all_functions:
            if func.fqn == symbol:
                continue
            
            # Calculate similarity based on:
            # 1. Name similarity
            # 2. Similar callees
            # 3. Similar callers
            
            similarity = self._calculate_code_similarity(reference, func)
            
            if similarity >= threshold:
                similar.append(func)
        
        return sorted(similar, key=lambda s: self._calculate_code_similarity(reference, s), reverse=True)
    
    def suggest_refactoring(self, symbol: str) -> List[RefactoringSuggestion]:
        """
        Suggest improvements for a function.
        
        Args:
            symbol: FQN of the symbol
            
        Returns:
            List of refactoring suggestions
        """
        suggestions = []
        
        sym = self.graph.get_symbol(symbol)
        if not sym:
            return suggestions
        
        # Check complexity
        complexity = self.analyzer.get_complexity(symbol)
        
        if complexity.cyclomatic > 10:
            suggestions.append(RefactoringSuggestion(
                type="extract_method",
                location=sym.location,
                description=f"Function {sym.name} has high cyclomatic complexity ({complexity.cyclomatic})",
                benefit="Improved readability and testability",
                example="Break down into smaller functions, each handling a specific responsibility"
            ))
        
        if complexity.parameter_count > 5:
            suggestions.append(RefactoringSuggestion(
                type="introduce_parameter_object",
                location=sym.location,
                description=f"Function {sym.name} has too many parameters ({complexity.parameter_count})",
                benefit="Cleaner interface and easier to extend",
                example="Group related parameters into a configuration object or data class"
            ))
        
        # Check for long names
        if len(sym.name) > 40:
            suggestions.append(RefactoringSuggestion(
                type="rename",
                location=sym.location,
                description=f"Function name '{sym.name}' is too long",
                benefit="Improved readability",
                example=f"Consider a shorter name like '{sym.name[:20]}...'"
            ))
        
        # Check for duplicate code
        similar = self.find_similar_code(symbol, threshold=0.85)
        if similar:
            suggestions.append(RefactoringSuggestion(
                type="extract_common",
                location=sym.location,
                description=f"Function similar to {similar[0].name}",
                benefit="Reduced duplication and easier maintenance",
                example="Extract common functionality into a shared function"
            ))
        
        return suggestions
    
    def get_security_context(self, symbol: str) -> SecurityContext:
        """
        Get security-relevant information about a symbol.
        
        Args:
            symbol: FQN of the symbol
            
        Returns:
            SecurityContext with security analysis
        """
        sym = self.graph.get_symbol(symbol)
        if not sym:
            raise ValueError(f"Symbol {symbol} not found")
        
        # Check various security aspects
        handles_input = self._handles_user_input(symbol)
        accesses_db = self._accesses_database(symbol)
        performs_auth = self._performs_auth(symbol)
        performs_crypto = self._uses_encryption(symbol)
        
        # Get external calls
        external_calls = self._find_external_calls(symbol)
        
        # Find vulnerabilities
        vulnerabilities = self._find_vulnerabilities_in(symbol)
        
        # Determine privilege level
        privilege_level = self._determine_privilege_level(symbol)
        
        return SecurityContext(
            symbol=symbol,
            handles_user_input=handles_input,
            accesses_database=accesses_db,
            performs_auth=performs_auth,
            performs_crypto=performs_crypto,
            external_calls=external_calls,
            vulnerabilities=vulnerabilities,
            privilege_level=privilege_level
        )
    
    def get_code_summary(self, file_path: str) -> Dict[str, Any]:
        """
        Get a summary of code in a file.
        
        Args:
            file_path: Path to the file (relative to repo root)
            
        Returns:
            Dictionary with file summary
        """
        symbols = self.graph.get_file_symbols(file_path)
        
        # Count by type
        type_counts = {}
        for sym in symbols:
            type_counts[sym.kind.value] = type_counts.get(sym.kind.value, 0) + 1
        
        # Find main symbols
        classes = [s for s in symbols if s.kind == SymbolKind.CLASS]
        functions = [s for s in symbols if s.kind == SymbolKind.FUNCTION]
        
        # Calculate metrics
        total_complexity = sum(
            self.analyzer.get_complexity(s.fqn).cyclomatic 
            for s in functions
        )
        
        return {
            "file": file_path,
            "total_symbols": len(symbols),
            "symbol_counts": type_counts,
            "main_classes": [c.name for c in classes],
            "main_functions": [f.name for f in functions],
            "total_complexity": total_complexity,
            "average_complexity": total_complexity / len(functions) if functions else 0
        }
    
    def find_entry_points(self) -> List[Symbol]:
        """
        Find main entry points in the codebase.
        
        Returns:
            List of entry point symbols
        """
        entry_points = []
        
        # Common entry point patterns
        patterns = ["main", "start", "init", "setup", "run", "execute", "handler"]
        
        for pattern in patterns:
            symbols = self.graph.find_symbols(pattern, SymbolKind.FUNCTION)
            
            for sym in symbols:
                # Check if it's truly an entry point (no/few callers)
                callers = self.graph.get_callers(sym.fqn)
                if len(callers) <= 2:  # Main functions have few callers
                    entry_points.append(sym)
        
        # Also find API endpoints
        endpoints = self._find_api_endpoints()
        entry_points.extend(endpoints)
        
        # Remove duplicates
        seen = set()
        unique = []
        for ep in entry_points:
            if ep.fqn not in seen:
                seen.add(ep.fqn)
                unique.append(ep)
        
        return unique
    
    # ========== Helper Methods ==========
    
    def _infer_purpose(self, symbol: Symbol) -> str:
        """Infer function purpose from name and behavior"""
        name_lower = symbol.name.lower()
        
        # Check common patterns
        if "validate" in name_lower:
            return "Validates input or data according to business rules"
        elif "authenticate" in name_lower or "auth" in name_lower:
            return "Handles authentication or authorization"
        elif "parse" in name_lower:
            return "Parses data from one format to another"
        elif "fetch" in name_lower or "get" in name_lower:
            return "Retrieves data from a source"
        elif "save" in name_lower or "store" in name_lower:
            return "Persists data to storage"
        elif "process" in name_lower or "handle" in name_lower:
            return "Processes or handles specific business logic"
        elif "render" in name_lower or "display" in name_lower:
            return "Renders or displays information"
        elif "calculate" in name_lower or "compute" in name_lower:
            return "Performs calculations or computations"
        else:
            # Generic purpose based on callees
            callees = self.graph.get_callees(symbol.fqn, max_depth=1)
            if len(callees) > 5:
                return "Orchestrates multiple operations"
            elif len(callees) == 0:
                return "Performs a simple operation or returns a value"
            else:
                return "Performs specific business logic"
    
    def _extract_parameters(self, symbol: Symbol) -> List[Dict[str, Any]]:
        """Extract parameter information from signature"""
        if not symbol.signature:
            return []
        
        # Simple extraction (would need proper parsing)
        params = []
        if "(" in symbol.signature and ")" in symbol.signature:
            param_str = symbol.signature[symbol.signature.find("(")+1:symbol.signature.find(")")]
            if param_str:
                for param in param_str.split(","):
                    param = param.strip()
                    if ":" in param:
                        name, type_hint = param.split(":", 1)
                        params.append({"name": name.strip(), "type": type_hint.strip()})
                    else:
                        params.append({"name": param, "type": "Any"})
        
        return params
    
    def _extract_return_type(self, symbol: Symbol) -> Optional[str]:
        """Extract return type from signature"""
        if not symbol.signature:
            return None
        
        if "->" in symbol.signature:
            return symbol.signature.split("->")[1].strip()
        
        return None
    
    def _find_side_effects(self, symbol: str) -> List[str]:
        """Find side effects of a function"""
        side_effects = []
        
        callees = self.graph.get_callees(symbol, max_depth=2)
        
        for call_path in callees:
            for callee in call_path.path:
                name_lower = callee.name.lower()
                
                if any(x in name_lower for x in ["write", "save", "update", "delete", "insert"]):
                    side_effects.append(f"Modifies data via {callee.name}")
                elif any(x in name_lower for x in ["send", "post", "request"]):
                    side_effects.append(f"Makes external call via {callee.name}")
                elif any(x in name_lower for x in ["print", "log", "debug"]):
                    side_effects.append(f"Produces output via {callee.name}")
        
        return list(set(side_effects))  # Remove duplicates
    
    def _estimate_test_coverage(self, symbol: str) -> float:
        """Estimate test coverage for a symbol"""
        # Look for test files that might test this symbol
        tests = self._find_related_tests(symbol)
        
        if tests:
            # Has tests
            return 0.8  # Estimated
        else:
            # No tests found
            return 0.0
    
    def _find_related_tests(self, symbol: str) -> List[Symbol]:
        """Find test functions related to a symbol"""
        tests = []
        
        # Extract function name
        sym = self.graph.get_symbol(symbol)
        if not sym:
            return tests
        
        # Look for test functions with similar names
        test_patterns = [f"test_{sym.name}", f"{sym.name}_test", f"test{sym.name.capitalize()}"]
        
        for pattern in test_patterns:
            tests.extend(self.graph.find_symbols(pattern, SymbolKind.FUNCTION))
        
        return tests
    
    def _identify_features(self, symbols: set) -> List[str]:
        """Identify feature areas from symbol names"""
        features = set()
        
        feature_keywords = {
            "auth": "Authentication",
            "user": "User Management",
            "payment": "Payments",
            "order": "Order Processing",
            "product": "Product Catalog",
            "cart": "Shopping Cart",
            "email": "Email Service",
            "notification": "Notifications",
            "report": "Reporting",
            "admin": "Administration"
        }
        
        for symbol_fqn in symbols:
            for keyword, feature in feature_keywords.items():
                if keyword in symbol_fqn.lower():
                    features.add(feature)
        
        return list(features)
    
    def _calculate_risk(self, direct_count: int, transitive_count: int, test_count: int) -> float:
        """Calculate risk score for changes"""
        # Higher risk with more dependencies and fewer tests
        base_risk = min(1.0, (direct_count * 0.1 + transitive_count * 0.01))
        
        # Reduce risk if tests exist
        if test_count > 0:
            base_risk *= 0.7
        
        return min(1.0, base_risk)
    
    def _calculate_code_similarity(self, sym1: Symbol, sym2: Symbol) -> float:
        """Calculate similarity between two symbols"""
        # Name similarity
        name_sim = self._string_similarity(sym1.name, sym2.name)
        
        # Structural similarity (callees)
        callees1 = set(c.path[-1].name for c in self.graph.get_callees(sym1.fqn, max_depth=1))
        callees2 = set(c.path[-1].name for c in self.graph.get_callees(sym2.fqn, max_depth=1))
        
        if callees1 or callees2:
            struct_sim = len(callees1 & callees2) / len(callees1 | callees2)
        else:
            struct_sim = 0
        
        # Weighted average
        return name_sim * 0.3 + struct_sim * 0.7
    
    def _string_similarity(self, s1: str, s2: str) -> float:
        """Calculate string similarity"""
        s1_lower = s1.lower()
        s2_lower = s2.lower()
        
        # Exact match
        if s1_lower == s2_lower:
            return 1.0
        
        # Substring match
        if s1_lower in s2_lower or s2_lower in s1_lower:
            return 0.8
        
        # Token overlap
        tokens1 = set(s1_lower.replace("_", " ").split())
        tokens2 = set(s2_lower.replace("_", " ").split())
        
        if tokens1 and tokens2:
            return len(tokens1 & tokens2) / len(tokens1 | tokens2)
        
        return 0.0
    
    def _handles_user_input(self, symbol: str) -> bool:
        """Check if symbol handles user input"""
        sym = self.graph.get_symbol(symbol)
        if not sym:
            return False
        
        input_keywords = ["request", "input", "param", "arg", "query", "body", "form"]
        
        # Check name
        if any(keyword in sym.name.lower() for keyword in input_keywords):
            return True
        
        # Check parameters
        params = self._extract_parameters(sym)
        for param in params:
            if any(keyword in param["name"].lower() for keyword in input_keywords):
                return True
        
        return False
    
    def _accesses_database(self, symbol: str) -> bool:
        """Check if symbol accesses database"""
        callees = self.graph.get_callees(symbol, max_depth=3)
        
        db_keywords = ["query", "execute", "fetch", "insert", "update", "delete", "select"]
        
        for call_path in callees:
            for callee in call_path.path:
                if any(keyword in callee.name.lower() for keyword in db_keywords):
                    return True
        
        return False
    
    def _performs_auth(self, symbol: str) -> bool:
        """Check if symbol performs authentication"""
        sym = self.graph.get_symbol(symbol)
        if not sym:
            return False
        
        auth_keywords = ["auth", "login", "verify", "token", "permission", "role"]
        
        return any(keyword in sym.name.lower() for keyword in auth_keywords)
    
    def _uses_encryption(self, symbol: str) -> bool:
        """Check if symbol uses encryption"""
        callees = self.graph.get_callees(symbol, max_depth=2)
        
        crypto_keywords = ["encrypt", "decrypt", "hash", "cipher", "crypto", "sign"]
        
        for call_path in callees:
            for callee in call_path.path:
                if any(keyword in callee.name.lower() for keyword in crypto_keywords):
                    return True
        
        return False
    
    def _find_external_calls(self, symbol: str) -> List[str]:
        """Find external API calls"""
        external = []
        
        callees = self.graph.get_callees(symbol, max_depth=2)
        
        external_keywords = ["http", "request", "fetch", "api", "client", "send"]
        
        for call_path in callees:
            for callee in call_path.path:
                if any(keyword in callee.name.lower() for keyword in external_keywords):
                    external.append(callee.name)
        
        return list(set(external))
    
    def _find_vulnerabilities_in(self, symbol: str) -> List[SecurityIssue]:
        """Find vulnerabilities in a specific symbol"""
        vulnerabilities = []
        
        # Check for SQL injection
        if self._accesses_database(symbol) and self._handles_user_input(symbol):
            # Simplified check - would need deeper analysis
            vulnerabilities.extend(self.analyzer.find_sql_injections())
        
        # Filter to only this symbol
        return [v for v in vulnerabilities if symbol in v.description]
    
    def _determine_privilege_level(self, symbol: str) -> str:
        """Determine privilege level of a function"""
        sym = self.graph.get_symbol(symbol)
        if not sym:
            return "user"
        
        name_lower = sym.name.lower()
        
        if any(x in name_lower for x in ["admin", "superuser", "root"]):
            return "admin"
        elif any(x in name_lower for x in ["system", "internal", "private"]):
            return "system"
        else:
            return "user"
    
    def _find_api_endpoints(self) -> List[Symbol]:
        """Find API endpoint functions"""
        endpoints = []
        
        # Common endpoint patterns
        patterns = ["route", "api", "endpoint", "handler", "controller", "view"]
        
        for pattern in patterns:
            symbols = self.graph.find_symbols(pattern, SymbolKind.FUNCTION)
            
            # Filter to likely endpoints
            for sym in symbols:
                # Check if it looks like an endpoint
                if any(x in sym.name.lower() for x in ["get", "post", "put", "delete", "patch"]):
                    endpoints.append(sym)
                elif self._handles_user_input(sym.fqn):
                    endpoints.append(sym)
        
        return endpoints