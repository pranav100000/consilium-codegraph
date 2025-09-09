use anyhow::Result;
use std::fs;
use std::time::Instant;
use std::path::Path;
use tempfile::TempDir;
use store::GraphStore;
use scip_mapper::ScipMapper;

#[test]
fn bench_scip_parsing_performance() -> Result<()> {
    println!("📊 SCIP Parsing Performance Benchmark");
    
    let scip_mapper = ScipMapper::new("scip-typescript", "0.3.16");
    
    // Test TypeScript SCIP parsing
    let ts_scip_path = "/Users/pranavsharan/Developer/consilium-codegraph/test_ts_project/index.scip";
    if Path::new(ts_scip_path).exists() {
        println!("\n🚀 TypeScript SCIP Processing:");
        
        let file_size = fs::metadata(ts_scip_path)?.len();
        
        // Test JSON parsing
        let parsing_start = Instant::now();
        let scip_index = scip_mapper.parse_scip_index(ts_scip_path)?;
        let parsing_time = parsing_start.elapsed();
        
        // Test IR conversion
        let conversion_start = Instant::now();
        let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "bench")?;
        let conversion_time = conversion_start.elapsed();
        
        // Test storage performance
        let temp_dir = TempDir::new()?;
        let store = GraphStore::new(temp_dir.path())?;
        let commit_id = store.get_or_create_commit("bench")?;
        
        let storage_start = Instant::now();
        for symbol in &symbols {
            let _ = store.insert_symbol(commit_id, symbol);
        }
        let storage_time = storage_start.elapsed();
        
        let total_time = parsing_time + conversion_time + storage_time;
        
        println!("  File size:     {:>8} bytes", file_size);
        println!("  Parsing:       {:>8.0}ms", parsing_time.as_millis());
        println!("  Conversion:    {:>8.0}ms", conversion_time.as_millis());
        println!("  Storage:       {:>8.0}ms", storage_time.as_millis());
        println!("  Total:         {:>8.0}ms", total_time.as_millis());
        println!("  Symbols:       {:>8}", symbols.len());
        println!("  Edges:         {:>8}", edges.len());
        println!("  Occurrences:   {:>8}", occurrences.len());
        
        // Performance assertions
        assert!(parsing_time.as_millis() < 2000, "Parsing should complete under 2s");
        assert!(conversion_time.as_millis() < 1000, "Conversion should complete under 1s");
        assert!(!symbols.is_empty(), "Should find symbols");
    }
    
    // Test Python SCIP parsing
    let py_scip_path = "/Users/pranavsharan/Developer/consilium-codegraph/test_python_project/index.scip";
    if Path::new(py_scip_path).exists() {
        println!("\n🐍 Python SCIP Processing:");
        
        let file_size = fs::metadata(py_scip_path)?.len();
        
        // Test JSON parsing
        let parsing_start = Instant::now();
        let scip_index = scip_mapper.parse_scip_index(py_scip_path)?;
        let parsing_time = parsing_start.elapsed();
        
        // Test IR conversion
        let conversion_start = Instant::now();
        let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, "bench")?;
        let conversion_time = conversion_start.elapsed();
        
        // Test storage performance
        let temp_dir = TempDir::new()?;
        let store = GraphStore::new(temp_dir.path())?;
        let commit_id = store.get_or_create_commit("bench")?;
        
        let storage_start = Instant::now();
        for symbol in &symbols {
            let _ = store.insert_symbol(commit_id, symbol);
        }
        let storage_time = storage_start.elapsed();
        
        let total_time = parsing_time + conversion_time + storage_time;
        
        println!("  File size:     {:>8} bytes", file_size);
        println!("  Parsing:       {:>8.0}ms", parsing_time.as_millis());
        println!("  Conversion:    {:>8.0}ms", conversion_time.as_millis());
        println!("  Storage:       {:>8.0}ms", storage_time.as_millis());
        println!("  Total:         {:>8.0}ms", total_time.as_millis());
        println!("  Symbols:       {:>8}", symbols.len());
        println!("  Edges:         {:>8}", edges.len());
        println!("  Occurrences:   {:>8}", occurrences.len());
        
        // Performance assertions
        assert!(parsing_time.as_millis() < 3000, "Parsing should complete under 3s");
        assert!(conversion_time.as_millis() < 1000, "Conversion should complete under 1s");
        assert!(!symbols.is_empty(), "Should find symbols");
    }
    
    println!("\n✅ SCIP Performance Benchmark Complete!");
    Ok(())
}

#[test]
fn bench_scip_large_file_performance() -> Result<()> {
    println!("🔍 Large SCIP File Performance Test");
    
    // Find the larger SCIP file to test with
    let ts_scip_path = "/Users/pranavsharan/Developer/consilium-codegraph/test_ts_project/index.scip";
    let py_scip_path = "/Users/pranavsharan/Developer/consilium-codegraph/test_python_project/index.scip";
    
    let mut test_file = None;
    let mut test_lang = "";
    
    if Path::new(ts_scip_path).exists() {
        let ts_size = fs::metadata(ts_scip_path)?.len();
        test_file = Some((ts_scip_path, ts_size));
        test_lang = "TypeScript";
    }
    
    if Path::new(py_scip_path).exists() {
        let py_size = fs::metadata(py_scip_path)?.len();
        if test_file.is_none() || py_size > test_file.unwrap().1 {
            test_file = Some((py_scip_path, py_size));
            test_lang = "Python";
        }
    }
    
    if let Some((file_path, file_size)) = test_file {
        println!("Testing with {} SCIP file ({} bytes)", test_lang, file_size);
        
        let scip_mapper = ScipMapper::new("test", "1.0");
        
        // Multiple runs to test consistency
        let mut parse_times = Vec::new();
        let mut convert_times = Vec::new();
        
        for run in 1..=3 {
            println!("\nRun {}:", run);
            
            let parsing_start = Instant::now();
            let scip_index = scip_mapper.parse_scip_index(file_path)?;
            let parsing_time = parsing_start.elapsed();
            parse_times.push(parsing_time);
            
            let conversion_start = Instant::now();
            let (symbols, edges, occurrences) = scip_mapper.map_scip_to_ir(&scip_index, &format!("bench_{}", run))?;
            let conversion_time = conversion_start.elapsed();
            convert_times.push(conversion_time);
            
            println!("  Parsing:    {:>6.0}ms", parsing_time.as_millis());
            println!("  Conversion: {:>6.0}ms", conversion_time.as_millis());
            println!("  Symbols:    {:>6}", symbols.len());
            println!("  Edges:      {:>6}", edges.len());
            println!("  Occurrences:{:>6}", occurrences.len());
        }
        
        // Calculate averages
        let avg_parse = parse_times.iter().sum::<std::time::Duration>().as_millis() / parse_times.len() as u128;
        let avg_convert = convert_times.iter().sum::<std::time::Duration>().as_millis() / convert_times.len() as u128;
        
        println!("\n📈 Performance Summary:");
        println!("  Average parsing:    {}ms", avg_parse);
        println!("  Average conversion: {}ms", avg_convert);
        println!("  File size:          {} bytes", file_size);
        println!("  Throughput:         {:.1} KB/s", (file_size as f64) / (avg_parse as f64 / 1000.0) / 1024.0);
        
        // Performance targets
        assert!(avg_parse < 5000, "Average parsing should be under 5s");
        assert!(avg_convert < 2000, "Average conversion should be under 2s");
    } else {
        println!("No SCIP files found for testing");
    }
    
    Ok(())
}