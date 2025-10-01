use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use store::GraphStore;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use ts_harness::TypeScriptHarness;
use py_harness::PythonHarness;
use go_harness::GoHarness;
use rust_harness::RustHarness;
use java_harness::JavaHarness;
use cpp_harness::CppHarness;
use csharp_harness::CSharpHarness;

mod walker;
use walker::FileWalker;

mod resolution;
use resolution::ResolutionEngine;

mod language_strategy;
mod metrics;
use metrics::MetricsCollector;

#[derive(Parser)]
#[command(name = "reviewbot")]
#[command(about = "Fast code graph builder with semantic enrichment", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
}

#[derive(Subcommand)]
enum GraphCommands {
    Stats,
    Cycles {
        symbol: String,
    },
    Path {
        from: String,
        to: String,
    },
}

#[derive(Subcommand)]
enum Commands {
    Scan {
        #[arg(long)]
        no_semantic: bool,
        
        #[arg(long)]
        semantic: bool,
        
        #[arg(long)]
        incremental: bool,
        
        #[arg(long)]
        no_write: bool,
        
        #[arg(long)]
        commit: Option<String>,
        
        #[arg(long)]
        jobs: Option<usize>,
        
        #[arg(long, value_delimiter = ',')]
        lang: Vec<String>,
    },
    
    Show {
        #[arg(long)]
        symbol: String,
        
        #[arg(long)]
        callers: bool,
        
        #[arg(long)]
        callees: bool,
        
        #[arg(long)]
        importers: bool,
        
        #[arg(long, default_value = "1")]
        depth: usize,
    },
    
    Search {
        query: String,
        
        #[arg(long, default_value = "20")]
        k: usize,
        
        #[arg(long)]
        hybrid: bool,
    },
    
    Graph {
        #[command(subcommand)]
        cmd: GraphCommands,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    let cli = Cli::parse();
    
    let repo_root = cli.repo.unwrap_or_else(|| {
        std::env::current_dir().expect("Failed to get current directory")
    });
    
    match cli.command {
        Commands::Scan { no_write, semantic, no_semantic, incremental: _, .. } => {
            let mut metrics = MetricsCollector::new();
            metrics.start_phase("initialization");
            
            if no_write {
                info!("Running scan in dry-run mode (--no-write)");
            }
            
            // Determine if we should run semantic analysis
            let run_semantic = semantic && !no_semantic;
            if run_semantic {
                info!("Semantic analysis enabled");
            }
            
            let commit_sha = get_current_commit(&repo_root)?;
            info!("Scanning repository at commit: {}", commit_sha);
            
            metrics.end_phase("initialization");
            metrics.update_memory_usage();
            
            // Check for incremental scan opportunity
            metrics.start_phase("file_discovery");
            let mut files_to_process = Vec::new();
            let mut incremental = false;
            
            if !no_write {
                let store = GraphStore::new(&repo_root)?;
                if let Some(last_commit) = store.get_last_scanned_commit()? {
                    let mut changed = Vec::new();

                    if last_commit != commit_sha {
                        // Get committed changes since last scan
                        let committed_changes = get_changed_files(&repo_root, &last_commit, &commit_sha)?;
                        changed.extend(committed_changes);
                        info!("Found {} committed changes since {}", changed.len(), &last_commit[0..7]);
                    }

                    // Always check for uncommitted changes (working directory)
                    let uncommitted_changes = get_uncommitted_changes(&repo_root)?;
                    if !uncommitted_changes.is_empty() {
                        info!("Found {} uncommitted changes in working directory", uncommitted_changes.len());
                        changed.extend(uncommitted_changes);
                    }

                    // Remove duplicates
                    changed.sort();
                    changed.dedup();

                    if !changed.is_empty() && changed.len() < 100 {  // Arbitrary threshold
                        info!("Incremental scan: {} total files changed", changed.len());

                        // Get impacted files (files that import changed files)
                        let mut impacted = std::collections::HashSet::new();
                        for file in &changed {
                            impacted.insert(file.clone());
                            for dependent in store.get_file_dependents(file)? {
                                impacted.insert(dependent);
                            }
                        }

                        files_to_process = impacted.into_iter()
                            .map(|f| repo_root.join(&f))
                            .collect();
                        incremental = true;
                        info!("Total files to reprocess (including dependents): {}", files_to_process.len());
                    } else if changed.is_empty() {
                        info!("Repository unchanged since last scan");
                        println!("Repository unchanged since last scan");
                        return Ok(());
                    }
                }
            }
            
            // If not incremental, walk all files
            if !incremental {
                let walker = FileWalker::new(repo_root.clone());
                files_to_process = walker.walk()?;
            }
            
            metrics.end_phase("file_discovery");
            metrics.update_memory_usage();
            
            if files_to_process.is_empty() {
                println!("No files found to index");
                return Ok(());
            }
            
            if !no_write {
                metrics.start_phase("syntactic_analysis");
                let store = GraphStore::new(&repo_root)?;
                let commit_id = store.create_commit_snapshot(&commit_sha)?;
                
                let mut ts_harness = TypeScriptHarness::new()?;
                let mut py_harness = PythonHarness::new()?;
                let mut go_harness = GoHarness::new()?;
                let mut rust_harness = RustHarness::new()?;
                let mut java_harness = JavaHarness::new()?;
                let mut cpp_harness = CppHarness::new_cpp()?;
                let mut c_harness = CppHarness::new_c()?;
                let mut csharp_harness = CSharpHarness::new()?;
                let mut total_symbols = 0;
                let mut total_edges = 0;
                let mut total_lines = 0;
                
                // If incremental, delete old data for files we're reprocessing
                if incremental {
                    for file_path in &files_to_process {
                        if let Ok(relative_path) = file_path.strip_prefix(&repo_root) {
                            let path_str = relative_path.to_string_lossy();
                            store.delete_file_data(commit_id, &path_str)?;
                        }
                    }
                }
                
                // Process each file
                for file_path in &files_to_process {
                    let relative_path = file_path.strip_prefix(&repo_root)
                        .unwrap_or(file_path)
                        .to_string_lossy()
                        .to_string();

                    // Read file content with UTF-8 error handling
                    let content = match std::fs::read_to_string(file_path) {
                        Ok(c) => c,
                        Err(e) => {
                            // Check if it's a UTF-8 error (binary file with wrong extension)
                            if e.to_string().contains("invalid utf-8") ||
                               e.to_string().contains("stream did not contain valid UTF-8") {
                                warn!(
                                    "Skipping file with invalid UTF-8 (possibly binary): {} - {}",
                                    relative_path, e
                                );
                            } else {
                                warn!("Failed to read file {}: {}", relative_path, e);
                            }
                            continue;
                        }
                    };

                    let hash = FileWalker::compute_file_hash(&content);
                    let lines = content.lines().count();
                    total_lines += lines;
                    
                    // Store file information
                    store.insert_file(commit_id, &relative_path, &hash, content.len())?;
                    
                    // Parse TypeScript/JavaScript files
                    if relative_path.ends_with(".ts") || relative_path.ends_with(".tsx") ||
                       relative_path.ends_with(".js") || relative_path.ends_with(".jsx") {
                        let (symbols, edges, occurrences) = ts_harness.parse_file(
                            &content,
                            &relative_path,
                            &commit_sha
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse Python files
                    else if relative_path.ends_with(".py") {
                        let (symbols, edges, occurrences) = py_harness.parse_file(
                            &content,
                            &relative_path,
                            &commit_sha
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse Go files
                    else if relative_path.ends_with(".go") {
                        let (symbols, edges, occurrences) = go_harness.parse_file(
                            &content,
                            &relative_path,
                            &commit_sha
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse Rust files
                    else if relative_path.ends_with(".rs") {
                        let (symbols, edges, occurrences) = rust_harness.parse(
                            &relative_path,
                            &content
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse Java files
                    else if relative_path.ends_with(".java") {
                        let (symbols, edges, occurrences) = java_harness.parse(
                            &relative_path,
                            &content
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse C++ files
                    else if relative_path.ends_with(".cpp") || relative_path.ends_with(".cc")
                        || relative_path.ends_with(".cxx") || relative_path.ends_with(".hpp")
                        || relative_path.ends_with(".hh") || relative_path.ends_with(".hxx") {
                        let (symbols, edges, occurrences) = cpp_harness.parse(
                            &relative_path,
                            &content
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse C files
                    else if relative_path.ends_with(".c") || relative_path.ends_with(".h") {
                        let (symbols, edges, occurrences) = c_harness.parse(
                            &relative_path,
                            &content
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse C# files
                    else if relative_path.ends_with(".cs") {
                        let (symbols, edges, occurrences) = csharp_harness.parse_file(
                            &relative_path,
                            &content
                        )?;

                        // Batch insert for better performance
                        store.batch_insert_symbols(commit_id, &symbols)?;
                        store.batch_insert_edges(commit_id, &edges)?;
                        store.batch_insert_occurrences(commit_id, &occurrences)?;

                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                }
                
                metrics.end_phase("syntactic_analysis");
                metrics.record_lines_of_code(total_lines);
                metrics.record_file_count("total", files_to_process.len());
                metrics.record_symbol_count("total", total_symbols);
                metrics.record_edge_count("total", total_edges);
                metrics.update_memory_usage();
                
                // Run semantic analysis if enabled
                if run_semantic {
                    metrics.start_phase("semantic_analysis");
                    info!("Starting semantic analysis with SCIP indexers...");
                    
                    // Create resolution engine with store
                    let store_for_resolution = GraphStore::new(&repo_root)?;
                    let mut resolution_engine = ResolutionEngine::new(store_for_resolution);
                    
                    // Choose between incremental and full semantic analysis
                    let result = if incremental {
                        info!("Running incremental semantic analysis");
                        resolution_engine.resolve_project_incremental(&repo_root, &commit_sha).await
                    } else {
                        info!("Running full semantic analysis");
                        resolution_engine.resolve_project(&repo_root, &commit_sha).await
                    };
                    
                    // Handle result
                    match result {
                        Ok(()) => {
                            info!("Semantic analysis completed successfully");
                        },
                        Err(e) => {
                            info!("Semantic analysis encountered errors: {}", e);
                        }
                    }
                    
                    metrics.end_phase("semantic_analysis");
                    metrics.update_memory_usage();
                }
                
                let action = if incremental { "Updated" } else { "Indexed" };
                let analysis_type = if run_semantic { "semantic + syntactic" } else { "syntactic" };
                info!("{} {} files, {} symbols, {} edges ({})", action, files_to_process.len(), total_symbols, total_edges, analysis_type);
                println!("{} {} files, {} symbols, {} edges ({})", action, files_to_process.len(), total_symbols, total_edges, analysis_type);
                
                // Finalize and display performance metrics
                let _performance_metrics = metrics.finalize();
            } else {
                println!("Found {} files (dry run)", files_to_process.len());
                metrics.record_file_count("total", files_to_process.len());
                let _ = metrics.finalize();
            }
        }
        
        Commands::Show { symbol, callers, callees, importers, depth } => {
            let store = GraphStore::new(&repo_root)?;
            
            // Find the symbol
            if let Some(sym) = store.find_symbol_by_fqn(&symbol)? {
                println!("Symbol: {}", sym.name);
                println!("  Type: {:?}", sym.kind);
                println!("  FQN: {}", sym.fqn);
                println!("  File: {}:{}-{}", sym.file_path, sym.span.start_line + 1, sym.span.end_line + 1);
                
                if callers {
                    println!("\nCallers (depth={}):", depth);
                    let callers = store.get_callers(&sym.id, depth)?;
                    if callers.is_empty() {
                        println!("  (none found)");
                    } else {
                        for caller in callers {
                            println!("  - {} ({}:{})", caller.fqn, caller.file_path, caller.span.start_line + 1);
                        }
                    }
                }
                
                if callees {
                    println!("\nCallees (depth={}):", depth);
                    let callees = store.get_callees(&sym.id, depth)?;
                    if callees.is_empty() {
                        println!("  (none found)");
                    } else {
                        for callee in callees {
                            println!("  - {} ({}:{})", callee.fqn, callee.file_path, callee.span.start_line + 1);
                        }
                    }
                }
                
                if importers {
                    println!("\nImporters:");
                    println!("  (not yet implemented)");
                }
            } else {
                println!("Symbol not found: {}", symbol);
                println!("Try searching with: reviewbot search '{}'", symbol);
            }
        }
        
        Commands::Search { query, k, .. } => {
            let store = GraphStore::new(&repo_root)?;
            let results = store.search_symbols(&query, k)?;
            
            if results.is_empty() {
                println!("No symbols found matching '{}'", query);
            } else {
                println!("Found {} symbols matching '{}':", results.len(), query);
                for sym in results {
                    println!("  {} ({:?})", sym.fqn, sym.kind);
                    println!("    File: {}:{}", sym.file_path, sym.span.start_line + 1);
                }
            }
        }
        
        Commands::Graph { cmd } => {
            let store = GraphStore::new(&repo_root)?;
            
            match cmd {
                GraphCommands::Stats => {
                    let graph = store.build_graph()?;
                    let stats = graph.stats();
                    
                    println!("Graph Statistics:");
                    println!("  Nodes (symbols): {}", stats.node_count);
                    println!("  Edges (relationships): {}", stats.edge_count);
                    println!("  Has cycles: {}", if stats.is_cyclic { "Yes" } else { "No" });
                }
                
                GraphCommands::Cycles { symbol } => {
                    let graph = store.build_graph()?;
                    let cycles = graph.find_cycles_containing(&symbol);
                    
                    if cycles.is_empty() {
                        println!("No cycles found containing '{}'", symbol);
                    } else {
                        println!("Found {} cycle(s) containing '{}':", cycles.len(), symbol);
                        for (i, cycle) in cycles.iter().enumerate() {
                            println!("\nCycle {}:", i + 1);
                            for sym_id in cycle {
                                if let Some(sym) = store.find_symbol_by_id(sym_id)? {
                                    println!("  - {} ({})", sym.fqn, sym.file_path);
                                }
                            }
                        }
                    }
                }
                
                GraphCommands::Path { from, to } => {
                    let graph = store.build_graph()?;
                    
                    // Find symbols by FQN first
                    let from_sym = store.find_symbol_by_fqn(&from)?;
                    let to_sym = store.find_symbol_by_fqn(&to)?;
                    
                    if from_sym.is_none() {
                        println!("Source symbol not found: {}", from);
                        return Ok(());
                    }
                    if to_sym.is_none() {
                        println!("Target symbol not found: {}", to);
                        return Ok(());
                    }
                    
                    let from_id = from_sym.unwrap().id;
                    let to_id = to_sym.unwrap().id;
                    
                    if let Some(path) = graph.find_path(&from_id, &to_id) {
                        println!("Path from '{}' to '{}':", from, to);
                        for sym_id in path {
                            if let Some(sym) = store.find_symbol_by_id(&sym_id)? {
                                println!("  -> {} ({})", sym.fqn, sym.file_path);
                            }
                        }
                    } else {
                        println!("No path found from '{}' to '{}'", from, to);
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn get_current_commit(repo_root: &PathBuf) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_root)
        .output()?;
    
    if !output.status.success() {
        return Ok("unknown".to_string());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_changed_files(repo_root: &PathBuf, from_commit: &str, to_commit: &str) -> Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", &format!("{}..{}", from_commit, to_commit)])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        // Fallback to all files if diff fails
        return Ok(Vec::new());
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| {
            line.ends_with(".ts") || line.ends_with(".tsx") ||
            line.ends_with(".js") || line.ends_with(".jsx") ||
            line.ends_with(".py") || line.ends_with(".go") ||
            line.ends_with(".cs")
        })
        .map(|s| s.to_string())
        .collect();

    Ok(files)
}

fn get_uncommitted_changes(repo_root: &PathBuf) -> Result<Vec<String>> {
    // Get modified and staged files
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(repo_root)
        .output()?;

    let mut files = Vec::new();

    if output.status.success() {
        files.extend(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| {
                line.ends_with(".ts") || line.ends_with(".tsx") ||
                line.ends_with(".js") || line.ends_with(".jsx") ||
                line.ends_with(".py") || line.ends_with(".go") ||
                line.ends_with(".cs") || line.ends_with(".rs") ||
                line.ends_with(".java") || line.ends_with(".cpp") ||
                line.ends_with(".c") || line.ends_with(".h")
            })
            .map(|s| s.to_string()));
    }

    // Also get untracked files that match our patterns
    let output = std::process::Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        files.extend(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| {
                line.ends_with(".ts") || line.ends_with(".tsx") ||
                line.ends_with(".js") || line.ends_with(".jsx") ||
                line.ends_with(".py") || line.ends_with(".go") ||
                line.ends_with(".cs") || line.ends_with(".rs") ||
                line.ends_with(".java") || line.ends_with(".cpp") ||
                line.ends_with(".c") || line.ends_with(".h")
            })
            .map(|s| s.to_string()));
    }

    // Remove duplicates
    files.sort();
    files.dedup();

    Ok(files)
}