"use strict";
/**
 * Data models for the Code Graph API
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.AnalysisQuality = exports.Severity = exports.EdgeType = exports.SymbolKind = void 0;
exports.isComplex = isComplex;
exports.getImpactRadius = getImpactRadius;
exports.hasCycles = hasCycles;
exports.needsRefactoring = needsRefactoring;
exports.isSecurityCritical = isSecurityCritical;
/**
 * Types of symbols in the code graph
 */
var SymbolKind;
(function (SymbolKind) {
    SymbolKind["FUNCTION"] = "function";
    SymbolKind["METHOD"] = "method";
    SymbolKind["CLASS"] = "class";
    SymbolKind["INTERFACE"] = "interface";
    SymbolKind["VARIABLE"] = "variable";
    SymbolKind["TYPE"] = "type";
    SymbolKind["MODULE"] = "module";
    SymbolKind["PACKAGE"] = "package";
    SymbolKind["NAMESPACE"] = "namespace";
    SymbolKind["ENUM"] = "enum";
    SymbolKind["ENUM_MEMBER"] = "enum_member";
    SymbolKind["STRUCT"] = "struct";
    SymbolKind["TRAIT"] = "trait";
    SymbolKind["CONSTANT"] = "constant";
    SymbolKind["FIELD"] = "field";
    SymbolKind["PROPERTY"] = "property";
})(SymbolKind || (exports.SymbolKind = SymbolKind = {}));
/**
 * Types of relationships between symbols
 */
var EdgeType;
(function (EdgeType) {
    EdgeType["CONTAINS"] = "contains";
    EdgeType["DECLARES"] = "declares";
    EdgeType["CALLS"] = "calls";
    EdgeType["IMPORTS"] = "imports";
    EdgeType["EXTENDS"] = "extends";
    EdgeType["IMPLEMENTS"] = "implements";
    EdgeType["USES"] = "uses";
    EdgeType["RETURNS"] = "returns";
    EdgeType["THROWS"] = "throws";
    EdgeType["OVERRIDES"] = "overrides";
})(EdgeType || (exports.EdgeType = EdgeType = {}));
/**
 * Issue severity levels
 */
var Severity;
(function (Severity) {
    Severity["CRITICAL"] = "critical";
    Severity["HIGH"] = "high";
    Severity["MEDIUM"] = "medium";
    Severity["LOW"] = "low";
    Severity["INFO"] = "info";
})(Severity || (exports.Severity = Severity = {}));
/**
 * Quality level of analysis
 */
var AnalysisQuality;
(function (AnalysisQuality) {
    AnalysisQuality["SEMANTIC"] = "semantic";
    AnalysisQuality["SYNTACTIC"] = "syntactic";
    AnalysisQuality["HEURISTIC"] = "heuristic";
})(AnalysisQuality || (exports.AnalysisQuality = AnalysisQuality = {}));
/**
 * Check if complexity exceeds thresholds
 */
function isComplex(metrics) {
    return metrics.cyclomatic > 10 || metrics.cognitive > 15;
}
/**
 * Get total number of affected symbols
 */
function getImpactRadius(impact) {
    return impact.transitiveImpact.size;
}
/**
 * Check if dependency graph has cycles
 */
function hasCycles(graph) {
    return graph.cycles.length > 0;
}
/**
 * Check if function needs refactoring
 */
function needsRefactoring(explanation) {
    return (isComplex(explanation.complexity) ||
        explanation.testCoverage < 0.5 ||
        explanation.sideEffects.length > 3);
}
/**
 * Check if symbol requires security review
 */
function isSecurityCritical(context) {
    return (context.handlesUserInput ||
        context.performsAuth ||
        context.performsCrypto ||
        ["admin", "system"].includes(context.privilegeLevel));
}
//# sourceMappingURL=models.js.map