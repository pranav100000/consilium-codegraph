"""
Tests for data models
"""

import pytest
from agent_api.models import (
    Symbol, SymbolKind, Location, EdgeType, Severity,
    AnalysisQuality, ComplexityMetrics, SecurityIssue,
    ImpactAnalysis, FunctionExplanation, SecurityContext,
    DependencyGraph, CodeSmell, RefactoringSuggestion
)


class TestLocation:
    """Test Location model"""
    
    def test_location_creation(self):
        loc = Location(file="test.py", line=10, column=5)
        assert loc.file == "test.py"
        assert loc.line == 10
        assert loc.column == 5
        assert loc.end_line is None
        assert loc.end_column is None
    
    def test_location_with_range(self):
        loc = Location(
            file="test.py", 
            line=10, 
            column=5,
            end_line=15,
            end_column=20
        )
        assert loc.end_line == 15
        assert loc.end_column == 20


class TestSymbol:
    """Test Symbol model"""
    
    def test_symbol_creation(self):
        sym = Symbol(
            fqn="MyClass::my_method",
            name="my_method",
            kind=SymbolKind.METHOD,
            location=Location(file="test.py", line=10)
        )
        assert sym.fqn == "MyClass::my_method"
        assert sym.name == "my_method"
        assert sym.kind == SymbolKind.METHOD
        assert sym.location.file == "test.py"
        assert sym.confidence == 1.0
        assert sym.metadata == {}
    
    def test_symbol_with_metadata(self):
        sym = Symbol(
            fqn="test_func",
            name="test_func",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1),
            metadata={"test": True, "complexity": 5}
        )
        assert sym.metadata["test"] is True
        assert sym.metadata["complexity"] == 5
    
    def test_symbol_kinds(self):
        """Test all symbol kinds"""
        for kind in SymbolKind:
            sym = Symbol(
                fqn=f"test_{kind.value}",
                name=f"test_{kind.value}",
                kind=kind,
                location=Location(file="test.py", line=1)
            )
            assert sym.kind == kind


class TestComplexityMetrics:
    """Test ComplexityMetrics model"""
    
    def test_complexity_creation(self):
        metrics = ComplexityMetrics(
            cyclomatic=15,
            cognitive=20,
            lines_of_code=100,
            nesting_depth=4,
            parameter_count=6,
            return_points=3
        )
        assert metrics.cyclomatic == 15
        assert metrics.cognitive == 20
        assert metrics.lines_of_code == 100
    
    def test_is_complex_property(self):
        # Not complex
        simple = ComplexityMetrics(
            cyclomatic=5,
            cognitive=10,
            lines_of_code=50,
            nesting_depth=2,
            parameter_count=3,
            return_points=1
        )
        assert simple.is_complex is False
        
        # Complex due to cyclomatic complexity
        complex_cyclo = ComplexityMetrics(
            cyclomatic=15,
            cognitive=10,
            lines_of_code=50,
            nesting_depth=2,
            parameter_count=3,
            return_points=1
        )
        assert complex_cyclo.is_complex is True
        
        # Complex due to cognitive complexity
        complex_cognitive = ComplexityMetrics(
            cyclomatic=5,
            cognitive=20,
            lines_of_code=50,
            nesting_depth=2,
            parameter_count=3,
            return_points=1
        )
        assert complex_cognitive.is_complex is True


class TestSecurityIssue:
    """Test SecurityIssue model"""
    
    def test_security_issue_creation(self):
        issue = SecurityIssue(
            issue_id="sql_001",
            type="sql_injection",
            severity=Severity.CRITICAL,
            location=Location(file="test.py", line=10),
            description="SQL injection vulnerability",
            evidence=["query = 'SELECT * FROM users WHERE id = ' + user_id"],
            fix_suggestion="Use parameterized queries",
            confidence=0.95
        )
        assert issue.issue_id == "sql_001"
        assert issue.severity == Severity.CRITICAL
        assert issue.confidence == 0.95
        assert issue.false_positive is False
    
    def test_security_issue_with_cwe(self):
        issue = SecurityIssue(
            issue_id="xss_001",
            type="xss",
            severity=Severity.HIGH,
            location=Location(file="test.py", line=20),
            description="Cross-site scripting vulnerability",
            evidence=["return '<div>' + user_input + '</div>'"],
            fix_suggestion="Escape user input",
            confidence=0.85,
            cwe_id="CWE-79",
            owasp_category="A03:2021"
        )
        assert issue.cwe_id == "CWE-79"
        assert issue.owasp_category == "A03:2021"
    
    def test_severity_levels(self):
        """Test all severity levels"""
        for severity in Severity:
            issue = SecurityIssue(
                issue_id=f"test_{severity.value}",
                type="test",
                severity=severity,
                location=Location(file="test.py", line=1),
                description="Test issue",
                evidence=[],
                fix_suggestion="Fix it",
                confidence=1.0
            )
            assert issue.severity == severity


class TestImpactAnalysis:
    """Test ImpactAnalysis model"""
    
    def test_impact_analysis_creation(self):
        impact = ImpactAnalysis(
            symbol="test_function",
            direct_callers=[],
            transitive_impact={"func1", "func2", "func3"},
            affected_tests=[],
            affected_features=["Authentication", "Payments"],
            risk_score=0.7
        )
        assert impact.symbol == "test_function"
        assert len(impact.transitive_impact) == 3
        assert impact.risk_score == 0.7
    
    def test_impact_radius_property(self):
        impact = ImpactAnalysis(
            symbol="test",
            direct_callers=[],
            transitive_impact={"a", "b", "c", "d", "e"},
            affected_tests=[],
            affected_features=[],
            risk_score=0.5
        )
        assert impact.impact_radius == 5


class TestDependencyGraph:
    """Test DependencyGraph model"""
    
    def test_dependency_graph_creation(self):
        sym = Symbol(
            fqn="test",
            name="test",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        graph = DependencyGraph(
            root=sym,
            dependencies={"test": ["dep1", "dep2"]},
            dependents={"test": ["caller1"]},
            cycles=[]
        )
        assert graph.root.fqn == "test"
        assert len(graph.dependencies["test"]) == 2
        assert graph.has_cycles() is False
    
    def test_has_cycles(self):
        sym = Symbol(
            fqn="test",
            name="test",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        graph = DependencyGraph(
            root=sym,
            dependencies={},
            dependents={},
            cycles=[["a", "b", "c", "a"]]
        )
        assert graph.has_cycles() is True


class TestFunctionExplanation:
    """Test FunctionExplanation model"""
    
    def test_function_explanation_creation(self):
        sym = Symbol(
            fqn="complex_func",
            name="complex_func",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        explanation = FunctionExplanation(
            symbol=sym,
            purpose="Processes user data",
            parameters=[{"name": "data", "type": "str"}],
            returns="ProcessedData",
            side_effects=["Writes to database", "Sends email"],
            complexity=ComplexityMetrics(15, 20, 100, 4, 6, 3),
            test_coverage=0.3,
            dependencies=["database", "email_service"]
        )
        assert explanation.purpose == "Processes user data"
        assert len(explanation.side_effects) == 2
        assert explanation.test_coverage == 0.3
    
    def test_needs_refactoring_property(self):
        sym = Symbol(
            fqn="test",
            name="test",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        # Needs refactoring due to complexity
        complex_func = FunctionExplanation(
            symbol=sym,
            purpose="Test",
            parameters=[],
            returns=None,
            side_effects=[],
            complexity=ComplexityMetrics(15, 10, 100, 4, 3, 2),
            test_coverage=0.8,
            dependencies=[]
        )
        assert complex_func.needs_refactoring is True
        
        # Needs refactoring due to low test coverage
        untested_func = FunctionExplanation(
            symbol=sym,
            purpose="Test",
            parameters=[],
            returns=None,
            side_effects=[],
            complexity=ComplexityMetrics(5, 5, 50, 2, 3, 1),
            test_coverage=0.3,
            dependencies=[]
        )
        assert untested_func.needs_refactoring is True
        
        # Needs refactoring due to many side effects
        side_effect_func = FunctionExplanation(
            symbol=sym,
            purpose="Test",
            parameters=[],
            returns=None,
            side_effects=["a", "b", "c", "d"],
            complexity=ComplexityMetrics(5, 5, 50, 2, 3, 1),
            test_coverage=0.8,
            dependencies=[]
        )
        assert side_effect_func.needs_refactoring is True
        
        # Good function
        good_func = FunctionExplanation(
            symbol=sym,
            purpose="Test",
            parameters=[],
            returns=None,
            side_effects=["logs"],
            complexity=ComplexityMetrics(5, 5, 50, 2, 3, 1),
            test_coverage=0.8,
            dependencies=[]
        )
        assert good_func.needs_refactoring is False


class TestSecurityContext:
    """Test SecurityContext model"""
    
    def test_security_context_creation(self):
        context = SecurityContext(
            symbol="auth_handler",
            handles_user_input=True,
            accesses_database=True,
            performs_auth=True,
            performs_crypto=False,
            external_calls=["payment_api", "email_service"],
            vulnerabilities=[],
            privilege_level="admin"
        )
        assert context.symbol == "auth_handler"
        assert context.handles_user_input is True
        assert len(context.external_calls) == 2
        assert context.privilege_level == "admin"
    
    def test_is_security_critical_property(self):
        # Critical due to user input
        input_handler = SecurityContext(
            symbol="test",
            handles_user_input=True,
            accesses_database=False,
            performs_auth=False,
            performs_crypto=False,
            external_calls=[],
            vulnerabilities=[],
            privilege_level="user"
        )
        assert input_handler.is_security_critical is True
        
        # Critical due to authentication
        auth_handler = SecurityContext(
            symbol="test",
            handles_user_input=False,
            accesses_database=False,
            performs_auth=True,
            performs_crypto=False,
            external_calls=[],
            vulnerabilities=[],
            privilege_level="user"
        )
        assert auth_handler.is_security_critical is True
        
        # Critical due to crypto
        crypto_handler = SecurityContext(
            symbol="test",
            handles_user_input=False,
            accesses_database=False,
            performs_auth=False,
            performs_crypto=True,
            external_calls=[],
            vulnerabilities=[],
            privilege_level="user"
        )
        assert crypto_handler.is_security_critical is True
        
        # Critical due to privilege level
        admin_handler = SecurityContext(
            symbol="test",
            handles_user_input=False,
            accesses_database=False,
            performs_auth=False,
            performs_crypto=False,
            external_calls=[],
            vulnerabilities=[],
            privilege_level="admin"
        )
        assert admin_handler.is_security_critical is True
        
        # Not critical
        normal_handler = SecurityContext(
            symbol="test",
            handles_user_input=False,
            accesses_database=True,  # DB access alone doesn't make it critical
            performs_auth=False,
            performs_crypto=False,
            external_calls=[],
            vulnerabilities=[],
            privilege_level="user"
        )
        assert normal_handler.is_security_critical is False


class TestCodeSmell:
    """Test CodeSmell model"""
    
    def test_code_smell_creation(self):
        smell = CodeSmell(
            type="long_method",
            location=Location(file="test.py", line=10),
            description="Method is too long",
            impact="Harder to understand and maintain",
            refactoring_suggestion="Extract into smaller methods",
            severity=Severity.MEDIUM,
            metrics={"lines": 200, "complexity": 25}
        )
        assert smell.type == "long_method"
        assert smell.severity == Severity.MEDIUM
        assert smell.metrics["lines"] == 200


class TestRefactoringSuggestion:
    """Test RefactoringSuggestion model"""
    
    def test_refactoring_suggestion_creation(self):
        suggestion = RefactoringSuggestion(
            type="extract_method",
            location=Location(file="test.py", line=50),
            description="Extract complex logic into separate method",
            benefit="Improved readability and testability",
            example="def calculate_discount(price, customer_type): ...",
            automated=False
        )
        assert suggestion.type == "extract_method"
        assert suggestion.automated is False
        assert suggestion.example is not None