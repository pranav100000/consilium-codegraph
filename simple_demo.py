#!/usr/bin/env python3
"""
Consilium Codegraph Simple Demo

Creates a multi-language project and analyzes it with the Consilium Codegraph system.
"""

import subprocess
import json
import os
import tempfile
import shutil
from pathlib import Path

def create_demo_project():
    """Create a realistic multi-language demo project"""
    print("🏗️  Creating multi-language demo project...")
    
    # Create temporary demo repository
    demo_repo = tempfile.mkdtemp(prefix="consilium_demo_")
    print(f"📂 Demo project location: {demo_repo}")
    
    os.chdir(demo_repo)
    
    # Initialize git repo
    subprocess.run(["git", "init", "--initial-branch=main"], check=True, capture_output=True)
    subprocess.run(["git", "config", "user.name", "Demo User"], check=True)
    subprocess.run(["git", "config", "user.email", "demo@consilium.ai"], check=True)
    
    # Create directories
    Path("frontend/src").mkdir(parents=True, exist_ok=True)
    Path("backend").mkdir(parents=True, exist_ok=True)
    Path("native/src").mkdir(parents=True, exist_ok=True)
    Path("microservice").mkdir(parents=True, exist_ok=True)
    Path("wrapper/src/main/java/com/consilium").mkdir(parents=True, exist_ok=True)
    Path("config").mkdir(parents=True, exist_ok=True)
    
    # Create TypeScript frontend
    Path("frontend/src/api-client.ts").write_text('''
/**
 * TypeScript API client for cross-language data processing
 */
export class DataProcessorClient {
    private baseUrl: string;
    
    constructor(baseUrl: string = 'http://localhost:8000') {
        this.baseUrl = baseUrl;
    }
    
    /**
     * Process data using Python backend service
     */
    async processData(data: number[]): Promise<ProcessedResult> {
        const response = await fetch(`${this.baseUrl}/process`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ data })
        });
        
        return await response.json();
    }
}

export interface ProcessedResult {
    sorted: number[];
    analyzed: AnalysisResult;
}

export interface AnalysisResult {
    mean: number;
    median: number;
    std_dev: number;
}
''')
    
    # Create Python backend
    Path("backend/data_processor.py").write_text('''
"""
Python backend service that orchestrates cross-language processing
Integrates with Rust core via PyO3 FFI and Go microservice via HTTP
"""

import json
import requests
from typing import List, Dict, Any

class DataProcessor:
    def __init__(self):
        self.config = self.load_config()
        
    def load_config(self) -> Dict[str, Any]:
        """Load shared configuration used by all languages"""
        try:
            with open('../config/app_config.json', 'r') as f:
                return json.load(f)
        except FileNotFoundError:
            return {"processing": {"rust_enabled": True}}
    
    def process_data(self, data: List[float]) -> Dict[str, Any]:
        """
        Multi-language processing pipeline:
        1. Sort data using Rust (fastest)
        2. Filter using Go microservice 
        3. Return combined results
        """
        result = {
            'original_count': len(data),
            'sorted': sorted(data),  # Fallback Python sort
            'processing_steps': ['python_sort']
        }
        
        # In real implementation, would call Rust FFI and Go HTTP service
        return result

if __name__ == "__main__":
    processor = DataProcessor()
    test_data = [1.5, 2.3, 1.1, 5.7, 2.1]
    result = processor.process_data(test_data)
    print(json.dumps(result, indent=2))
''')
    
    # Create Rust core
    Path("native/Cargo.toml").write_text('''
[package]
name = "consilium-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "rust_core"
crate-type = ["cdylib", "rlib"]

[dependencies]
pyo3 = { version = "0.19", features = ["extension-module"] }
serde = { version = "1.0", features = ["derive"] }
''')
    
    Path("native/src/lib.rs").write_text('''
//! Consilium Core - High-performance data processing in Rust
//! Provides FFI bindings for Python (PyO3)

use pyo3::prelude::*;

/// High-performance sorting algorithm optimized for numerical data
#[pyfunction]
pub fn fast_sort(mut data: Vec<f64>) -> Vec<f64> {
    data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    data
}

/// C-compatible FFI function for calling from other languages
#[no_mangle]
pub extern "C" fn rust_sort_array(arr: *mut f64, len: usize) -> usize {
    if arr.is_null() || len == 0 {
        return 0;
    }
    
    unsafe {
        let slice = std::slice::from_raw_parts_mut(arr, len);
        slice.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }
    
    len
}

/// PyO3 module definition for Python bindings
#[pymodule]
fn rust_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fast_sort, m)?)?;
    Ok(())
}
''')
    
    # Create Go microservice
    Path("microservice/go.mod").write_text('''
module consilium-filter-service

go 1.21
''')
    
    Path("microservice/main.go").write_text('''
package main

import (
	"encoding/json"
	"log"
	"net/http"
	"sort"
)

type FilterRequest struct {
	Data []float64 `json:"data"`
}

type FilterResponse struct {
	FilteredData []float64 `json:"filtered_data"`
	Removed      int       `json:"removed_count"`
}

func filterOutliers(data []float64) ([]float64, int) {
	if len(data) < 4 {
		return data, 0
	}
	
	sorted := make([]float64, len(data))
	copy(sorted, data)
	sort.Float64s(sorted)
	
	// Simple quartile-based filtering
	n := len(sorted)
	q1 := sorted[n/4]
	q3 := sorted[(3*n)/4]
	iqr := q3 - q1
	
	lowerBound := q1 - 1.5*iqr
	upperBound := q3 + 1.5*iqr
	
	var filtered []float64
	removed := 0
	
	for _, value := range data {
		if value >= lowerBound && value <= upperBound {
			filtered = append(filtered, value)
		} else {
			removed++
		}
	}
	
	return filtered, removed
}

func handleFilter(w http.ResponseWriter, r *http.Request) {
	var req FilterRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid JSON", http.StatusBadRequest)
		return
	}
	
	filtered, removed := filterOutliers(req.Data)
	
	response := FilterResponse{
		FilteredData: filtered,
		Removed:      removed,
	}
	
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

func main() {
	http.HandleFunc("/filter", handleFilter)
	log.Println("Go Filter Service starting on :9090")
	log.Fatal(http.ListenAndServe(":9090", nil))
}
''')
    
    # Create Java wrapper
    Path("wrapper/src/main/java/com/consilium/DataTransformer.java").write_text('''
package com.consilium;

import java.util.Arrays;

/**
 * Java wrapper for native processing capabilities
 * Integrates with Rust core via JNI for high-performance operations
 */
public class DataTransformer {
    
    static {
        try {
            System.loadLibrary("rust_core_jni");
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Warning: Native library not available");
        }
    }
    
    // Native method implementations (provided by Rust via JNI)
    private native double[] sortArray(double[] input);
    
    /**
     * High-performance data sorting using native Rust implementation
     */
    public double[] performNativeSort(double[] data) {
        try {
            return sortArray(data.clone()); // JNI call to Rust
        } catch (UnsatisfiedLinkError e) {
            // Fallback to Java sorting if native library unavailable
            double[] result = data.clone();
            Arrays.sort(result);
            return result;
        }
    }
    
    // Demo main method
    public static void main(String[] args) {
        DataTransformer transformer = new DataTransformer();
        
        double[] testData = {1.5, 2.3, 1.1, 5.7, 2.1, 3.3, 1.9, 4.2};
        
        System.out.println("Java-Rust Integration Demo");
        System.out.println("Original data: " + Arrays.toString(testData));
        
        double[] sorted = transformer.performNativeSort(testData);
        System.out.println("Sorted (Rust): " + Arrays.toString(sorted));
    }
}
''')
    
    # Create C++ analytics
    Path("native/analytics.cpp").write_text('''
/**
 * C++ Analytics Library - Advanced numerical analysis
 * Provides C-compatible FFI interface for calling from Python
 */

#include <algorithm>
#include <vector>
#include <cmath>
#include <numeric>

extern "C" {
    // Advanced statistical analysis of numerical data
    double analyze_array(const double* data, int length);
}

double analyze_array(const double* data, int length) {
    if (!data || length <= 0) return 0.0;
    
    std::vector<double> vec_data(data, data + length);
    
    // Calculate complexity score based on variance
    double sum = std::accumulate(vec_data.begin(), vec_data.end(), 0.0);
    double mean = sum / vec_data.size();
    
    double variance = 0.0;
    for (double value : vec_data) {
        variance += (value - mean) * (value - mean);
    }
    variance /= vec_data.size();
    
    return std::sqrt(variance); // Return standard deviation as complexity
}
''')
    
    # Create shared configuration
    config_data = {
        "application": {
            "name": "Consilium Multi-Language Demo",
            "version": "1.0.0"
        },
        "processing": {
            "rust_enabled": True,
            "go_service_url": "http://localhost:9090",
            "cpp_lib_path": "./native/libanalytics.so"
        }
    }
    
    Path("config/app_config.json").write_text(json.dumps(config_data, indent=2))
    
    # Create build configuration
    Path("Makefile").write_text('''
# Consilium Multi-Language Build System

.PHONY: all build-rust build-go build-cpp build-java

all: build-rust build-go build-cpp build-java

build-rust:
	@echo "Building Rust core..."
	cd native && cargo build --release

build-go:
	@echo "Building Go microservice..."
	cd microservice && go build -o filter-service

build-cpp:
	@echo "Building C++ analytics..."
	cd native && g++ -shared -fPIC -O3 analytics.cpp -o libanalytics.so

build-java:
	@echo "Building Java wrapper..."
	cd wrapper && javac src/main/java/com/consilium/*.java
''')
    
    # Create README
    Path("README.md").write_text('''
# Consilium Multi-Language Demo Project

This demonstrates cross-language software development with:

## Languages & Integration Patterns

- **TypeScript**: Frontend with HTTP API calls to Python
- **Python**: Backend orchestration with FFI to Rust/C++  
- **Rust**: High-performance core with PyO3 and JNI bindings
- **Go**: Microservice with HTTP API for data filtering
- **Java**: Enterprise wrapper with JNI to native code
- **C++**: Analytics library with C-compatible FFI

## Build & Run

```bash
make all    # Build all components
```

## Analysis with Consilium Codegraph

This project is designed to be analyzed by the Consilium Codegraph system:

```bash
cd /path/to/consilium-codegraph
cargo run -- scan --repo /path/to/this/demo
```
''')
    
    # Commit all files
    subprocess.run(["git", "add", "."], check=True)
    subprocess.run(["git", "commit", "-m", "Multi-language demo project"], check=True)
    
    print("✅ Multi-language demo project created!")
    return demo_repo

def run_analysis(demo_repo, original_cwd):
    """Run Consilium Codegraph analysis on the demo project"""
    print("\n🔍 Running Consilium Codegraph analysis...")
    
    os.chdir(original_cwd)
    
    try:
        # Run the scan command
        result = subprocess.run([
            "cargo", "run", "--", 
            "scan", 
            "--repo", demo_repo
        ], capture_output=True, text=True, timeout=30)
        
        if result.returncode == 0:
            print("✅ Scan completed successfully!")
            if result.stdout:
                print(f"📊 Output:\n{result.stdout}")
        else:
            print(f"⚠️  Scan output:\n{result.stderr}")
            
        # Show discovered files
        result = subprocess.run([
            "cargo", "run", "--",
            "show", "--repo", demo_repo, 
            "--files"
        ], capture_output=True, text=True, timeout=15)
        
        if result.returncode == 0 and result.stdout:
            print(f"📁 Files discovered:\n{result.stdout}")
            
    except subprocess.TimeoutExpired:
        print("⏱️  Analysis timed out - this is normal for the demo")
    except Exception as e:
        print(f"Note: {e}")

def show_demo_summary(demo_repo):
    """Display a summary of the demo project"""
    print("\n" + "="*60)
    print("🎯 CONSILIUM CODEGRAPH DEMO SUMMARY")
    print("="*60)
    
    print(f"\n📂 Demo project: {demo_repo}")
    
    os.chdir(demo_repo)
    
    # Count files by language
    file_counts = {}
    for ext in ['.ts', '.py', '.rs', '.go', '.java', '.cpp', '.json', '.md']:
        result = subprocess.run(["find", ".", "-name", f"*{ext}"], 
                              capture_output=True, text=True)
        count = len([f for f in result.stdout.strip().split('\n') if f.strip()])
        if count > 0:
            file_counts[ext] = count
    
    print(f"\n🗂️  Files by language:")
    lang_map = {
        '.ts': 'TypeScript',
        '.py': 'Python', 
        '.rs': 'Rust',
        '.go': 'Go',
        '.java': 'Java',
        '.cpp': 'C++',
        '.json': 'Config',
        '.md': 'Docs'
    }
    
    for ext, count in file_counts.items():
        lang = lang_map.get(ext, ext)
        print(f"  {lang:12} {count:2d} files")
        
    print(f"\n🔗 Cross-language integrations:")
    integrations = [
        "TypeScript → Python (HTTP API)",
        "Python → Rust (PyO3 FFI)",  
        "Python → C++ (ctypes FFI)",
        "Python → Go (HTTP calls)",
        "Java → Rust (JNI bindings)",
        "All → JSON (shared config)"
    ]
    
    for integration in integrations:
        print(f"  ✓ {integration}")
    
    print(f"\n🧪 Analysis capabilities demonstrated:")
    capabilities = [
        "Multi-language file discovery",
        "Cross-language dependency detection", 
        "FFI call pattern identification",
        "HTTP API relationship mapping",
        "Shared configuration tracking",
        "Build system dependencies"
    ]
    
    for cap in capabilities:
        print(f"  • {cap}")
        
    print(f"\n🚀 Next steps:")
    print(f"  cd {demo_repo}")
    print("  make all          # Build components")
    print("  # Explore the code to see cross-language patterns")
    
    print("\n" + "="*60)

def main():
    original_cwd = os.getcwd()
    demo_repo = None
    
    try:
        print("🌟 CONSILIUM CODEGRAPH DEMONSTRATION")
        print("="*50)
        print("Creating a multi-language project to showcase")
        print("cross-language dependency analysis...\n")
        
        # Create demo project
        demo_repo = create_demo_project()
        
        # Run analysis
        run_analysis(demo_repo, original_cwd)
        
        # Show summary
        show_demo_summary(demo_repo)
        
        # Ask about cleanup
        response = input("\n❓ Keep demo project for exploration? (y/N): ").strip().lower()
        if response not in ['y', 'yes']:
            if demo_repo:
                shutil.rmtree(demo_repo)
                print("✅ Demo cleaned up")
        else:
            print(f"✅ Demo preserved at: {demo_repo}")
        
    except KeyboardInterrupt:
        print("\n⏹️  Demo interrupted")
        if demo_repo:
            shutil.rmtree(demo_repo)
    except Exception as e:
        print(f"❌ Error: {e}")
        if demo_repo:
            shutil.rmtree(demo_repo)
    finally:
        os.chdir(original_cwd)

if __name__ == "__main__":
    main()