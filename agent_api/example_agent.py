"""
Example of how agents would use the Code Graph API
"""

import asyncio
from typing import List, Dict, Any
from pathlib import Path

from agent_api import (
    CodeGraph,
    CodeAnalyzer,
    AgentHelpers,
    SecurityIssue,
    CodeSmell,
    RefactoringSuggestion,
    Severity
)
from agent_api.models import ReviewResult


class SecurityReviewAgent:
    """Agent specialized in security review"""
    
    def __init__(self, repo_path: str):
        self.graph = CodeGraph(repo_path)
        self.analyzer = CodeAnalyzer(self.graph)
        self.helpers = AgentHelpers(repo_path)
        
    def review(self, changed_files: List[str]) -> ReviewResult:
        """
        Perform security review on changed files.
        
        Args:
            changed_files: List of file paths that were modified
            
        Returns:
            ReviewResult with security findings
        """
        issues = []
        suggestions = []
        
        for file_path in changed_files:
            # Get all symbols in the file
            symbols = self.graph.get_file_symbols(file_path)
            
            for symbol in symbols:
                # Get security context
                security_context = self.helpers.get_security_context(symbol.fqn)
                
                # Check if this is security-critical
                if security_context.is_security_critical:
                    print(f"  Analyzing security-critical function: {symbol.name}")
                    
                    # Check for SQL injection
                    if security_context.accesses_database and security_context.handles_user_input:
                        sql_issues = self.analyzer.find_sql_injections()
                        issues.extend([i for i in sql_issues if symbol.fqn in i.description])
                    
                    # Check for missing authentication
                    if not security_context.performs_auth and "api" in symbol.name.lower():
                        auth_issues = self.analyzer.find_auth_bypasses()
                        issues.extend([i for i in auth_issues if symbol.fqn in i.description])
                    
                    # Add vulnerabilities from context
                    issues.extend(security_context.vulnerabilities)
        
        # Generate summary
        critical_count = len([i for i in issues if i.severity == Severity.CRITICAL])
        high_count = len([i for i in issues if i.severity == Severity.HIGH])
        
        summary = f"Found {len(issues)} security issues: {critical_count} critical, {high_count} high"
        
        return ReviewResult(
            agent_name="SecurityReviewer",
            issues=issues,
            code_smells=[],
            suggestions=suggestions,
            metrics={
                "files_reviewed": len(changed_files),
                "critical_issues": critical_count,
                "high_issues": high_count
            },
            summary=summary,
            confidence=0.85
        )


class PerformanceReviewAgent:
    """Agent specialized in performance review"""
    
    def __init__(self, repo_path: str):
        self.graph = CodeGraph(repo_path)
        self.analyzer = CodeAnalyzer(self.graph)
        self.helpers = AgentHelpers(repo_path)
    
    def review(self, changed_files: List[str]) -> ReviewResult:
        """
        Perform performance review on changed files.
        
        Args:
            changed_files: List of file paths that were modified
            
        Returns:
            ReviewResult with performance findings
        """
        issues = []
        code_smells = []
        suggestions = []
        
        for file_path in changed_files:
            symbols = self.graph.get_file_symbols(file_path)
            
            for symbol in symbols:
                # Check complexity
                complexity = self.analyzer.get_complexity(symbol.fqn)
                
                if complexity.is_complex:
                    print(f"  Found complex function: {symbol.name} (cyclomatic: {complexity.cyclomatic})")
                    
                    code_smells.append(CodeSmell(
                        type="high_complexity",
                        location=symbol.location,
                        description=f"Function {symbol.name} has high complexity",
                        impact="Harder to maintain and slower to execute",
                        refactoring_suggestion="Break into smaller functions",
                        severity=Severity.MEDIUM,
                        metrics={"cyclomatic": complexity.cyclomatic}
                    ))
                    
                    # Get refactoring suggestions
                    refactoring = self.helpers.suggest_refactoring(symbol.fqn)
                    suggestions.extend(refactoring)
                
                # Check for N+1 queries
                if self.helpers._accesses_database(symbol.fqn):
                    # Check if called in a loop
                    callers = self.graph.get_callers(symbol.fqn)
                    for caller_path in callers:
                        if any("loop" in s.name.lower() or "for" in s.name.lower() for s in caller_path.path):
                            issues.append(SecurityIssue(
                                issue_id=f"n_plus_one_{symbol.fqn}",
                                type="performance",
                                severity=Severity.MEDIUM,
                                location=symbol.location,
                                description=f"Potential N+1 query in {symbol.name}",
                                evidence=["Database access inside loop"],
                                fix_suggestion="Use batch queries or eager loading",
                                confidence=0.7
                            ))
        
        # Find duplicate code
        duplicates = self.analyzer.find_duplicates()
        for dup in duplicates[:5]:  # Limit to top 5
            code_smells.append(CodeSmell(
                type="duplicate_code",
                location=dup.locations[0],
                description=f"Duplicate code found ({dup.similarity:.0%} similar)",
                impact="Increased maintenance burden",
                refactoring_suggestion="Extract common functionality",
                severity=Severity.LOW,
                metrics={"similarity": dup.similarity, "lines": dup.lines}
            ))
        
        summary = f"Found {len(code_smells)} performance issues and {len(suggestions)} improvement opportunities"
        
        return ReviewResult(
            agent_name="PerformanceReviewer",
            issues=issues,
            code_smells=code_smells,
            suggestions=suggestions,
            metrics={
                "files_reviewed": len(changed_files),
                "complex_functions": len([s for s in code_smells if s.type == "high_complexity"]),
                "duplicate_blocks": len([s for s in code_smells if s.type == "duplicate_code"])
            },
            summary=summary,
            confidence=0.75
        )


class ArchitectureReviewAgent:
    """Agent specialized in architecture review"""
    
    def __init__(self, repo_path: str):
        self.graph = CodeGraph(repo_path)
        self.analyzer = CodeAnalyzer(self.graph)
        self.helpers = AgentHelpers(repo_path)
    
    def review(self, layer_rules: Dict[str, List[str]] = None) -> ReviewResult:
        """
        Perform architecture review.
        
        Args:
            layer_rules: Optional architecture layer rules
            
        Returns:
            ReviewResult with architecture findings
        """
        issues = []
        code_smells = []
        suggestions = []
        
        # Default layer rules if not provided
        if layer_rules is None:
            layer_rules = {
                "presentation": ["business"],
                "business": ["data"],
                "data": []
            }
        
        # Check for circular dependencies
        print("  Checking for circular dependencies...")
        cycles = self.analyzer.find_circular_dependencies()
        
        for cycle in cycles:
            code_smells.append(CodeSmell(
                type="circular_dependency",
                location=self.graph.get_symbol(cycle[0]).location,
                description=f"Circular dependency: {' -> '.join(cycle)}",
                impact="Tight coupling, harder to maintain",
                refactoring_suggestion="Break the cycle using dependency inversion",
                severity=Severity.HIGH,
                metrics={"cycle_length": len(cycle)}
            ))
        
        # Check layer violations
        print("  Checking architecture layer violations...")
        violations = self.analyzer.check_layer_violations(layer_rules)
        
        for violation in violations:
            issues.append(SecurityIssue(
                issue_id=f"layer_violation_{len(issues)}",
                type="architecture_violation",
                severity=Severity.MEDIUM,
                location=self.graph.get_symbol(violation.from_symbol).location,
                description=f"Layer violation: {violation.from_layer} -> {violation.to_layer}",
                evidence=[f"{violation.from_symbol} depends on {violation.to_symbol}"],
                fix_suggestion=f"Use proper layer hierarchy: {' -> '.join(violation.suggested_path)}",
                confidence=0.9
            ))
        
        # Analyze module dependencies
        dependencies = self.analyzer.get_module_dependencies()
        
        # Find god modules (too many dependencies)
        for module, deps in dependencies.items():
            if len(deps) > 10:
                suggestions.append(RefactoringSuggestion(
                    type="split_module",
                    location=self.graph.get_symbol(module).location if self.graph.get_symbol(module) else None,
                    description=f"Module {module} has too many dependencies ({len(deps)})",
                    benefit="Better separation of concerns",
                    example="Split into smaller, focused modules"
                ))
        
        summary = f"Found {len(cycles)} circular dependencies and {len(violations)} layer violations"
        
        return ReviewResult(
            agent_name="ArchitectureReviewer",
            issues=issues,
            code_smells=code_smells,
            suggestions=suggestions,
            metrics={
                "circular_dependencies": len(cycles),
                "layer_violations": len(violations),
                "total_modules": len(dependencies)
            },
            summary=summary,
            confidence=0.9
        )


class CodeReviewOrchestrator:
    """Orchestrates multiple review agents"""
    
    def __init__(self, repo_path: str):
        self.repo_path = repo_path
        self.security_agent = SecurityReviewAgent(repo_path)
        self.performance_agent = PerformanceReviewAgent(repo_path)
        self.architecture_agent = ArchitectureReviewAgent(repo_path)
    
    async def review_changes(self, changed_files: List[str]) -> Dict[str, Any]:
        """
        Run all review agents on changed files.
        
        Args:
            changed_files: List of modified file paths
            
        Returns:
            Combined review results
        """
        print(f"\\nReviewing {len(changed_files)} changed files...")
        print("=" * 60)
        
        # Run reviews (in real implementation, these would be async)
        print("\\n🔒 Running Security Review...")
        security_review = self.security_agent.review(changed_files)
        
        print("\\n⚡ Running Performance Review...")
        performance_review = self.performance_agent.review(changed_files)
        
        print("\\n🏗️ Running Architecture Review...")
        architecture_review = self.architecture_agent.review()
        
        # Combine results
        all_issues = (
            security_review.issues + 
            performance_review.issues + 
            architecture_review.issues
        )
        
        all_code_smells = (
            security_review.code_smells + 
            performance_review.code_smells + 
            architecture_review.code_smells
        )
        
        all_suggestions = (
            security_review.suggestions + 
            performance_review.suggestions + 
            architecture_review.suggestions
        )
        
        # Sort by severity
        all_issues.sort(key=lambda x: [Severity.CRITICAL, Severity.HIGH, Severity.MEDIUM, Severity.LOW, Severity.INFO].index(x.severity))
        
        # Generate report
        print("\\n" + "=" * 60)
        print("📊 REVIEW SUMMARY")
        print("=" * 60)
        
        print(f"\\n✅ Files Reviewed: {len(changed_files)}")
        print(f"\\n🚨 Issues Found:")
        print(f"  - Critical: {len([i for i in all_issues if i.severity == Severity.CRITICAL])}")
        print(f"  - High: {len([i for i in all_issues if i.severity == Severity.HIGH])}")
        print(f"  - Medium: {len([i for i in all_issues if i.severity == Severity.MEDIUM])}")
        print(f"  - Low: {len([i for i in all_issues if i.severity == Severity.LOW])}")
        
        print(f"\\n💡 Code Smells: {len(all_code_smells)}")
        print(f"🔧 Suggestions: {len(all_suggestions)}")
        
        # Show top issues
        if all_issues:
            print(f"\\n⚠️ Top Issues:")
            for issue in all_issues[:3]:
                print(f"  [{issue.severity.value.upper()}] {issue.type}: {issue.description[:80]}...")
        
        return {
            "files_reviewed": changed_files,
            "issues": all_issues,
            "code_smells": all_code_smells,
            "suggestions": all_suggestions,
            "metrics": {
                "security": security_review.metrics,
                "performance": performance_review.metrics,
                "architecture": architecture_review.metrics
            },
            "summary": {
                "total_issues": len(all_issues),
                "total_code_smells": len(all_code_smells),
                "total_suggestions": len(all_suggestions)
            }
        }


def main():
    """Example usage of the multi-agent code review system"""
    
    # Example repository path (update to your repo)
    repo_path = "/Users/pranavsharan/Developer/consilium-codegraph"
    
    # Example changed files (simulate a PR)
    changed_files = [
        "crates/core/src/main.rs",
        "crates/store/src/graph.rs"
    ]
    
    # Create orchestrator
    orchestrator = CodeReviewOrchestrator(repo_path)
    
    # Run review
    print("🤖 Multi-Agent Code Review System")
    print("=" * 60)
    
    # Run async review
    results = asyncio.run(orchestrator.review_changes(changed_files))
    
    print("\\n✅ Review Complete!")
    
    # In a real system, results would be posted as PR comments
    # or sent to the development team
    
    return results


if __name__ == "__main__":
    main()