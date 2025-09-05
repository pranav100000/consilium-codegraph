"""
Security analysis for code review agents
"""

import re
from dataclasses import dataclass
from typing import List, Optional, Dict, Any
from enum import Enum

from ..simple_api import CodeGraphAPI, Symbol


class Severity(Enum):
    CRITICAL = "critical"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"
    INFO = "info"


@dataclass
class SecurityIssue:
    """A security vulnerability or concern"""
    type: str
    severity: Severity
    symbol: str
    file: str
    line: int
    description: str
    evidence: List[str]
    fix_suggestion: str
    confidence: float
    cwe_id: Optional[str] = None


class SecurityAnalyzer:
    """
    Analyzes code for security vulnerabilities.
    Focused on practical issues that matter in code review.
    """
    
    def __init__(self, api: CodeGraphAPI):
        """
        Initialize with a CodeGraphAPI instance.
        
        Args:
            api: CodeGraphAPI for accessing the code graph
        """
        self.api = api
        self._init_patterns()
    
    def _init_patterns(self):
        """Initialize security patterns to look for"""
        # SQL injection patterns
        self.sql_concat_patterns = [
            r'\+.*["\'].*SELECT',
            r'\+.*["\'].*INSERT', 
            r'\+.*["\'].*UPDATE',
            r'\+.*["\'].*DELETE',
            r'\.format\(.*\).*(?:SELECT|INSERT|UPDATE|DELETE)',
            r'f["\'].*SELECT.*\{',  # f-strings with SQL
        ]
        
        # Command injection patterns
        self.command_patterns = [
            r'exec\s*\(',
            r'eval\s*\(',
            r'system\s*\(',
            r'popen\s*\(',
            r'subprocess\.(?:call|run|Popen)\s*\(',
        ]
        
        # Path traversal patterns
        self.path_patterns = [
            r'\.\./',  # Directory traversal
            r'\.\.\\',  # Windows traversal
        ]
        
        # Hardcoded secrets patterns
        self.secret_patterns = [
            r'(?:password|passwd|pwd)\s*=\s*["\'][^"\']+["\']',
            r'(?:api[_-]?key|apikey)\s*=\s*["\'][^"\']+["\']',
            r'(?:secret|token)\s*=\s*["\'][^"\']+["\']',
        ]
    
    def analyze_file(self, file_path: str) -> List[SecurityIssue]:
        """
        Analyze all symbols in a file for security issues.
        
        Args:
            file_path: Path to the file to analyze
            
        Returns:
            List of security issues found
        """
        issues = []
        symbols = self.api.get_file_symbols(file_path)
        
        for symbol in symbols:
            issues.extend(self.analyze_symbol(symbol))
        
        return issues
    
    def analyze_symbol(self, symbol: Symbol) -> List[SecurityIssue]:
        """
        Analyze a specific symbol for security issues.
        
        Args:
            symbol: Symbol to analyze
            
        Returns:
            List of security issues found
        """
        issues = []
        
        # Check for SQL injection
        if self._accesses_database(symbol.fqn):
            sql_issues = self._check_sql_injection(symbol)
            issues.extend(sql_issues)
        
        # Check for command injection
        cmd_issues = self._check_command_injection(symbol)
        issues.extend(cmd_issues)
        
        # Check for hardcoded secrets
        secret_issues = self._check_hardcoded_secrets(symbol)
        issues.extend(secret_issues)
        
        # Check for missing input validation
        if self._handles_user_input(symbol.fqn):
            validation_issues = self._check_input_validation(symbol)
            issues.extend(validation_issues)
        
        return issues
    
    def find_sql_injections(self) -> List[SecurityIssue]:
        """
        Find potential SQL injection vulnerabilities in the entire codebase.
        
        Returns:
            List of SQL injection issues
        """
        issues = []
        
        # Find functions that access database
        db_functions = self.api.find_symbols("query", kind="method")
        db_functions.extend(self.api.find_symbols("execute", kind="method"))
        db_functions.extend(self.api.find_symbols("sql", kind="function"))
        
        for func in db_functions:
            # Check if function receives user input through its callers
            callers = self.api.get_callers(func.fqn)
            
            for caller in callers:
                if self._handles_user_input(caller):
                    # Check for string concatenation in SQL
                    issue = SecurityIssue(
                        type="sql_injection",
                        severity=Severity.CRITICAL,
                        symbol=func.fqn,
                        file=func.file,
                        line=func.line,
                        description=f"Potential SQL injection in {func.name}. User input from {caller} may flow to database query.",
                        evidence=[f"Call chain: {caller} -> {func.fqn}"],
                        fix_suggestion="Use parameterized queries or prepared statements instead of string concatenation",
                        confidence=0.8,
                        cwe_id="CWE-89"
                    )
                    issues.append(issue)
        
        return issues
    
    def find_command_injections(self) -> List[SecurityIssue]:
        """
        Find potential command injection vulnerabilities.
        
        Returns:
            List of command injection issues
        """
        issues = []
        
        # Find functions that execute commands
        for pattern in ["exec", "eval", "system", "popen", "subprocess"]:
            dangerous_functions = self.api.find_symbols(pattern)
            
            for func in dangerous_functions:
                # Check if it receives external input
                callers = self.api.get_callers(func.fqn)
                
                for caller in callers:
                    if self._handles_user_input(caller):
                        issue = SecurityIssue(
                            type="command_injection",
                            severity=Severity.CRITICAL,
                            symbol=func.fqn,
                            file=func.file,
                            line=func.line,
                            description=f"Potential command injection in {func.name}. User input may be executed as system command.",
                            evidence=[f"Dangerous function: {func.name}", f"Called by: {caller}"],
                            fix_suggestion="Validate and sanitize all user input. Use subprocess with shell=False and pass arguments as a list.",
                            confidence=0.75,
                            cwe_id="CWE-78"
                        )
                        issues.append(issue)
        
        return issues
    
    def find_missing_auth_checks(self) -> List[SecurityIssue]:
        """
        Find API endpoints or sensitive functions without authentication checks.
        
        Returns:
            List of missing authentication issues
        """
        issues = []
        
        # Find potential API endpoints
        endpoints = self._find_api_endpoints()
        
        for endpoint in endpoints:
            # Check if endpoint has auth check
            if not self._has_auth_check(endpoint):
                issue = SecurityIssue(
                    type="missing_authentication",
                    severity=Severity.HIGH,
                    symbol=endpoint.fqn,
                    file=endpoint.file,
                    line=endpoint.line,
                    description=f"Endpoint {endpoint.name} appears to lack authentication checks",
                    evidence=[f"No auth-related calls found in {endpoint.name}"],
                    fix_suggestion="Add authentication middleware or decorator to protect this endpoint",
                    confidence=0.7,
                    cwe_id="CWE-306"
                )
                issues.append(issue)
        
        return issues
    
    def find_hardcoded_secrets(self) -> List[SecurityIssue]:
        """
        Find hardcoded passwords, API keys, and other secrets.
        
        Returns:
            List of hardcoded secret issues
        """
        issues = []
        
        # Search for common secret patterns
        for pattern in ["password", "api_key", "apikey", "secret", "token"]:
            symbols = self.api.find_symbols(pattern)
            
            for symbol in symbols:
                # Check if it's a hardcoded value
                if symbol.kind in ["variable", "constant"]:
                    if self._looks_like_hardcoded_secret(symbol):
                        issue = SecurityIssue(
                            type="hardcoded_secret",
                            severity=Severity.HIGH,
                            symbol=symbol.fqn,
                            file=symbol.file,
                            line=symbol.line,
                            description=f"Potential hardcoded secret in {symbol.name}",
                            evidence=[f"Variable name suggests sensitive data: {symbol.name}"],
                            fix_suggestion="Use environment variables or a secure secrets management system",
                            confidence=0.6,
                            cwe_id="CWE-798"
                        )
                        issues.append(issue)
        
        return issues
    
    def find_unsafe_deserialization(self) -> List[SecurityIssue]:
        """
        Find unsafe deserialization that could lead to code execution.
        
        Returns:
            List of unsafe deserialization issues
        """
        issues = []
        
        # Find deserialization functions
        unsafe_functions = ["pickle.loads", "yaml.load", "eval", "exec"]
        
        for func_pattern in unsafe_functions:
            symbols = self.api.find_symbols(func_pattern.split(".")[-1])
            
            for symbol in symbols:
                # Check if it processes external data
                callers = self.api.get_callers(symbol.fqn)
                
                if any(self._handles_user_input(c) for c in callers):
                    issue = SecurityIssue(
                        type="unsafe_deserialization",
                        severity=Severity.HIGH,
                        symbol=symbol.fqn,
                        file=symbol.file,
                        line=symbol.line,
                        description=f"Unsafe deserialization in {symbol.name} could lead to code execution",
                        evidence=[f"Function {symbol.name} deserializes data from untrusted sources"],
                        fix_suggestion="Use safe deserialization methods. For YAML, use yaml.safe_load(). Avoid pickle for untrusted data.",
                        confidence=0.8,
                        cwe_id="CWE-502"
                    )
                    issues.append(issue)
        
        return issues
    
    # ========== Helper Methods ==========
    
    def _accesses_database(self, symbol: str) -> bool:
        """Check if symbol accesses database"""
        db_keywords = ["query", "execute", "select", "insert", "update", "delete", "sql"]
        
        # Check symbol name
        symbol_lower = symbol.lower()
        if any(keyword in symbol_lower for keyword in db_keywords):
            return True
        
        # Check callees
        callees = self.api.get_callees(symbol)
        for callee in callees:
            if any(keyword in callee.lower() for keyword in db_keywords):
                return True
        
        return False
    
    def _handles_user_input(self, symbol: str) -> bool:
        """Check if symbol handles user input"""
        input_keywords = ["request", "input", "param", "arg", "query", "body", "form", "user"]
        
        symbol_lower = symbol.lower()
        return any(keyword in symbol_lower for keyword in input_keywords)
    
    def _has_auth_check(self, symbol: Symbol) -> bool:
        """Check if symbol has authentication check"""
        auth_keywords = ["auth", "authenticate", "authorize", "permission", "check_user", "require_login"]
        
        # Check symbol name
        if any(keyword in symbol.name.lower() for keyword in auth_keywords):
            return True
        
        # Check callees
        callees = self.api.get_callees(symbol.fqn)
        for callee in callees:
            if any(keyword in callee.lower() for keyword in auth_keywords):
                return True
        
        return False
    
    def _find_api_endpoints(self) -> List[Symbol]:
        """Find potential API endpoints"""
        endpoints = []
        
        # Common endpoint patterns
        patterns = ["route", "api", "endpoint", "handler", "controller", "get", "post", "put", "delete", "patch"]
        
        for pattern in patterns:
            symbols = self.api.find_symbols(pattern, kind="function")
            symbols.extend(self.api.find_symbols(pattern, kind="method"))
            
            # Filter to likely endpoints
            for sym in symbols:
                if any(x in sym.name.lower() for x in ["get_", "post_", "put_", "delete_", "handle_", "api_"]):
                    endpoints.append(sym)
        
        return endpoints
    
    def _looks_like_hardcoded_secret(self, symbol: Symbol) -> bool:
        """Check if symbol looks like a hardcoded secret"""
        # Simple heuristic - would need file parsing for accuracy
        sensitive_names = ["password", "passwd", "pwd", "secret", "key", "token", "api_key", "apikey"]
        
        name_lower = symbol.name.lower()
        return any(sensitive in name_lower for sensitive in sensitive_names)
    
    def _check_sql_injection(self, symbol: Symbol) -> List[SecurityIssue]:
        """Check a specific symbol for SQL injection"""
        issues = []
        
        # Check if this symbol builds SQL queries with string concatenation
        # This is simplified - real implementation would parse the code
        if "query" in symbol.name.lower() or "sql" in symbol.name.lower():
            callers = self.api.get_callers(symbol.fqn)
            
            if any(self._handles_user_input(c) for c in callers):
                issue = SecurityIssue(
                    type="sql_injection",
                    severity=Severity.CRITICAL,
                    symbol=symbol.fqn,
                    file=symbol.file,
                    line=symbol.line,
                    description=f"Potential SQL injection vulnerability in {symbol.name}",
                    evidence=["Function handles database queries and receives user input"],
                    fix_suggestion="Use parameterized queries or an ORM with proper escaping",
                    confidence=0.7,
                    cwe_id="CWE-89"
                )
                issues.append(issue)
        
        return issues
    
    def _check_command_injection(self, symbol: Symbol) -> List[SecurityIssue]:
        """Check for command injection vulnerabilities"""
        issues = []
        
        # Check if symbol name suggests command execution
        dangerous_names = ["exec", "eval", "system", "shell", "popen", "spawn"]
        
        if any(danger in symbol.name.lower() for danger in dangerous_names):
            if self._handles_user_input(symbol.fqn):
                issue = SecurityIssue(
                    type="command_injection",
                    severity=Severity.CRITICAL,
                    symbol=symbol.fqn,
                    file=symbol.file,
                    line=symbol.line,
                    description=f"Potential command injection in {symbol.name}",
                    evidence=[f"Function {symbol.name} may execute system commands with user input"],
                    fix_suggestion="Avoid shell execution. Use safe APIs with proper input validation.",
                    confidence=0.7,
                    cwe_id="CWE-78"
                )
                issues.append(issue)
        
        return issues
    
    def _check_hardcoded_secrets(self, symbol: Symbol) -> List[SecurityIssue]:
        """Check for hardcoded secrets"""
        issues = []
        
        if self._looks_like_hardcoded_secret(symbol):
            if symbol.kind in ["variable", "constant"]:
                issue = SecurityIssue(
                    type="hardcoded_secret",
                    severity=Severity.HIGH,
                    symbol=symbol.fqn,
                    file=symbol.file,
                    line=symbol.line,
                    description=f"Potential hardcoded secret: {symbol.name}",
                    evidence=[f"Variable name suggests sensitive data"],
                    fix_suggestion="Use environment variables or a secrets management system",
                    confidence=0.5,
                    cwe_id="CWE-798"
                )
                issues.append(issue)
        
        return issues
    
    def _check_input_validation(self, symbol: Symbol) -> List[SecurityIssue]:
        """Check for missing input validation"""
        issues = []
        
        # Check if function validates its input
        callees = self.api.get_callees(symbol.fqn)
        
        validation_keywords = ["validate", "sanitize", "escape", "check", "verify", "clean"]
        has_validation = any(
            any(keyword in callee.lower() for keyword in validation_keywords)
            for callee in callees
        )
        
        if not has_validation and self._handles_user_input(symbol.fqn):
            issue = SecurityIssue(
                type="missing_validation",
                severity=Severity.MEDIUM,
                symbol=symbol.fqn,
                file=symbol.file,
                line=symbol.line,
                description=f"Function {symbol.name} handles user input without apparent validation",
                evidence=["No validation-related function calls found"],
                fix_suggestion="Add input validation and sanitization before processing user data",
                confidence=0.6,
                cwe_id="CWE-20"
            )
            issues.append(issue)
        
        return issues