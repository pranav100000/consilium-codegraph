"""
Tests for AgentHelpers class
"""

import pytest
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent_api.helpers import AgentHelpers
from agent_api.models import (
    Symbol, SymbolKind, Location, FunctionExplanation,
    ImpactAnalysis, SecurityContext, RefactoringSuggestion,
    ComplexityMetrics, SecurityIssue, Severity
)


class TestAgentHelpers:
    """Test AgentHelpers class"""
    
    def test_init(self, mock_repo, mock_db):
        """Test helpers initialization"""
        helpers = AgentHelpers(str(mock_repo))
        
        assert helpers.graph is not None
        assert helpers.analyzer is not None
    
    def test_explain_function(self, mock_repo, mock_db):
        """Test function explanation"""
        helpers = AgentHelpers(str(mock_repo))
        
        explanation = helpers.explain_function("AuthService::authenticate")
        
        assert isinstance(explanation, FunctionExplanation)
        assert explanation.symbol.fqn == "AuthService::authenticate"
        assert explanation.purpose is not None
        assert isinstance(explanation.parameters, list)
        assert isinstance(explanation.complexity, ComplexityMetrics)
        assert 0 <= explanation.test_coverage <= 1
    
    def test_explain_function_not_found(self, mock_repo, mock_db):
        """Test explanation for non-existent function"""
        helpers = AgentHelpers(str(mock_repo))
        
        with pytest.raises(ValueError, match="Symbol .* not found"):
            helpers.explain_function("NonExistent::function")
    
    def test_analyze_change_impact(self, mock_repo, mock_db):
        """Test change impact analysis"""
        helpers = AgentHelpers(str(mock_repo))
        
        impact = helpers.analyze_change_impact("Database::query")
        
        assert isinstance(impact, ImpactAnalysis)
        assert impact.symbol == "Database::query"
        assert isinstance(impact.direct_callers, list)
        assert isinstance(impact.transitive_impact, set)
        assert isinstance(impact.affected_tests, list)
        assert isinstance(impact.affected_features, list)
        assert 0 <= impact.risk_score <= 1
        
        # Should have callers
        assert len(impact.direct_callers) > 0
        assert impact.impact_radius > 0
    
    def test_find_similar_code(self, mock_repo, mock_db):
        """Test finding similar code"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Find functions similar to authenticate
        similar = helpers.find_similar_code("AuthService::authenticate", threshold=0.3)
        
        assert isinstance(similar, list)
        for sym in similar:
            assert isinstance(sym, Symbol)
            assert sym.fqn != "AuthService::authenticate"  # Should exclude self
    
    def test_find_similar_code_high_threshold(self, mock_repo, mock_db):
        """Test finding similar code with high threshold"""
        helpers = AgentHelpers(str(mock_repo))
        
        # With very high threshold, should find few or no matches
        similar = helpers.find_similar_code("main", threshold=0.95)
        
        assert len(similar) <= 2  # Should find very few matches
    
    def test_suggest_refactoring(self, mock_repo, mock_db):
        """Test refactoring suggestions"""
        helpers = AgentHelpers(str(mock_repo))
        
        suggestions = helpers.suggest_refactoring("complex_function")
        
        assert isinstance(suggestions, list)
        
        # Should suggest refactoring for function with 6 parameters
        param_suggestions = [s for s in suggestions 
                           if s.type == "introduce_parameter_object"]
        assert len(param_suggestions) > 0
        
        for suggestion in suggestions:
            assert isinstance(suggestion, RefactoringSuggestion)
            assert suggestion.benefit is not None
    
    def test_get_security_context(self, mock_repo, mock_db):
        """Test security context extraction"""
        helpers = AgentHelpers(str(mock_repo))
        
        context = helpers.get_security_context("AuthService::authenticate")
        
        assert isinstance(context, SecurityContext)
        assert context.symbol == "AuthService::authenticate"
        assert isinstance(context.handles_user_input, bool)
        assert isinstance(context.accesses_database, bool)
        assert isinstance(context.performs_auth, bool)
        assert isinstance(context.external_calls, list)
        assert context.privilege_level in ["user", "admin", "system"]
        
        # Authentication function should be security critical
        assert context.is_security_critical
    
    def test_get_security_context_not_found(self, mock_repo, mock_db):
        """Test security context for non-existent symbol"""
        helpers = AgentHelpers(str(mock_repo))
        
        with pytest.raises(ValueError, match="Symbol .* not found"):
            helpers.get_security_context("NonExistent")
    
    def test_get_code_summary(self, mock_repo, mock_db):
        """Test code file summary"""
        helpers = AgentHelpers(str(mock_repo))
        
        summary = helpers.get_code_summary("src/auth.py")
        
        assert isinstance(summary, dict)
        assert summary["file"] == "src/auth.py"
        assert summary["total_symbols"] == 4
        assert "symbol_counts" in summary
        assert "main_classes" in summary
        assert "main_functions" in summary
        assert "total_complexity" in summary
        assert "average_complexity" in summary
        
        # Should have AuthService class
        assert "AuthService" in summary["main_classes"]
    
    def test_find_entry_points(self, mock_repo, mock_db):
        """Test finding entry points"""
        helpers = AgentHelpers(str(mock_repo))
        
        entry_points = helpers.find_entry_points()
        
        assert isinstance(entry_points, list)
        
        # Should find main function
        main_funcs = [ep for ep in entry_points if "main" in ep.name.lower()]
        assert len(main_funcs) > 0
        
        for ep in entry_points:
            assert isinstance(ep, Symbol)
    
    def test_infer_purpose(self, mock_repo, mock_db):
        """Test function purpose inference"""
        helpers = AgentHelpers(str(mock_repo))
        
        auth_symbol = Symbol(
            fqn="validate_user",
            name="validate_user",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        purpose = helpers._infer_purpose(auth_symbol)
        assert "validate" in purpose.lower()
        
        parse_symbol = Symbol(
            fqn="parse_json",
            name="parse_json",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        
        purpose = helpers._infer_purpose(parse_symbol)
        assert "parse" in purpose.lower()
    
    def test_extract_parameters(self, mock_repo, mock_db):
        """Test parameter extraction"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Test with typed parameters
        typed_symbol = Symbol(
            fqn="test",
            name="test",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1),
            signature="def test(name: str, age: int, active: bool)"
        )
        
        params = helpers._extract_parameters(typed_symbol)
        assert len(params) == 3
        assert params[0]["name"] == "name"
        assert params[0]["type"] == "str"
        assert params[1]["name"] == "age"
        assert params[1]["type"] == "int"
        
        # Test with untyped parameters
        untyped_symbol = Symbol(
            fqn="test",
            name="test",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1),
            signature="def test(a, b, c)"
        )
        
        params = helpers._extract_parameters(untyped_symbol)
        assert len(params) == 3
        assert all(p["type"] == "Any" for p in params)
    
    def test_extract_return_type(self, mock_repo, mock_db):
        """Test return type extraction"""
        helpers = AgentHelpers(str(mock_repo))
        
        # With return type
        typed_symbol = Symbol(
            fqn="test",
            name="test",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1),
            signature="def test() -> bool"
        )
        
        return_type = helpers._extract_return_type(typed_symbol)
        assert return_type == "bool"
        
        # Without return type
        untyped_symbol = Symbol(
            fqn="test",
            name="test",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1),
            signature="def test()"
        )
        
        return_type = helpers._extract_return_type(untyped_symbol)
        assert return_type is None
    
    def test_find_side_effects(self, mock_repo, mock_db):
        """Test side effect detection"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Function that writes to database should have side effects
        side_effects = helpers._find_side_effects("AuthService::authenticate")
        
        assert isinstance(side_effects, list)
        # Should detect database modification via query
        db_effects = [e for e in side_effects if "data" in e.lower() or "query" in e.lower()]
        assert len(db_effects) > 0
    
    def test_estimate_test_coverage(self, mock_repo, mock_db):
        """Test test coverage estimation"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Add a test function
        conn = helpers.graph.conn
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO symbols (file_id, fqn, name, kind, line)
            VALUES (1, 'test_authenticate', 'test_authenticate', 'function', 100)
        """)
        conn.commit()
        
        # Should find test
        coverage = helpers._estimate_test_coverage("authenticate")
        assert coverage > 0
        
        # Function without tests
        coverage = helpers._estimate_test_coverage("complex_function")
        assert coverage == 0.0
    
    def test_identify_features(self, mock_repo, mock_db):
        """Test feature identification"""
        helpers = AgentHelpers(str(mock_repo))
        
        symbols = {
            "auth_service",
            "user_manager",
            "payment_processor",
            "order_handler"
        }
        
        features = helpers._identify_features(symbols)
        
        assert "Authentication" in features
        assert "User Management" in features
        assert "Payments" in features
        assert "Order Processing" in features
    
    def test_calculate_risk(self, mock_repo, mock_db):
        """Test risk calculation"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Low risk (few dependencies, has tests)
        low_risk = helpers._calculate_risk(
            direct_count=1,
            transitive_count=3,
            test_count=5
        )
        assert low_risk < 0.5
        
        # High risk (many dependencies, no tests)
        high_risk = helpers._calculate_risk(
            direct_count=10,
            transitive_count=50,
            test_count=0
        )
        assert high_risk > 0.5
        assert high_risk <= 1.0
    
    def test_string_similarity(self, mock_repo, mock_db):
        """Test string similarity calculation"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Exact match
        assert helpers._string_similarity("test", "test") == 1.0
        
        # Substring
        assert helpers._string_similarity("test", "testing") == 0.8
        
        # Token overlap
        sim = helpers._string_similarity("get_user_data", "fetch_user_info")
        assert 0 < sim < 1
        
        # No similarity
        assert helpers._string_similarity("abc", "xyz") == 0.0
    
    def test_handles_user_input(self, mock_repo, mock_db):
        """Test user input detection"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Should detect based on name
        assert helpers._handles_user_input("handle_request") is True
        assert helpers._handles_user_input("process_input") is True
        assert helpers._handles_user_input("calculate") is False
    
    def test_accesses_database(self, mock_repo, mock_db):
        """Test database access detection"""
        helpers = AgentHelpers(str(mock_repo))
        
        # authenticate calls Database::query
        assert helpers._accesses_database("AuthService::authenticate") is True
        
        # main doesn't directly access database
        # (it calls process_data which accesses database, but we check depth)
        assert helpers._accesses_database("main") is True  # Within depth 3
    
    def test_performs_auth(self, mock_repo, mock_db):
        """Test authentication detection"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Should detect auth functions
        assert helpers._performs_auth("AuthService::authenticate") is True
        assert helpers._performs_auth("check_permission") is False  # permission != auth
        assert helpers._performs_auth("main") is False
    
    def test_uses_encryption(self, mock_repo, mock_db):
        """Test encryption usage detection"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Add a crypto function
        conn = helpers.graph.conn
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO symbols (file_id, fqn, name, kind, line)
            VALUES (1, 'encrypt_data', 'encrypt_data', 'function', 200)
        """)
        cursor.execute("""
            INSERT INTO edges (src, dst, edge_type, resolution)
            VALUES ('AuthService::authenticate', 'encrypt_data', 'calls', 'syntactic')
        """)
        conn.commit()
        
        # Should detect crypto usage
        assert helpers._uses_encryption("AuthService::authenticate") is True
        assert helpers._uses_encryption("main") is False
    
    def test_find_external_calls(self, mock_repo, mock_db):
        """Test external call detection"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Add external calls
        conn = helpers.graph.conn
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO symbols (file_id, fqn, name, kind, line)
            VALUES (1, 'http_request', 'http_request', 'function', 300)
        """)
        cursor.execute("""
            INSERT INTO symbols (file_id, fqn, name, kind, line)
            VALUES (1, 'api_client_send', 'api_client_send', 'function', 310)
        """)
        cursor.execute("""
            INSERT INTO edges (src, dst, edge_type, resolution)
            VALUES ('main', 'http_request', 'calls', 'syntactic')
        """)
        cursor.execute("""
            INSERT INTO edges (src, dst, edge_type, resolution)
            VALUES ('main', 'api_client_send', 'calls', 'syntactic')
        """)
        conn.commit()
        
        external = helpers._find_external_calls("main")
        
        assert len(external) > 0
        assert "http_request" in external
        assert "api_client_send" in external
    
    def test_determine_privilege_level(self, mock_repo, mock_db):
        """Test privilege level determination"""
        helpers = AgentHelpers(str(mock_repo))
        
        # Admin function
        admin_sym = Symbol(
            fqn="admin_delete_user",
            name="admin_delete_user",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        assert helpers._determine_privilege_level("admin_delete_user") == "admin"
        
        # System function
        system_sym = Symbol(
            fqn="system_init",
            name="system_init",
            kind=SymbolKind.FUNCTION,
            location=Location(file="test.py", line=1)
        )
        assert helpers._determine_privilege_level("system_init") == "system"
        
        # Regular user function
        assert helpers._determine_privilege_level("get_profile") == "user"