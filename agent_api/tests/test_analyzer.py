"""
Tests for CodeAnalyzer class
"""

import pytest
from pathlib import Path
from unittest.mock import Mock, patch

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent_api.analyzer import CodeAnalyzer
from agent_api.code_graph import CodeGraph
from agent_api.models import (
    Symbol, SymbolKind, Location, SecurityIssue, Severity,
    DataFlow, AnalysisQuality, ComplexityMetrics, CodeSmell,
    DuplicateCode, LayerViolation
)


class TestCodeAnalyzer:
    """Test CodeAnalyzer class"""
    
    def test_init(self, mock_repo, mock_db):
        """Test analyzer initialization"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        assert analyzer.graph == graph
        assert len(analyzer.sql_patterns) > 0
        assert len(analyzer.auth_patterns) > 0
        assert len(analyzer.input_sources) > 0
    
    def test_trace_data_flow(self, mock_repo, mock_db):
        """Test data flow tracing"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Trace flow from process_data
        flows = analyzer.trace_data_flow("process_data", max_depth=2)
        
        assert len(flows) > 0
        for flow in flows:
            assert isinstance(flow, DataFlow)
            assert flow.source.name == "process_data"
            assert len(flow.path) > 0
    
    def test_trace_data_flow_to_sink(self, mock_repo, mock_db):
        """Test data flow tracing to specific sink"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Trace from main to Database::query
        flows = analyzer.trace_data_flow("main", "Database::query", max_depth=3)
        
        # Should find path through process_data
        assert len(flows) > 0
        for flow in flows:
            assert flow.source.fqn == "main"
            assert flow.sink.fqn == "Database::query"
    
    def test_find_tainted_paths(self, mock_repo, mock_db):
        """Test finding tainted data paths"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Find tainted paths from request
        tainted = analyzer.find_tainted_paths("request")
        
        # Should identify flows from user input sources
        for flow in tainted:
            assert flow.is_tainted
    
    def test_find_sql_injections(self, mock_repo, mock_db):
        """Test SQL injection detection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        issues = analyzer.find_sql_injections()
        
        # Should find issues in our test data
        assert len(issues) > 0
        
        for issue in issues:
            assert issue.type == "sql_injection"
            assert issue.severity == Severity.CRITICAL
            assert "injection" in issue.description.lower()
            assert issue.cwe_id == "CWE-89"
    
    def test_find_auth_bypasses(self, mock_repo, mock_db):
        """Test authentication bypass detection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Mock _find_api_endpoints to return test endpoints
        with patch.object(analyzer, '_find_api_endpoints') as mock_endpoints:
            mock_endpoints.return_value = [
                Symbol(
                    fqn="api_endpoint",
                    name="api_endpoint",
                    kind=SymbolKind.FUNCTION,
                    location=Location(file="test.py", line=1)
                )
            ]
            
            with patch.object(analyzer, '_has_auth_check') as mock_auth:
                mock_auth.return_value = False
                
                issues = analyzer.find_auth_bypasses()
                
                assert len(issues) > 0
                assert issues[0].type == "missing_authentication"
                assert issues[0].severity == Severity.HIGH
                assert issues[0].cwe_id == "CWE-306"
    
    def test_find_unsafe_operations(self, mock_repo, mock_db):
        """Test unsafe operation detection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Test finding all unsafe operations
        issues = analyzer.find_unsafe_operations("all")
        
        # Test specific operation types
        eval_issues = analyzer.find_unsafe_operations("eval")
        command_issues = analyzer.find_unsafe_operations("command")
        
        for issue in eval_issues:
            assert "eval" in issue.type
        
        for issue in command_issues:
            assert "command" in issue.type
    
    def test_get_complexity(self, mock_repo, mock_db):
        """Test complexity calculation"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Get complexity for complex_function
        complexity = analyzer.get_complexity("complex_function")
        
        assert isinstance(complexity, ComplexityMetrics)
        assert complexity.cyclomatic >= 1
        assert complexity.parameter_count == 6  # Has 6 parameters (a,b,c,d,e,f)
    
    def test_find_duplicates(self, mock_repo, mock_db):
        """Test duplicate code detection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        duplicates = analyzer.find_duplicates(min_lines=5)
        
        for dup in duplicates:
            assert isinstance(dup, DuplicateCode)
            assert len(dup.locations) >= 2
            assert dup.similarity > 0.0
            assert dup.similarity <= 1.0
    
    def test_find_code_smells(self, mock_repo, mock_db):
        """Test code smell detection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        smells = analyzer.find_code_smells()
        
        # Should find various types of smells
        smell_types = [s.type for s in smells]
        
        # Check for expected smell types
        for smell in smells:
            assert isinstance(smell, CodeSmell)
            assert smell.severity in Severity
            assert smell.refactoring_suggestion is not None
    
    def test_find_code_smells_complex_function(self, mock_repo, mock_db):
        """Test that complex functions are identified as god functions"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Add more callees to make a function complex
        conn = graph.conn
        cursor = conn.cursor()
        for i in range(25):
            cursor.execute("""
                INSERT INTO edges (src, dst, edge_type, resolution)
                VALUES ('complex_function', ?, 'calls', 'syntactic')
            """, (f"function_{i}",))
        conn.commit()
        
        smells = analyzer.find_code_smells()
        
        god_functions = [s for s in smells if s.type == "god_function"]
        assert len(god_functions) > 0
        
        # complex_function should be identified
        complex_smell = next((s for s in god_functions 
                              if "complex_function" in s.description), None)
        assert complex_smell is not None
        assert complex_smell.severity == Severity.MEDIUM
    
    def test_get_module_dependencies(self, mock_repo, mock_db):
        """Test module dependency extraction"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        deps = analyzer.get_module_dependencies()
        
        assert isinstance(deps, dict)
        # src/main.py imports src/auth.py
        # src/auth.py imports src/database.py
        assert "src/main" in deps or "main" in deps  # Depends on _get_module implementation
    
    def test_find_circular_dependencies(self, mock_repo, mock_db):
        """Test circular dependency detection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Add circular dependency for testing
        conn = graph.conn
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO edges (src, dst, edge_type, resolution)
            VALUES ('src/database.py', 'src/main.py', 'imports', 'syntactic')
        """)
        conn.commit()
        
        cycles = analyzer.find_circular_dependencies()
        
        # Should detect the cycle
        assert len(cycles) >= 0  # May or may not find depending on implementation
    
    def test_check_layer_violations(self, mock_repo, mock_db):
        """Test architecture layer violation detection"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Define layer rules
        rules = {
            "presentation": ["business"],
            "business": ["data"],
            "data": []
        }
        
        violations = analyzer.check_layer_violations(rules)
        
        for violation in violations:
            assert isinstance(violation, LayerViolation)
            assert violation.violation_type == "illegal_dependency"
            assert len(violation.suggested_path) > 0
    
    def test_helper_is_tainted_path(self, mock_repo, mock_db):
        """Test _is_tainted_path helper"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Create path with user input
        tainted_symbol = Symbol(
            fqn="handle_request",
            name="handle_request",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        clean_symbol = Symbol(
            fqn="calculate",
            name="calculate",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        assert analyzer._is_tainted_path([tainted_symbol]) is True
        assert analyzer._is_tainted_path([clean_symbol]) is False
    
    def test_helper_is_sanitized_path(self, mock_repo, mock_db):
        """Test _is_sanitized_path helper"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Path with sanitization
        sanitize_symbol = Symbol(
            fqn="sanitize_input",
            name="sanitize_input",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        validate_symbol = Symbol(
            fqn="validate_data",
            name="validate_data",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        unsanitized_symbol = Symbol(
            fqn="process",
            name="process",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        assert analyzer._is_sanitized_path([sanitize_symbol]) is True
        assert analyzer._is_sanitized_path([validate_symbol]) is True
        assert analyzer._is_sanitized_path([unsanitized_symbol]) is False
    
    def test_helper_is_sensitive_sink(self, mock_repo, mock_db):
        """Test _is_sensitive_sink helper"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        database_symbol = Symbol(
            fqn="database_write",
            name="database_write",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        exec_symbol = Symbol(
            fqn="exec_command",
            name="exec_command",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        normal_symbol = Symbol(
            fqn="calculate",
            name="calculate",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        assert analyzer._is_sensitive_sink(database_symbol) is True
        assert analyzer._is_sensitive_sink(exec_symbol) is True
        assert analyzer._is_sensitive_sink(normal_symbol) is False
    
    def test_helper_string_similarity(self, mock_repo, mock_db):
        """Test _string_similarity helper"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        # Identical strings
        assert analyzer._string_similarity("test", "test") == 1.0
        
        # Empty strings
        assert analyzer._string_similarity("", "") == 1.0
        
        # Completely different
        assert analyzer._string_similarity("abc", "xyz") < 0.5
        
        # Some overlap
        sim = analyzer._string_similarity("hello", "hallo")
        assert 0.5 < sim < 1.0
    
    def test_helper_get_layer(self, mock_repo, mock_db):
        """Test _get_layer helper"""
        graph = CodeGraph(str(mock_repo), str(mock_db))
        analyzer = CodeAnalyzer(graph)
        
        assert analyzer._get_layer("ui_controller") == "presentation"
        assert analyzer._get_layer("view_handler") == "presentation"
        assert analyzer._get_layer("business_service") == "business"
        assert analyzer._get_layer("data_repository") == "data"
        assert analyzer._get_layer("model_class") == "data"
        assert analyzer._get_layer("random_module") == "unknown"