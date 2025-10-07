/**
 * Consilium CodeGraph TypeScript Client
 *
 * A TypeScript client for querying code graphs with semantic enrichment.
 * Provides programmatic access to the Consilium CodeGraph database.
 */

// Core classes
export { CodeGraph } from "./code-graph";
export { CodeGraphAPI, analyzeCodebase, findRelatedCode } from "./code-graph-api";

// Agent-focused API (designed to complement Read/Grep/Glob tools)
export {
  AgentCodeGraph,
  SymbolInfo,
  SymbolSearchResult,
  RelationshipNode,
  RelationshipType,
} from "./agent-api";

// Scanner functions (for indexing repositories)
export {
  scanRepository,
  scanRepositorySync,
  isScanned,
  getCLIInfo,
  ScanOptions,
  ScanResult,
} from "./scanner";

// Database adapter (works with both Node.js and Bun)
export {
  createDatabase,
  getDatabaseRuntime,
  DatabaseAdapter,
  StatementAdapter,
} from "./db-adapter";

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
