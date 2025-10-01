/**
 * Consilium CodeGraph TypeScript Client
 *
 * A TypeScript client for querying code graphs with semantic enrichment.
 * Provides programmatic access to the Consilium CodeGraph database.
 */

// Core classes
export { CodeGraph } from "./code-graph";
export { CodeGraphAPI, analyzeCodebase, findRelatedCode } from "./code-graph-api";

// Models and types
export {
  Symbol,
  Edge,
  Location,
  CallPath,
  DataFlow,
  SecurityIssue,
  ComplexityMetrics,
  CodeSmell,
  DuplicateCode,
  ImpactAnalysis,
  DependencyGraph,
  FunctionExplanation,
  SecurityContext,
  RefactoringSuggestion,
  LayerViolation,
  ReviewResult,
  GraphStats,
} from "./models";

// Enums
export {
  SymbolKind,
  EdgeType,
  Severity,
  AnalysisQuality,
} from "./models";

// Utility functions
export {
  isComplex,
  getImpactRadius,
  hasCycles,
  needsRefactoring,
  isSecurityCritical,
} from "./models";
