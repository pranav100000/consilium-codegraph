use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use store::GraphStore;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ts_harness::TypeScriptHarness;
use py_harness::PythonHarness;
use go_harness::GoHarness;
use rust_harness::RustHarness;
use java_harness::JavaHarness;
use cpp_harness::CppHarness;

mod walker;
use walker::FileWalker;

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
        Commands::Scan { no_write, .. } => {
            if no_write {
                info!("Running scan in dry-run mode (--no-write)");
            }
            
            let commit_sha = get_current_commit(&repo_root)?;
            info!("Scanning repository at commit: {}", commit_sha);
            
            // Check for incremental scan opportunity
            let mut files_to_process = Vec::new();
            let mut incremental = false;
            
            if !no_write {
                let store = GraphStore::new(&repo_root)?;
                if let Some(last_commit) = store.get_last_scanned_commit()? {
                    if last_commit != commit_sha {
                        // Get changed files since last scan
                        let changed = get_changed_files(&repo_root, &last_commit, &commit_sha)?;
                        if !changed.is_empty() && changed.len() < 100 {  // Arbitrary threshold
                            info!("Incremental scan: {} files changed since {}", changed.len(), &last_commit[0..7]);
                            
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
                        }
                    } else {
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
            
            if files_to_process.is_empty() {
                println!("No files found to index");
                return Ok(());
            }
            
            if !no_write {
                let store = GraphStore::new(&repo_root)?;
                let commit_id = store.create_commit_snapshot(&commit_sha)?;
                
                let mut ts_harness = TypeScriptHarness::new()?;
                let mut py_harness = PythonHarness::new()?;
                let mut go_harness = GoHarness::new()?;
                let mut rust_harness = RustHarness::new()?;
                let mut java_harness = JavaHarness::new()?;
                let mut cpp_harness = CppHarness::new_cpp()?;
                let mut c_harness = CppHarness::new_c()?;
                let mut total_symbols = 0;
                let mut total_edges = 0;
                
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
                    
                    let content = std::fs::read_to_string(file_path)?;
                    let hash = FileWalker::compute_file_hash(&content);
                    
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
                        
                        // Store symbols
                        for symbol in &symbols {
                            store.insert_symbol(commit_id, symbol)?;
                        }
                        
                        // Store edges
                        for edge in &edges {
                            store.insert_edge(commit_id, edge)?;
                        }
                        
                        // Store occurrences
                        for occurrence in &occurrences {
                            store.insert_occurrence(commit_id, occurrence)?;
                        }
                        
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
                        
                        // Store symbols
                        for symbol in &symbols {
                            store.insert_symbol(commit_id, symbol)?;
                        }
                        
                        // Store edges
                        for edge in &edges {
                            store.insert_edge(commit_id, edge)?;
                        }
                        
                        // Store occurrences
                        for occurrence in &occurrences {
                            store.insert_occurrence(commit_id, occurrence)?;
                        }
                        
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
                        
                        // Store symbols
                        for symbol in &symbols {
                            store.insert_symbol(commit_id, symbol)?;
                        }
                        
                        // Store edges
                        for edge in &edges {
                            store.insert_edge(commit_id, edge)?;
                        }
                        
                        // Store occurrences
                        for occurrence in &occurrences {
                            store.insert_occurrence(commit_id, occurrence)?;
                        }
                        
                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse Rust files
                    else if relative_path.ends_with(".rs") {
                        let (symbols, edges, occurrences) = rust_harness.parse(
                            &relative_path,
                            &content
                        )?;
                        
                        // Store symbols
                        for symbol in &symbols {
                            store.insert_symbol(commit_id, symbol)?;
                        }
                        
                        // Store edges
                        for edge in &edges {
                            store.insert_edge(commit_id, edge)?;
                        }
                        
                        // Store occurrences
                        for occurrence in &occurrences {
                            store.insert_occurrence(commit_id, occurrence)?;
                        }
                        
                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse Java files
                    else if relative_path.ends_with(".java") {
                        let (symbols, edges, occurrences) = java_harness.parse(
                            &relative_path,
                            &content
                        )?;
                        
                        // Store symbols
                        for symbol in &symbols {
                            store.insert_symbol(commit_id, symbol)?;
                        }
                        
                        // Store edges
                        for edge in &edges {
                            store.insert_edge(commit_id, edge)?;
                        }
                        
                        // Store occurrences
                        for occurrence in &occurrences {
                            store.insert_occurrence(commit_id, occurrence)?;
                        }
                        
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
                        
                        // Store symbols
                        for symbol in &symbols {
                            store.insert_symbol(commit_id, symbol)?;
                        }
                        
                        // Store edges
                        for edge in &edges {
                            store.insert_edge(commit_id, edge)?;
                        }
                        
                        // Store occurrences
                        for occurrence in &occurrences {
                            store.insert_occurrence(commit_id, occurrence)?;
                        }
                        
                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                    // Parse C files
                    else if relative_path.ends_with(".c") || relative_path.ends_with(".h") {
                        let (symbols, edges, occurrences) = c_harness.parse(
                            &relative_path,
                            &content
                        )?;
                        
                        // Store symbols
                        for symbol in &symbols {
                            store.insert_symbol(commit_id, symbol)?;
                        }
                        
                        // Store edges
                        for edge in &edges {
                            store.insert_edge(commit_id, edge)?;
                        }
                        
                        // Store occurrences
                        for occurrence in &occurrences {
                            store.insert_occurrence(commit_id, occurrence)?;
                        }
                        
                        total_symbols += symbols.len();
                        total_edges += edges.len();
                    }
                }
                
                let action = if incremental { "Updated" } else { "Indexed" };
                info!("{} {} files, {} symbols, {} edges", action, files_to_process.len(), total_symbols, total_edges);
                println!("{} {} files, {} symbols, {} edges", action, files_to_process.len(), total_symbols, total_edges);
            } else {
                println!("Found {} files (dry run)", files_to_process.len());
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
            line.ends_with(".py") || line.ends_with(".go")
        })
        .map(|s| s.to_string())
        .collect();
    
    Ok(files)
}