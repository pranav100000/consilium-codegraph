/**
 * Example: Using Consilium CodeGraph as a library in your own project
 */

import { CodeGraphAPI, analyzeCodebase, findRelatedCode } from "@consilium/codegraph-client";

// Example 1: Simple API usage
export function getRepositoryStats(repoPath: string) {
  const api = new CodeGraphAPI(repoPath);

  try {
    const stats = api.getStats();
    return {
      files: stats.totalFiles,
      symbols: stats.totalSymbols,
      edges: stats.totalEdges,
      symbolBreakdown: stats.symbolsByKind,
      edgeBreakdown: stats.edgesByType,
    };
  } finally {
    api.close();
  }
}

// Example 2: Find specific symbols
export function findComponents(repoPath: string, componentName: string) {
  const api = new CodeGraphAPI(repoPath);

  try {
    const components = api.findSymbols(componentName);
    return components.map(c => ({
      name: c.name,
      file: c.location.file,
      line: c.location.line,
      kind: c.kind,
    }));
  } finally {
    api.close();
  }
}

// Example 3: Impact analysis
export function analyzeImpact(repoPath: string, symbolName: string) {
  const related = findRelatedCode(repoPath, symbolName);

  return {
    symbol: symbolName,
    directCallers: related.callers.length,
    directCallees: related.callees.length,
    totalImpact: related.impact.length,
    callersList: related.callers,
    impactedSymbols: related.impact,
  };
}

// Example 4: Quick overview
export function getQuickOverview(repoPath: string) {
  const analysis = analyzeCodebase(repoPath);

  return {
    stats: {
      files: analysis.stats.totalFiles,
      symbols: analysis.stats.totalSymbols,
      edges: analysis.stats.totalEdges,
    },
    quality: {
      cycles: analysis.cycles.length,
      complexFunctions: analysis.complexFunctions.length,
      entryPoints: analysis.entryPoints.length,
    },
    topComplexFunctions: analysis.complexFunctions.slice(0, 5).map(f => ({
      name: f.function,
      complexity: f.calleesCount,
    })),
  };
}

// Example 5: Custom analyzer class
export class ProjectAnalyzer {
  private repoPath: string;

  constructor(repoPath: string) {
    this.repoPath = repoPath;
  }

  // Get all classes in the project
  getAllClasses() {
    const api = new CodeGraphAPI(this.repoPath);
    try {
      return api.findSymbols("", "Class");
    } finally {
      api.close();
    }
  }

  // Get all functions in the project
  getAllFunctions() {
    const api = new CodeGraphAPI(this.repoPath);
    try {
      return api.findSymbols("", "Function");
    } finally {
      api.close();
    }
  }

  // Find circular dependencies
  findCycles() {
    const api = new CodeGraphAPI(this.repoPath);
    try {
      return api.findCycles();
    } finally {
      api.close();
    }
  }

  // Get symbols in a specific file
  getFileSymbols(filePath: string) {
    const api = new CodeGraphAPI(this.repoPath);
    try {
      return api.getFileSymbols(filePath);
    } finally {
      api.close();
    }
  }

  // Check who calls a function
  getCallers(symbolName: string) {
    const api = new CodeGraphAPI(this.repoPath);
    try {
      return api.getCallers(symbolName);
    } finally {
      api.close();
    }
  }

  // Check what a function calls
  getCallees(symbolName: string) {
    const api = new CodeGraphAPI(this.repoPath);
    try {
      return api.getCallees(symbolName);
    } finally {
      api.close();
    }
  }

  // Get complete analysis
  getCompleteAnalysis() {
    return {
      overview: getQuickOverview(this.repoPath),
      stats: getRepositoryStats(this.repoPath),
      cycles: this.findCycles(),
    };
  }
}

// Example usage (if run directly)
if (import.meta.url === `file://${process.argv[1]}`) {
  const targetRepo = process.argv[2] || process.cwd();

  console.log(`\n📊 Analyzing: ${targetRepo}\n`);

  // Use the convenience function
  const overview = getQuickOverview(targetRepo);
  console.log("Overview:", JSON.stringify(overview, null, 2));

  // Use the class
  const analyzer = new ProjectAnalyzer(targetRepo);
  const classes = analyzer.getAllClasses();
  console.log(`\nFound ${classes.length} classes`);

  const cycles = analyzer.findCycles();
  if (cycles.length > 0) {
    console.log(`⚠️  Found ${cycles.length} circular dependencies!`);
  }
}
