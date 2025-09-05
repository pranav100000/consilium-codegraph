"""
Data models for the Code Graph API
"""

from dataclasses import dataclass
from enum import Enum
from typing import List, Optional, Set, Dict, Any
from pathlib import Path


class SymbolKind(Enum):
    """Types of symbols in the code graph"""
    FUNCTION = "function"
    METHOD = "method"
    CLASS = "class"
    INTERFACE = "interface"
    VARIABLE = "variable"
    TYPE = "type"
    MODULE = "module"
    PACKAGE = "package"
    NAMESPACE = "namespace"
    ENUM = "enum"
    ENUM_MEMBER = "enum_member"
    STRUCT = "struct"
    TRAIT = "trait"
    CONSTANT = "constant"
    FIELD = "field"
    PROPERTY = "property"
    
    @classmethod
    def _missing_(cls, value):
        """Handle unknown symbol kinds gracefully"""
        # Try case-insensitive match
        for member in cls:
            if member.value.lower() == value.lower():
                return member
        # Default to FUNCTION for unknown types
        return cls.FUNCTION


class EdgeType(Enum):
    """Types of relationships between symbols"""
    CONTAINS = "contains"
    DECLARES = "declares"
    CALLS = "calls"
    IMPORTS = "imports"
    EXTENDS = "extends"
    IMPLEMENTS = "implements"
    USES = "uses"
    RETURNS = "returns"
    THROWS = "throws"
    OVERRIDES = "overrides"


class Severity(Enum):
    """Issue severity levels"""
    CRITICAL = "critical"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"
    INFO = "info"


class AnalysisQuality(Enum):
    """Quality level of analysis"""
    SEMANTIC = "semantic"  # Full semantic analysis (e.g., from Joern)
    SYNTACTIC = "syntactic"  # Syntax-only analysis (e.g., from Tree-sitter)
    HEURISTIC = "heuristic"  # Pattern-based approximation


@dataclass
class Location:
    """Location in source code"""
    file: str
    line: int
    column: Optional[int] = None
    end_line: Optional[int] = None
    end_column: Optional[int] = None


@dataclass
class Symbol:
    """A symbol in the code graph"""
    fqn: str  # Fully qualified name
    name: str
    kind: SymbolKind
    location: Location
    signature: Optional[str] = None
    docstring: Optional[str] = None
    analyzer: str = "unknown"  # Which tool analyzed this
    confidence: float = 1.0
    metadata: Dict[str, Any] = None

    def __post_init__(self):
        if self.metadata is None:
            self.metadata = {}


@dataclass
class CallPath:
    """A path through function calls"""
    path: List[Symbol]
    depth: int
    is_recursive: bool = False
    confidence: float = 1.0


@dataclass
class DataFlow:
    """Data flow from source to sink"""
    source: Symbol
    sink: Symbol
    path: List[Symbol]
    is_tainted: bool
    is_sanitized: bool
    confidence: float
    analysis_quality: AnalysisQuality


@dataclass
class SecurityIssue:
    """A security vulnerability or concern"""
    issue_id: str
    type: str
    severity: Severity
    location: Location
    description: str
    evidence: List[str]  # Code snippets showing the issue
    fix_suggestion: str
    confidence: float
    cwe_id: Optional[str] = None
    owasp_category: Optional[str] = None
    false_positive: bool = False


@dataclass
class ComplexityMetrics:
    """Code complexity measurements"""
    cyclomatic: int
    cognitive: int
    lines_of_code: int
    nesting_depth: int
    parameter_count: int
    return_points: int
    
    @property
    def is_complex(self) -> bool:
        """Check if complexity exceeds thresholds"""
        return self.cyclomatic > 10 or self.cognitive > 15


@dataclass
class CodeSmell:
    """Code quality issue"""
    type: str
    location: Location
    description: str
    impact: str  # How this affects code quality
    refactoring_suggestion: str
    severity: Severity
    metrics: Optional[Dict[str, Any]] = None


@dataclass
class DuplicateCode:
    """Duplicate code detection result"""
    locations: List[Location]
    lines: int
    tokens: int
    similarity: float
    code_snippet: str


@dataclass
class ImpactAnalysis:
    """Analysis of change impact"""
    symbol: str
    direct_callers: List[Symbol]
    transitive_impact: Set[str]  # FQNs of all affected symbols
    affected_tests: List[Symbol]
    affected_features: List[str]
    risk_score: float
    
    @property
    def impact_radius(self) -> int:
        """Total number of affected symbols"""
        return len(self.transitive_impact)


@dataclass
class DependencyGraph:
    """Dependency relationships"""
    root: Symbol
    dependencies: Dict[str, List[str]]  # FQN -> List of dependency FQNs
    dependents: Dict[str, List[str]]  # FQN -> List of dependent FQNs
    cycles: List[List[str]]  # Circular dependencies
    
    def has_cycles(self) -> bool:
        return len(self.cycles) > 0


@dataclass
class FunctionExplanation:
    """High-level explanation of a function"""
    symbol: Symbol
    purpose: str
    parameters: List[Dict[str, Any]]
    returns: Optional[str]
    side_effects: List[str]
    complexity: ComplexityMetrics
    test_coverage: float
    dependencies: List[str]
    
    @property
    def needs_refactoring(self) -> bool:
        """Check if function needs refactoring"""
        return (self.complexity.is_complex or 
                self.test_coverage < 0.5 or 
                len(self.side_effects) > 3)


@dataclass
class SecurityContext:
    """Security-relevant information about a symbol"""
    symbol: str
    handles_user_input: bool
    accesses_database: bool
    performs_auth: bool
    performs_crypto: bool
    external_calls: List[str]
    vulnerabilities: List[SecurityIssue]
    privilege_level: str  # "user", "admin", "system"
    
    @property
    def is_security_critical(self) -> bool:
        """Check if this requires security review"""
        return (self.handles_user_input or 
                self.performs_auth or 
                self.performs_crypto or 
                self.privilege_level in ["admin", "system"])


@dataclass
class RefactoringSuggestion:
    """Suggested code improvement"""
    type: str  # "extract_method", "rename", "simplify", etc.
    location: Location
    description: str
    benefit: str
    example: Optional[str] = None
    automated: bool = False  # Can be applied automatically


@dataclass
class LayerViolation:
    """Architecture layer violation"""
    from_layer: str
    to_layer: str
    from_symbol: str
    to_symbol: str
    violation_type: str  # "skip_layer", "reverse_dependency", etc.
    suggested_path: List[str]  # Correct path through layers


@dataclass
class ReviewResult:
    """Complete review result from an agent"""
    agent_name: str
    issues: List[SecurityIssue]
    code_smells: List[CodeSmell]
    suggestions: List[RefactoringSuggestion]
    metrics: Dict[str, Any]
    summary: str
    confidence: float