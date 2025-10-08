/**
 * Consilium CodeGraph TypeScript Client
 *
 * A TypeScript client for querying code graphs with semantic enrichment.
 * Provides programmatic access to the Consilium CodeGraph database.
 */
export { CodeGraph } from "./code-graph";
export { CodeGraphAPI, analyzeCodebase, findRelatedCode } from "./code-graph-api";
export { AgentCodeGraph, SymbolInfo, SymbolSearchResult, RelationshipNode, RelationshipType, } from "./agent-api";
export { scanRepository, scanRepositorySync, isScanned, getCLIInfo, ScanOptions, ScanResult, } from "./scanner";
export { createDatabase, getDatabaseRuntime, DatabaseAdapter, StatementAdapter, } from "./db-adapter";
export { Symbol, Edge, Location, CallPath, DataFlow, SecurityIssue, ComplexityMetrics, CodeSmell, DuplicateCode, ImpactAnalysis, DependencyGraph, FunctionExplanation, SecurityContext, RefactoringSuggestion, LayerViolation, ReviewResult, GraphStats, } from "./models";
export { SymbolKind, EdgeType, Severity, AnalysisQuality, } from "./models";
export { isComplex, getImpactRadius, hasCycles, needsRefactoring, isSecurityCritical, } from "./models";
//# sourceMappingURL=index.d.ts.map