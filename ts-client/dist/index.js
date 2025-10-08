"use strict";
/**
 * Consilium CodeGraph TypeScript Client
 *
 * A TypeScript client for querying code graphs with semantic enrichment.
 * Provides programmatic access to the Consilium CodeGraph database.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.isSecurityCritical = exports.needsRefactoring = exports.hasCycles = exports.getImpactRadius = exports.isComplex = exports.AnalysisQuality = exports.Severity = exports.EdgeType = exports.SymbolKind = exports.getDatabaseRuntime = exports.createDatabase = exports.getCLIInfo = exports.isScanned = exports.scanRepositorySync = exports.scanRepository = exports.AgentCodeGraph = exports.findRelatedCode = exports.analyzeCodebase = exports.CodeGraphAPI = exports.CodeGraph = void 0;
// Core classes
var code_graph_1 = require("./code-graph");
Object.defineProperty(exports, "CodeGraph", { enumerable: true, get: function () { return code_graph_1.CodeGraph; } });
var code_graph_api_1 = require("./code-graph-api");
Object.defineProperty(exports, "CodeGraphAPI", { enumerable: true, get: function () { return code_graph_api_1.CodeGraphAPI; } });
Object.defineProperty(exports, "analyzeCodebase", { enumerable: true, get: function () { return code_graph_api_1.analyzeCodebase; } });
Object.defineProperty(exports, "findRelatedCode", { enumerable: true, get: function () { return code_graph_api_1.findRelatedCode; } });
// Agent-focused API (designed to complement Read/Grep/Glob tools)
var agent_api_1 = require("./agent-api");
Object.defineProperty(exports, "AgentCodeGraph", { enumerable: true, get: function () { return agent_api_1.AgentCodeGraph; } });
// Scanner functions (for indexing repositories)
var scanner_1 = require("./scanner");
Object.defineProperty(exports, "scanRepository", { enumerable: true, get: function () { return scanner_1.scanRepository; } });
Object.defineProperty(exports, "scanRepositorySync", { enumerable: true, get: function () { return scanner_1.scanRepositorySync; } });
Object.defineProperty(exports, "isScanned", { enumerable: true, get: function () { return scanner_1.isScanned; } });
Object.defineProperty(exports, "getCLIInfo", { enumerable: true, get: function () { return scanner_1.getCLIInfo; } });
// Database adapter (works with both Node.js and Bun)
var db_adapter_1 = require("./db-adapter");
Object.defineProperty(exports, "createDatabase", { enumerable: true, get: function () { return db_adapter_1.createDatabase; } });
Object.defineProperty(exports, "getDatabaseRuntime", { enumerable: true, get: function () { return db_adapter_1.getDatabaseRuntime; } });
// Enums
var models_1 = require("./models");
Object.defineProperty(exports, "SymbolKind", { enumerable: true, get: function () { return models_1.SymbolKind; } });
Object.defineProperty(exports, "EdgeType", { enumerable: true, get: function () { return models_1.EdgeType; } });
Object.defineProperty(exports, "Severity", { enumerable: true, get: function () { return models_1.Severity; } });
Object.defineProperty(exports, "AnalysisQuality", { enumerable: true, get: function () { return models_1.AnalysisQuality; } });
// Utility functions
var models_2 = require("./models");
Object.defineProperty(exports, "isComplex", { enumerable: true, get: function () { return models_2.isComplex; } });
Object.defineProperty(exports, "getImpactRadius", { enumerable: true, get: function () { return models_2.getImpactRadius; } });
Object.defineProperty(exports, "hasCycles", { enumerable: true, get: function () { return models_2.hasCycles; } });
Object.defineProperty(exports, "needsRefactoring", { enumerable: true, get: function () { return models_2.needsRefactoring; } });
Object.defineProperty(exports, "isSecurityCritical", { enumerable: true, get: function () { return models_2.isSecurityCritical; } });
//# sourceMappingURL=index.js.map