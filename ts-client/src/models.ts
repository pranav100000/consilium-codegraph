/**
 * Data models for the Code Graph API
 */

/**
 * Types of symbols in the code graph
 */
export enum SymbolKind {
  FUNCTION = "function",
  METHOD = "method",
  CLASS = "class",
  INTERFACE = "interface",
  VARIABLE = "variable",
  TYPE = "type",
  MODULE = "module",
  PACKAGE = "package",
  NAMESPACE = "namespace",
  ENUM = "enum",
  ENUM_MEMBER = "enum_member",
  STRUCT = "struct",
  TRAIT = "trait",
  CONSTANT = "constant",
  FIELD = "field",
  PROPERTY = "property",
}

/**
 * Types of relationships between symbols
 */
export enum EdgeType {
  CONTAINS = "contains",
  DECLARES = "declares",
  CALLS = "calls",
  IMPORTS = "imports",
  EXTENDS = "extends",
  IMPLEMENTS = "implements",
  USES = "uses",
  RETURNS = "returns",
  THROWS = "throws",
  OVERRIDES = "overrides",
}

/**
 * Issue severity levels
 */
export enum Severity {
  CRITICAL = "critical",
  HIGH = "high",
  MEDIUM = "medium",
  LOW = "low",
  INFO = "info",
}

/**
 * Quality level of analysis
 */
export enum AnalysisQuality {
  SEMANTIC = "semantic",   // Full semantic analysis (e.g., from SCIP)
  SYNTACTIC = "syntactic", // Syntax-only analysis (e.g., from Tree-sitter)
  HEURISTIC = "heuristic", // Pattern-based approximation
}

/**
 * Location in source code
 */
export interface Location {
  file: string;
  line: number;
  column?: number;
  endLine?: number;
  endColumn?: number;
}

/**
 * A symbol in the code graph
 */
export interface Symbol {
  fqn: string;           // Fully qualified name
  name: string;
  kind: SymbolKind;
  location: Location;
  signature?: string;
  docstring?: string;
  analyzer?: string;     // Which tool analyzed this
  confidence?: number;
  metadata?: Record<string, any>;
}

/**
 * An edge in the code graph
 */
export interface Edge {
  source: string;        // Source symbol FQN
  target: string;        // Target symbol FQN
  edgeType: EdgeType;
}

/**
 * A path through function calls
 */
export interface CallPath {
  path: Symbol[];
  depth: number;
  isRecursive?: boolean;
  confidence?: number;
}

/**
 * Data flow from source to sink
 */
export interface DataFlow {
  source: Symbol;
  sink: Symbol;
  path: Symbol[];
  isTainted: boolean;
  isSanitized: boolean;
  confidence: number;
  analysisQuality: AnalysisQuality;
}

/**
 * A security vulnerability or concern
 */
export interface SecurityIssue {
  issueId: string;
  type: string;
  severity: Severity;
  location: Location;
  description: string;
  evidence: string[];    // Code snippets showing the issue
  fixSuggestion: string;
  confidence: number;
  cweId?: string;
  owaspCategory?: string;
  falsePositive?: boolean;
}

/**
 * Code complexity measurements
 */
export interface ComplexityMetrics {
  cyclomatic: number;
  cognitive: number;
  linesOfCode: number;
  nestingDepth: number;
  parameterCount: number;
  returnPoints: number;
}

/**
 * Check if complexity exceeds thresholds
 */
export function isComplex(metrics: ComplexityMetrics): boolean {
  return metrics.cyclomatic > 10 || metrics.cognitive > 15;
}

/**
 * Code quality issue
 */
export interface CodeSmell {
  type: string;
  location: Location;
  description: string;
  impact: string;        // How this affects code quality
  refactoringSuggestion: string;
  severity: Severity;
  metrics?: Record<string, any>;
}

/**
 * Duplicate code detection result
 */
export interface DuplicateCode {
  locations: Location[];
  lines: number;
  tokens: number;
  similarity: number;
  codeSnippet: string;
}

/**
 * Analysis of change impact
 */
export interface ImpactAnalysis {
  symbol: string;
  directCallers: Symbol[];
  transitiveImpact: Set<string>;  // FQNs of all affected symbols
  affectedTests: Symbol[];
  affectedFeatures: string[];
  riskScore: number;
}

/**
 * Get total number of affected symbols
 */
export function getImpactRadius(impact: ImpactAnalysis): number {
  return impact.transitiveImpact.size;
}

/**
 * Dependency relationships
 */
export interface DependencyGraph {
  root: Symbol;
  dependencies: Record<string, string[]>;  // FQN -> List of dependency FQNs
  dependents: Record<string, string[]>;    // FQN -> List of dependent FQNs
  cycles: string[][];                      // Circular dependencies
}

/**
 * Check if dependency graph has cycles
 */
export function hasCycles(graph: DependencyGraph): boolean {
  return graph.cycles.length > 0;
}

/**
 * High-level explanation of a function
 */
export interface FunctionExplanation {
  symbol: Symbol;
  purpose: string;
  parameters: Array<Record<string, any>>;
  returns?: string;
  sideEffects: string[];
  complexity: ComplexityMetrics;
  testCoverage: number;
  dependencies: string[];
}

/**
 * Check if function needs refactoring
 */
export function needsRefactoring(explanation: FunctionExplanation): boolean {
  return (
    isComplex(explanation.complexity) ||
    explanation.testCoverage < 0.5 ||
    explanation.sideEffects.length > 3
  );
}

/**
 * Security-relevant information about a symbol
 */
export interface SecurityContext {
  symbol: string;
  handlesUserInput: boolean;
  accessesDatabase: boolean;
  performsAuth: boolean;
  performsCrypto: boolean;
  externalCalls: string[];
  vulnerabilities: SecurityIssue[];
  privilegeLevel: "user" | "admin" | "system";
}

/**
 * Check if symbol requires security review
 */
export function isSecurityCritical(context: SecurityContext): boolean {
  return (
    context.handlesUserInput ||
    context.performsAuth ||
    context.performsCrypto ||
    ["admin", "system"].includes(context.privilegeLevel)
  );
}

/**
 * Suggested code improvement
 */
export interface RefactoringSuggestion {
  type: string;          // "extract_method", "rename", "simplify", etc.
  location: Location;
  description: string;
  benefit: string;
  example?: string;
  automated?: boolean;   // Can be applied automatically
}

/**
 * Architecture layer violation
 */
export interface LayerViolation {
  fromLayer: string;
  toLayer: string;
  fromSymbol: string;
  toSymbol: string;
  violationType: string; // "skip_layer", "reverse_dependency", etc.
  suggestedPath: string[]; // Correct path through layers
}

/**
 * Complete review result from an agent
 */
export interface ReviewResult {
  agentName: string;
  issues: SecurityIssue[];
  codeSmells: CodeSmell[];
  suggestions: RefactoringSuggestion[];
  metrics: Record<string, any>;
  summary: string;
  confidence: number;
}

/**
 * Statistics about the code graph
 */
export interface GraphStats {
  symbolsByKind: Record<string, number>;
  edgesByType: Record<string, number>;
  totalFiles: number;
  totalSymbols: number;
  totalEdges: number;
}
