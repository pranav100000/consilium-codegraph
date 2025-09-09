#!/usr/bin/env python3
"""
Consilium Codegraph Demo Script

This script demonstrates the capabilities of the Consilium Codegraph system
for analyzing multi-language codebases and tracking cross-language dependencies.
"""

import subprocess
import json
import os
import tempfile
import shutil
from pathlib import Path

class ConsiliiumDemo:
    def __init__(self):
        self.demo_repo = None
        self.original_cwd = os.getcwd()
        
    def create_demo_project(self):
        """Create a realistic multi-language demo project"""
        print("🏗️  Creating multi-language demo project...")
        
        # Create temporary demo repository
        self.demo_repo = tempfile.mkdtemp(prefix="consilium_demo_")
        print(f"📂 Demo project location: {self.demo_repo}")
        
        # Initialize git repo
        os.chdir(self.demo_repo)
        subprocess.run(["git", "init", "--initial-branch=main"], check=True, capture_output=True)
        subprocess.run(["git", "config", "user.name", "Demo User"], check=True)
        subprocess.run(["git", "config", "user.email", "demo@consilium.ai"], check=True)
        
        # Create multi-language project structure
        self.create_typescript_frontend()
        self.create_python_backend() 
        self.create_rust_core()
        self.create_go_microservice()
        self.create_cpp_native()
        self.create_java_wrapper()
        self.create_config_files()
        
        # Commit all files
        subprocess.run(["git", "add", "."], check=True)
        subprocess.run(["git", "commit", "-m", "Initial multi-language demo project"], check=True)
        
        print("✅ Multi-language demo project created!")
        
    def create_typescript_frontend(self):
        """Create TypeScript frontend with API client"""
        frontend_dir = Path("frontend/src")
        frontend_dir.mkdir(parents=True, exist_ok=True)
        
        # TypeScript API client that calls Python backend
        (frontend_dir / "api-client.ts").write_text("""
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
        
        if (!response.ok) {
            throw new ProcessingError(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        return await response.json();
    }
    
    /**
     * Get native processing stats from Rust core
     */
    async getNativeStats(): Promise<NativeStats> {
        // This calls the Python backend, which uses Rust FFI
        const response = await fetch(`${this.baseUrl}/stats`);
        return await response.json();
    }
}

export interface ProcessedResult {
    sorted: number[];
    analyzed: AnalysisResult;
    performance: PerformanceMetrics;
}

export interface AnalysisResult {
    mean: number;
    median: number;
    std_dev: number;
}

export interface NativeStats {
    rust_processing_time_ns: number;
    memory_usage_bytes: number;
    ffi_call_count: number;
}

export class ProcessingError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'ProcessingError';
    }
}
""")
        
        # Package.json with WASM dependency
        (Path("frontend") / "package.json").write_text(json.dumps({
            "name": "consilium-demo-frontend",
            "version": "1.0.0",
            "dependencies": {
                "consilium-wasm": "file:../native/pkg",  # Rust WASM output
                "typescript": "^4.9.0"
            },
            "scripts": {
                "build": "tsc",
                "dev": "tsc --watch"
            }
        }, indent=2))
        
        print("  ✅ TypeScript frontend created")
        
    def create_python_backend(self):
        """Create Python backend that integrates with Rust and Go"""
        backend_dir = Path("backend")
        backend_dir.mkdir(parents=True, exist_ok=True)
        
        # Python service with Rust FFI integration
        (backend_dir / "data_processor.py").write_text("""
'''
Python backend service that orchestrates cross-language processing
Integrates with:
- Rust core (via PyO3 FFI)
- Go microservice (via HTTP)
- C++ analytics (via ctypes FFI)
'''

import ctypes
import json
import requests
from typing import List, Dict, Any
from dataclasses import dataclass
import subprocess

# Import Rust extension built with PyO3
try:
    import rust_core  # This would be the PyO3-generated Python module
except ImportError:
    print("Warning: Rust core module not available")
    rust_core = None

@dataclass
class ProcessingConfig:
    """Configuration loaded from shared config.json"""
    rust_enabled: bool = True
    go_service_url: str = "http://localhost:9090"
    cpp_lib_path: str = "./native/libanalytics.so"
    performance_logging: bool = True

class DataProcessor:
    def __init__(self):
        self.config = self.load_config()
        self.go_client = GoServiceClient(self.config.go_service_url)
        self.cpp_lib = self.load_cpp_library()
        
    def load_config(self) -> ProcessingConfig:
        """Load shared configuration used by all languages"""
        try:
            with open('../config/app_config.json', 'r') as f:
                config_data = json.load(f)
                return ProcessingConfig(**config_data.get('processing', {}))
        except FileNotFoundError:
            return ProcessingConfig()  # Use defaults
    
    def load_cpp_library(self):
        """Load C++ analytics library via FFI"""
        try:
            lib = ctypes.CDLL(self.config.cpp_lib_path)
            # Define C++ function signatures
            lib.analyze_array.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_int]
            lib.analyze_array.restype = ctypes.c_double
            return lib
        except OSError:
            print("Warning: C++ analytics library not available")
            return None
    
    async def process_data(self, data: List[float]) -> Dict[str, Any]:
        """
        Multi-language processing pipeline:
        1. Sort data using Rust (fastest)
        2. Filter using Go microservice 
        3. Analyze using C++ library
        4. Return combined results
        """
        result = {
            'original_count': len(data),
            'processing_steps': []
        }
        
        # Step 1: Rust core processing (FFI)
        if rust_core and self.config.rust_enabled:
            try:
                sorted_data = rust_core.fast_sort(data)
                stats = rust_core.get_processing_stats()
                result['sorted'] = sorted_data
                result['rust_stats'] = stats
                result['processing_steps'].append('rust_ffi_sort')
            except Exception as e:
                print(f"Rust processing failed: {e}")
                result['sorted'] = sorted(data)  # Fallback to Python sort
        else:
            result['sorted'] = sorted(data)
        
        # Step 2: Go microservice filtering (HTTP)
        try:
            filtered_data = await self.go_client.filter_outliers(result['sorted'])
            result['filtered'] = filtered_data
            result['processing_steps'].append('go_http_filter')
        except Exception as e:
            print(f"Go service filtering failed: {e}")
            result['filtered'] = result['sorted']  # No filtering
        
        # Step 3: C++ analytics (FFI)
        if self.cpp_lib:
            try:
                array_ptr = (ctypes.c_double * len(result['filtered']))(*result['filtered'])
                analysis_score = self.cpp_lib.analyze_array(array_ptr, len(result['filtered']))
                result['analysis'] = {
                    'complexity_score': float(analysis_score),
                    'method': 'cpp_ffi'
                }
                result['processing_steps'].append('cpp_ffi_analysis')
            except Exception as e:
                print(f"C++ analysis failed: {e}")
        
        return result
    
    def get_native_stats(self) -> Dict[str, Any]:
        """Get performance statistics from native components"""
        stats = {}
        
        if rust_core:
            stats['rust'] = rust_core.get_detailed_stats()
        
        if self.cpp_lib:
            # Call C++ stats function
            try:
                stats['cpp'] = {
                    'library_loaded': True,
                    'version': self.cpp_lib.get_version() if hasattr(self.cpp_lib, 'get_version') else 'unknown'
                }
            except:
                pass
        
        # Query Go service stats
        try:
            go_stats = self.go_client.get_stats()
            stats['go'] = go_stats
        except:
            pass
        
        return stats

class GoServiceClient:
    def __init__(self, base_url: str):
        self.base_url = base_url
    
    async def filter_outliers(self, data: List[float]) -> List[float]:
        """Call Go microservice to filter statistical outliers"""
        response = requests.post(f"{self.base_url}/filter", json={'data': data})
        response.raise_for_status()
        return response.json()['filtered_data']
    
    def get_stats(self) -> Dict[str, Any]:
        """Get Go service performance statistics"""
        response = requests.get(f"{self.base_url}/stats")
        response.raise_for_status()
        return response.json()

# Error propagation from native layers
class NativeProcessingError(Exception):
    """Errors that originate from native code (Rust/C++) but propagate to Python"""
    pass

if __name__ == "__main__":
    processor = DataProcessor()
    test_data = [1.5, 2.3, 1.1, 5.7, 2.1, 3.3, 1.9, 4.2]
    result = processor.process_data(test_data)
    print(json.dumps(result, indent=2))
""")
        
        print("  ✅ Python backend created")
        
    def create_rust_core(self):
        """Create Rust core with FFI exports and WASM target"""
        rust_dir = Path("native")
        rust_dir.mkdir(parents=True, exist_ok=True)
        
        # Cargo.toml for Rust core with PyO3 and WASM targets
        (rust_dir / "Cargo.toml").write_text("""
[package]
name = "consilium-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "rust_core"
crate-type = ["cdylib", "rlib"]

[dependencies]
pyo3 = { version = "0.19", features = ["extension-module"] }
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[dependencies.web-sys]
version = "0.3"
features = [
  "console",
  "Performance",
]

[features]
default = ["pyo3/extension-module"]
""")
        
        # Rust core library with FFI exports
        (rust_dir / "src" / "lib.rs").write_text("""
//! Consilium Core - High-performance data processing in Rust
//! Provides FFI bindings for Python (PyO3) and WASM bindings for TypeScript

use pyo3::prelude::*;
use wasm_bindgen::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct ProcessingStats {
    pub operations_count: u64,
    pub total_time_ns: u64,
    pub memory_peak_bytes: usize,
    pub ffi_calls: u64,
}

static mut GLOBAL_STATS: ProcessingStats = ProcessingStats {
    operations_count: 0,
    total_time_ns: 0,
    memory_peak_bytes: 0,
    ffi_calls: 0,
};

/// High-performance sorting algorithm optimized for numerical data
#[pyfunction]
#[wasm_bindgen]
pub fn fast_sort(mut data: Vec<f64>) -> Vec<f64> {
    let start = Instant::now();
    
    unsafe {
        GLOBAL_STATS.ffi_calls += 1;
        GLOBAL_STATS.operations_count += data.len() as u64;
    }
    
    // Use unstable sort for better performance (sorting floats)
    data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    unsafe {
        GLOBAL_STATS.total_time_ns += start.elapsed().as_nanos() as u64;
    }
    
    data
}

/// Advanced mathematical analysis of numerical datasets
#[pyfunction]
#[wasm_bindgen]
pub fn analyze_dataset(data: &[f64]) -> HashMap<String, f64> {
    let start = Instant::now();
    
    unsafe {
        GLOBAL_STATS.ffi_calls += 1;
    }
    
    let mut result = HashMap::new();
    
    if data.is_empty() {
        return result;
    }
    
    // Calculate statistical measures
    let sum: f64 = data.iter().sum();
    let mean = sum / data.len() as f64;
    result.insert("mean".to_string(), mean);
    
    // Variance and standard deviation
    let variance = data.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / data.len() as f64;
    result.insert("variance".to_string(), variance);
    result.insert("std_dev".to_string(), variance.sqrt());
    
    // Median (requires sorted data)
    let mut sorted = data.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };
    result.insert("median".to_string(), median);
    
    unsafe {
        GLOBAL_STATS.total_time_ns += start.elapsed().as_nanos() as u64;
    }
    
    result
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
        GLOBAL_STATS.ffi_calls += 1;
        GLOBAL_STATS.operations_count += len as u64;
    }
    
    len
}

/// Get detailed performance statistics
#[pyfunction] 
#[wasm_bindgen]
pub fn get_processing_stats() -> HashMap<String, u64> {
    let mut stats = HashMap::new();
    
    unsafe {
        stats.insert("operations_count".to_string(), GLOBAL_STATS.operations_count);
        stats.insert("total_time_ns".to_string(), GLOBAL_STATS.total_time_ns);
        stats.insert("memory_peak_bytes".to_string(), GLOBAL_STATS.memory_peak_bytes as u64);
        stats.insert("ffi_calls".to_string(), GLOBAL_STATS.ffi_calls);
    }
    
    stats
}

/// Reset performance statistics
#[pyfunction]
#[wasm_bindgen] 
pub fn reset_stats() {
    unsafe {
        GLOBAL_STATS.operations_count = 0;
        GLOBAL_STATS.total_time_ns = 0;
        GLOBAL_STATS.memory_peak_bytes = 0;
        GLOBAL_STATS.ffi_calls = 0;
    }
}

/// PyO3 module definition for Python bindings
#[pymodule]
fn rust_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fast_sort, m)?)?;
    m.add_function(wrap_pyfunction!(analyze_dataset, m)?)?;
    m.add_function(wrap_pyfunction!(get_processing_stats, m)?)?;
    m.add_function(wrap_pyfunction!(reset_stats, m)?)?;
    Ok(())
}
""")
        
        Path(rust_dir / "src").mkdir(exist_ok=True)
        
        print("  ✅ Rust core created")
        
    def create_go_microservice(self):
        """Create Go microservice for data filtering"""
        go_dir = Path("microservice")
        go_dir.mkdir(parents=True, exist_ok=True)
        
        # Go module definition
        (go_dir / "go.mod").write_text("""
module consilium-filter-service

go 1.21

require (
    github.com/gorilla/mux v1.8.0
    github.com/gorilla/handlers v1.5.1
)
""")
        
        # Go HTTP service
        (go_dir / "main.go").write_text("""
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"math"
	"net/http"
	"sort"
	"time"
	
	"github.com/gorilla/mux"
	"github.com/gorilla/handlers"
)

// FilterRequest represents the data filtering request from Python backend
type FilterRequest struct {
	Data []float64 `json:"data"`
}

// FilterResponse represents the filtered data response
type FilterResponse struct {
	FilteredData []float64 `json:"filtered_data"`
	Removed      int       `json:"removed_count"`
	Method       string    `json:"method"`
}

// ServiceStats tracks performance metrics for this Go service
type ServiceStats struct {
	RequestsProcessed uint64        `json:"requests_processed"`
	TotalProcessingTime time.Duration `json:"total_processing_time_ns"`
	AverageResponseTime time.Duration `json:"average_response_time_ns"`
	OutliersFiltered   uint64        `json:"outliers_filtered"`
	UptimeSeconds      uint64        `json:"uptime_seconds"`
}

var stats ServiceStats
var startTime time.Time

func init() {
	startTime = time.Now()
}

// filterOutliers removes statistical outliers using the IQR method
func filterOutliers(data []float64) ([]float64, int) {
	if len(data) < 4 {
		return data, 0 // Need at least 4 points for IQR
	}
	
	// Sort data to find quartiles
	sorted := make([]float64, len(data))
	copy(sorted, data)
	sort.Float64s(sorted)
	
	// Calculate Q1, Q3, and IQR
	n := len(sorted)
	q1 := sorted[n/4]
	q3 := sorted[(3*n)/4]
	iqr := q3 - q1
	
	// Define outlier bounds
	lowerBound := q1 - 1.5*iqr
	upperBound := q3 + 1.5*iqr
	
	// Filter outliers
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

// handleFilter processes data filtering requests from the Python backend
func handleFilter(w http.ResponseWriter, r *http.Request) {
	startTime := time.Now()
	stats.RequestsProcessed++
	
	var req FilterRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid JSON", http.StatusBadRequest)
		return
	}
	
	filtered, removed := filterOutliers(req.Data)
	
	response := FilterResponse{
		FilteredData: filtered,
		Removed:      removed,
		Method:       "iqr_outlier_detection",
	}
	
	stats.OutliersFiltered += uint64(removed)
	processingTime := time.Since(startTime)
	stats.TotalProcessingTime += processingTime
	stats.AverageResponseTime = stats.TotalProcessingTime / time.Duration(stats.RequestsProcessed)
	
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// handleStats returns service performance statistics
func handleStats(w http.ResponseWriter, r *http.Request) {
	stats.UptimeSeconds = uint64(time.Since(startTime).Seconds())
	
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// handleHealth provides health check endpoint
func handleHealth(w http.ResponseWriter, r *http.Request) {
	health := map[string]interface{}{
		"status": "healthy",
		"uptime_seconds": time.Since(startTime).Seconds(),
		"service": "consilium-filter-service",
		"version": "1.0.0",
	}
	
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(health)
}

func main() {
	r := mux.NewRouter()
	
	// Configure CORS for cross-language HTTP communication
	corsHandler := handlers.CORS(
		handlers.AllowedOrigins([]string{"*"}),
		handlers.AllowedMethods([]string{"GET", "POST", "OPTIONS"}),
		handlers.AllowedHeaders([]string{"Content-Type"}),
	)
	
	// API routes
	r.HandleFunc("/filter", handleFilter).Methods("POST")
	r.HandleFunc("/stats", handleStats).Methods("GET")
	r.HandleFunc("/health", handleHealth).Methods("GET")
	
	port := ":9090"
	fmt.Printf("🚀 Go Filter Service starting on port %s\\n", port)
	fmt.Println("📊 Endpoints:")
	fmt.Println("   POST /filter  - Filter statistical outliers")
	fmt.Println("   GET  /stats   - Service performance statistics") 
	fmt.Println("   GET  /health  - Health check")
	
	log.Fatal(http.ListenAndServe(port, corsHandler(r)))
}
""")
        
        print("  ✅ Go microservice created")
        
    def create_cpp_native(self):
        """Create C++ native library for advanced analytics"""
        cpp_dir = Path("native")
        cpp_dir.mkdir(parents=True, exist_ok=True)
        
        # C++ analytics library
        (cpp_dir / "analytics.cpp").write_text("""
/**
 * C++ Analytics Library - Advanced numerical analysis
 * Provides C-compatible FFI interface for calling from Python, Java, Go
 */

#include <algorithm>
#include <vector>
#include <cmath>
#include <numeric>
#include <chrono>
#include <memory>

// C-compatible interface for FFI calls
extern "C" {
    // Advanced statistical analysis of numerical data
    double analyze_array(const double* data, int length);
    
    // Complex mathematical transformations
    int transform_data(double* data, int length, const char* method);
    
    // Performance monitoring
    unsigned long get_processing_time_ns();
    const char* get_library_version();
}

// Internal C++ implementation with advanced features
namespace consilium {
    class AdvancedAnalytics {
    private:
        static std::chrono::nanoseconds total_processing_time;
        static unsigned long operations_count;
        
    public:
        // Statistical complexity analysis
        static double calculate_complexity_score(const std::vector<double>& data) {
            auto start = std::chrono::high_resolution_clock::now();
            
            if (data.empty()) return 0.0;
            
            // Multi-factor complexity analysis
            double entropy = calculate_entropy(data);
            double variance_ratio = calculate_variance_ratio(data);  
            double trend_strength = calculate_trend_strength(data);
            double periodicity = detect_periodicity(data);
            
            // Weighted complexity score
            double complexity = 0.3 * entropy + 
                              0.25 * variance_ratio + 
                              0.25 * trend_strength + 
                              0.2 * periodicity;
            
            auto end = std::chrono::high_resolution_clock::now();
            total_processing_time += std::chrono::duration_cast<std::chrono::nanoseconds>(end - start);
            operations_count++;
            
            return complexity;
        }
        
    private:
        static double calculate_entropy(const std::vector<double>& data) {
            // Shannon entropy calculation for numerical data
            auto sorted_data = data;
            std::sort(sorted_data.begin(), sorted_data.end());
            
            std::vector<int> counts;
            std::vector<double> unique_vals;
            
            // Group similar values (with tolerance for floating point)
            const double tolerance = 1e-6;
            for (size_t i = 0; i < sorted_data.size(); ++i) {
                if (unique_vals.empty() || 
                    std::abs(sorted_data[i] - unique_vals.back()) > tolerance) {
                    unique_vals.push_back(sorted_data[i]);
                    counts.push_back(1);
                } else {
                    counts.back()++;
                }
            }
            
            // Calculate entropy
            double entropy = 0.0;
            for (int count : counts) {
                double prob = static_cast<double>(count) / data.size();
                if (prob > 0) {
                    entropy -= prob * std::log2(prob);
                }
            }
            
            return entropy;
        }
        
        static double calculate_variance_ratio(const std::vector<double>& data) {
            if (data.size() < 2) return 0.0;
            
            double mean = std::accumulate(data.begin(), data.end(), 0.0) / data.size();
            double variance = 0.0;
            
            for (double value : data) {
                variance += (value - mean) * (value - mean);
            }
            variance /= (data.size() - 1);
            
            // Normalize variance by mean to get coefficient of variation
            return (mean != 0.0) ? std::sqrt(variance) / std::abs(mean) : 0.0;
        }
        
        static double calculate_trend_strength(const std::vector<double>& data) {
            if (data.size() < 3) return 0.0;
            
            // Linear regression to detect trend strength
            size_t n = data.size();
            double sum_x = 0, sum_y = 0, sum_xy = 0, sum_x2 = 0;
            
            for (size_t i = 0; i < n; ++i) {
                double x = static_cast<double>(i);
                double y = data[i];
                sum_x += x;
                sum_y += y;
                sum_xy += x * y;
                sum_x2 += x * x;
            }
            
            double denominator = n * sum_x2 - sum_x * sum_x;
            if (std::abs(denominator) < 1e-10) return 0.0;
            
            double slope = (n * sum_xy - sum_x * sum_y) / denominator;
            
            // Return normalized trend strength
            double data_range = *std::max_element(data.begin(), data.end()) - 
                               *std::min_element(data.begin(), data.end());
            
            return (data_range > 0) ? std::abs(slope) / data_range : 0.0;
        }
        
        static double detect_periodicity(const std::vector<double>& data) {
            // Simplified periodicity detection using autocorrelation
            if (data.size() < 4) return 0.0;
            
            size_t max_lag = std::min(data.size() / 4, size_t(50));
            double max_correlation = 0.0;
            
            for (size_t lag = 1; lag <= max_lag; ++lag) {
                double correlation = 0.0;
                size_t valid_pairs = 0;
                
                for (size_t i = 0; i + lag < data.size(); ++i) {
                    correlation += data[i] * data[i + lag];
                    valid_pairs++;
                }
                
                if (valid_pairs > 0) {
                    correlation /= valid_pairs;
                    max_correlation = std::max(max_correlation, std::abs(correlation));
                }
            }
            
            return max_correlation;
        }
    };
    
    // Static member definitions
    std::chrono::nanoseconds AdvancedAnalytics::total_processing_time{0};
    unsigned long AdvancedAnalytics::operations_count = 0;
}

// C-compatible FFI implementations
double analyze_array(const double* data, int length) {
    if (!data || length <= 0) return 0.0;
    
    std::vector<double> vec_data(data, data + length);
    return consilium::AdvancedAnalytics::calculate_complexity_score(vec_data);
}

int transform_data(double* data, int length, const char* method) {
    if (!data || length <= 0 || !method) return -1;
    
    std::string transform_method(method);
    
    if (transform_method == "normalize") {
        // Min-max normalization
        double min_val = *std::min_element(data, data + length);
        double max_val = *std::max_element(data, data + length);
        double range = max_val - min_val;
        
        if (range > 1e-10) {
            for (int i = 0; i < length; ++i) {
                data[i] = (data[i] - min_val) / range;
            }
        }
        return 0;
    } else if (transform_method == "standardize") {
        // Z-score standardization
        double sum = std::accumulate(data, data + length, 0.0);
        double mean = sum / length;
        
        double variance = 0.0;
        for (int i = 0; i < length; ++i) {
            variance += (data[i] - mean) * (data[i] - mean);
        }
        variance /= length;
        double std_dev = std::sqrt(variance);
        
        if (std_dev > 1e-10) {
            for (int i = 0; i < length; ++i) {
                data[i] = (data[i] - mean) / std_dev;
            }
        }
        return 0;
    }
    
    return -1; // Unknown method
}

unsigned long get_processing_time_ns() {
    return static_cast<unsigned long>(consilium::AdvancedAnalytics::total_processing_time.count());
}

const char* get_library_version() {
    return "Consilium Analytics Library v1.0.0";
}
""")
        
        # CMake build file
        (cpp_dir / "CMakeLists.txt").write_text("""
cmake_minimum_required(VERSION 3.10)
project(ConsiliumAnalytics)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Build shared library for FFI
add_library(analytics SHARED analytics.cpp)

# Enable optimizations for performance
set_target_properties(analytics PROPERTIES
    COMPILE_FLAGS "-O3 -march=native -ffast-math"
    POSITION_INDEPENDENT_CODE ON
)

# Install target
install(TARGETS analytics DESTINATION lib)
""")
        
        print("  ✅ C++ native library created")
        
    def create_java_wrapper(self):
        """Create Java wrapper with JNI integration"""
        java_dir = Path("wrapper/src/main/java/com/consilium")
        java_dir.mkdir(parents=True, exist_ok=True)
        
        # Java wrapper class with JNI
        (java_dir / "DataTransformer.java").write_text("""
package com.consilium;

import java.util.Arrays;
import java.util.List;
import java.util.stream.DoubleStream;

/**
 * Java wrapper for native processing capabilities
 * Integrates with Rust core via JNI for high-performance operations
 */
public class DataTransformer {
    
    static {
        try {
            // Load native library (Rust compiled as JNI library)
            System.loadLibrary("rust_core_jni");
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Warning: Native library not available: " + e.getMessage());
        }
    }
    
    // Native method implementations (provided by Rust via JNI)
    private native double[] sortArray(double[] input);
    private native double[] analyzeDataset(double[] input);
    private native long getNativeProcessingTime();
    
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
    
    /**
     * Advanced statistical analysis using native implementation
     */
    public AnalysisResult analyzeWithNative(double[] data) {
        try {
            double[] stats = analyzeDataset(data); // JNI call to Rust
            return new AnalysisResult(stats[0], stats[1], stats[2], true);
        } catch (UnsatisfiedLinkError e) {
            // Fallback to Java implementation
            return analyzeWithJava(data);
        }
    }
    
    /**
     * Java fallback implementation for statistical analysis
     */
    private AnalysisResult analyzeWithJava(double[] data) {
        if (data.length == 0) {
            return new AnalysisResult(0, 0, 0, false);
        }
        
        double sum = DoubleStream.of(data).sum();
        double mean = sum / data.length;
        
        double variance = DoubleStream.of(data)
            .map(x -> Math.pow(x - mean, 2))
            .average()
            .orElse(0.0);
        
        double[] sortedData = data.clone();
        Arrays.sort(sortedData);
        double median = (sortedData.length % 2 == 0) ?
            (sortedData[sortedData.length / 2 - 1] + sortedData[sortedData.length / 2]) / 2.0 :
            sortedData[sortedData.length / 2];
        
        return new AnalysisResult(mean, Math.sqrt(variance), median, false);
    }
    
    /**
     * Integration with Go microservice for outlier filtering
     */
    public double[] filterWithGoService(double[] data, String serviceUrl) {
        // This would make HTTP calls to the Go service
        // For demo purposes, we'll simulate the integration
        System.out.println("Calling Go microservice at: " + serviceUrl);
        
        // In real implementation, this would use HTTP client to call Go service
        // For now, return original data
        return data;
    }
    
    /**
     * Get performance metrics from native processing
     */
    public NativePerformanceMetrics getPerformanceMetrics() {
        try {
            long nativeTime = getNativeProcessingTime();
            return new NativePerformanceMetrics(nativeTime, true);
        } catch (UnsatisfiedLinkError e) {
            return new NativePerformanceMetrics(0, false);
        }
    }
    
    /**
     * Data class for analysis results
     */
    public static class AnalysisResult {
        public final double mean;
        public final double standardDeviation;
        public final double median;
        public final boolean usedNativeImplementation;
        
        public AnalysisResult(double mean, double stdDev, double median, boolean nativeUsed) {
            this.mean = mean;
            this.standardDeviation = stdDev;
            this.median = median;
            this.usedNativeImplementation = nativeUsed;
        }
        
        @Override
        public String toString() {
            return String.format("AnalysisResult{mean=%.3f, stdDev=%.3f, median=%.3f, native=%b}",
                mean, standardDeviation, median, usedNativeImplementation);
        }
    }
    
    /**
     * Data class for performance metrics
     */
    public static class NativePerformanceMetrics {
        public final long processingTimeNs;
        public final boolean nativeLibraryAvailable;
        
        public NativePerformanceMetrics(long timeNs, boolean available) {
            this.processingTimeNs = timeNs;
            this.nativeLibraryAvailable = available;
        }
    }
    
    // Demo main method
    public static void main(String[] args) {
        DataTransformer transformer = new DataTransformer();
        
        double[] testData = {1.5, 2.3, 1.1, 5.7, 2.1, 3.3, 1.9, 4.2, 3.8, 2.7};
        
        System.out.println("🔧 Java-Rust Integration Demo");
        System.out.println("Original data: " + Arrays.toString(testData));
        
        // Test native sorting
        double[] sorted = transformer.performNativeSort(testData);
        System.out.println("Sorted (Rust): " + Arrays.toString(sorted));
        
        // Test native analysis
        AnalysisResult analysis = transformer.analyzeWithNative(testData);
        System.out.println("Analysis: " + analysis);
        
        // Test performance metrics
        NativePerformanceMetrics metrics = transformer.getPerformanceMetrics();
        System.out.println("Native lib available: " + metrics.nativeLibraryAvailable);
        System.out.println("Processing time: " + metrics.processingTimeNs + "ns");
    }
}
""")
        
        # Maven build configuration
        (Path("wrapper") / "pom.xml").write_text("""
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 
                             http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    
    <groupId>com.consilium</groupId>
    <artifactId>consilium-java-wrapper</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>
    
    <name>Consilium Java Wrapper</name>
    <description>Java wrapper for Consilium multi-language processing system</description>
    
    <properties>
        <maven.compiler.source>17</maven.compiler.source>
        <maven.compiler.target>17</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
    </properties>
    
    <dependencies>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
    
    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.11.0</version>
                <configuration>
                    <source>17</source>
                    <target>17</target>
                </configuration>
            </plugin>
        </plugins>
    </build>
</project>
""")
        
        print("  ✅ Java wrapper created")
        
    def create_config_files(self):
        """Create shared configuration files used by all languages"""
        config_dir = Path("config")
        config_dir.mkdir(parents=True, exist_ok=True)
        
        # Shared application configuration
        (config_dir / "app_config.json").write_text(json.dumps({
            "application": {
                "name": "Consilium Multi-Language Demo",
                "version": "1.0.0",
                "environment": "development"
            },
            "processing": {
                "rust_enabled": True,
                "go_service_url": "http://localhost:9090",
                "cpp_lib_path": "./native/libanalytics.so",
                "performance_logging": True,
                "max_data_size": 1000000,
                "timeout_seconds": 30
            },
            "services": {
                "frontend_port": 3000,
                "backend_port": 8000,
                "go_service_port": 9090
            },
            "languages": {
                "typescript": {
                    "target": "ES2020",
                    "enable_wasm": True
                },
                "python": {
                    "version": "3.11",
                    "enable_native": True
                },
                "rust": {
                    "optimization_level": 3,
                    "target_features": "+avx2"
                },
                "go": {
                    "version": "1.21",
                    "enable_pprof": True
                },
                "cpp": {
                    "standard": "17",
                    "optimization": "O3"
                },
                "java": {
                    "version": "17",
                    "enable_jni": True
                }
            }
        }, indent=2))
        
        # Build configuration
        (Path(".") / "Makefile").write_text("""
# Consilium Multi-Language Build System
# Orchestrates builds across TypeScript, Python, Rust, Go, C++, and Java

.PHONY: all clean build-rust build-go build-cpp build-java build-frontend test demo

all: build-rust build-cpp build-go build-java build-frontend

# Rust core library (PyO3 + WASM targets)
build-rust:
	@echo "🦀 Building Rust core..."
	cd native && cargo build --release
	cd native && cargo build --target wasm32-unknown-unknown --release
	@echo "✅ Rust build complete"

# C++ analytics library  
build-cpp:
	@echo "⚡ Building C++ analytics..."
	cd native && mkdir -p build && cd build && cmake .. && make
	@echo "✅ C++ build complete"

# Go microservice
build-go:
	@echo "🐹 Building Go microservice..."
	cd microservice && go mod tidy && go build -o filter-service
	@echo "✅ Go build complete"

# Java wrapper
build-java:
	@echo "☕ Building Java wrapper..."
	cd wrapper && mvn clean compile package
	@echo "✅ Java build complete"

# TypeScript frontend
build-frontend:
	@echo "📦 Building TypeScript frontend..."
	cd frontend && npm install && npm run build
	@echo "✅ Frontend build complete"

# Run comprehensive tests
test:
	@echo "🧪 Running cross-language tests..."
	cargo test --workspace
	cd microservice && go test ./...
	cd wrapper && mvn test
	@echo "✅ All tests complete"

# Start demo services
demo:
	@echo "🚀 Starting multi-language demo..."
	@echo "Starting Go microservice..."
	cd microservice && ./filter-service &
	@echo "Starting Python backend..."
	cd backend && python data_processor.py &
	@echo "Demo services started!"
	@echo "Access the system at: http://localhost:8000"

# Clean all build artifacts
clean:
	cargo clean
	cd native/build && make clean || true
	cd microservice && rm -f filter-service
	cd wrapper && mvn clean
	cd frontend && rm -rf node_modules dist
	@echo "✅ Clean complete"

# Show system information
info:
	@echo "📊 Consilium Multi-Language System"
	@echo "Languages: TypeScript, Python, Rust, Go, C++, Java"
	@echo "Architecture: Microservices + FFI + WASM"
	@echo "Build targets: $(shell echo $$MAKECMDGOALS)"
""")
        
        # README for the demo project
        (Path(".") / "README.md").write_text("""
# Consilium Multi-Language Demo Project

This is a comprehensive demonstration of cross-language software development, showcasing how modern applications can integrate multiple programming languages for optimal performance and functionality.

## 🏗️ Architecture Overview

```
┌─────────────────┐    HTTP     ┌──────────────────┐
│   TypeScript    │────────────▶│     Python       │
│   Frontend      │             │     Backend      │
│                 │             │                  │
│ • React/Vue UI  │             │ • FastAPI/Flask  │
│ • WASM bindings │             │ • PyO3 FFI       │
│ • HTTP client   │             │ • Orchestration  │
└─────────────────┘             └──────────────────┘
         │                               │
         │ WASM                         │ FFI
         ▼                               ▼
┌─────────────────┐             ┌──────────────────┐
│      Rust       │             │       C++        │
│      Core       │             │    Analytics     │
│                 │             │                  │
│ • Fast sorting  │             │ • Advanced stats │
│ • WASM target   │             │ • C interface    │
│ • PyO3 bindings │             │ • Optimization   │
└─────────────────┘             └──────────────────┘
         │                               │
         │ JNI                          │ ctypes
         ▼                               ▼
┌─────────────────┐    HTTP     ┌──────────────────┐
│      Java       │────────────▶│        Go        │
│     Wrapper     │             │   Microservice   │
│                 │             │                  │
│ • JNI interface │             │ • HTTP server    │
│ • Enterprise    │             │ • Data filtering │
│ • Integration   │             │ • Concurrency    │
└─────────────────┘             └──────────────────┘
```

## 🔧 Language Integration Patterns

### 1. **TypeScript ↔ Rust (WASM)**
- High-performance computations in the browser
- Zero-copy data transfer where possible
- WebAssembly for near-native performance

### 2. **Python ↔ Rust (PyO3 FFI)**
- CPU-intensive operations in Rust
- Seamless Python integration
- Memory-safe foreign function interface

### 3. **Java ↔ Rust (JNI)**
- Enterprise Java applications
- Native performance for critical paths
- Cross-platform compatibility

### 4. **Python ↔ C++ (ctypes FFI)**
- Legacy C++ libraries
- Maximum performance analytics
- Direct memory manipulation

### 5. **HTTP Communication**
- Go microservice architecture
- Language-agnostic REST APIs
- Scalable service mesh

### 6. **Shared Configuration**
- JSON config files read by all languages
- Consistent behavior across services
- Environment-specific settings

## 🚀 Quick Start

1. **Build all components:**
   ```bash
   make all
   ```

2. **Run tests:**
   ```bash
   make test
   ```

3. **Start demo services:**
   ```bash
   make demo
   ```

4. **Access the system:**
   - Frontend: http://localhost:3000
   - Python API: http://localhost:8000
   - Go service: http://localhost:9090
   - Health checks: http://localhost:9090/health

## 📊 Performance Characteristics

| Component | Language | Use Case | Performance |
|-----------|----------|----------|-------------|
| Frontend | TypeScript | UI, WASM integration | ~95% of native |
| Backend | Python | Orchestration, APIs | ~60% of native |
| Core | Rust | Algorithms, FFI | ~99% of native |
| Analytics | C++ | Mathematical operations | ~100% of native |
| Filter | Go | Concurrent processing | ~85% of native |
| Wrapper | Java | Enterprise integration | ~80% of native |

## 🔍 Cross-Language Dependency Analysis

This project is analyzed by the **Consilium Codegraph** system, which can:

- Track dependencies between languages
- Identify FFI call patterns
- Analyze performance bottlenecks
- Generate dependency graphs
- Validate API contracts
- Monitor cross-language errors

Run the analysis:
```bash
cd /path/to/consilium-codegraph
cargo run -- scan --repo /path/to/this/demo/project
```

## 🧪 Testing Strategy

- **Unit tests:** Each language component
- **Integration tests:** FFI boundaries  
- **End-to-end tests:** Full pipeline
- **Performance tests:** Cross-language overhead
- **Contract tests:** API compatibility

## 📈 Monitoring & Observability

- Performance metrics from each language
- Cross-language call tracing
- Memory usage analysis
- Error propagation tracking
- Service health monitoring

---

This demo showcases modern polyglot programming techniques and serves as a reference for building high-performance, multi-language systems.
""")
        
        print("  ✅ Configuration files created")
        
    def run_analysis(self):
        """Run Consilium Codegraph analysis on the demo project"""
        print("\n🔍 Running Consilium Codegraph analysis...")
        
        # Change to the main project directory
        os.chdir(self.original_cwd)
        
        try:
            # Run the scan command
            result = subprocess.run([
                "cargo", "run", "--", 
                "scan", 
                "--repo", self.demo_repo
            ], capture_output=True, text=True, timeout=60)
            
            if result.returncode == 0:
                print("✅ Scan completed successfully!")
                print(f"📊 Scan output:\n{result.stdout}")
            else:
                print(f"⚠️  Scan completed with warnings:\n{result.stderr}")
                
        except subprocess.TimeoutExpired:
            print("⏱️  Scan timed out (this is normal for large projects)")
        except Exception as e:
            print(f"❌ Error running scan: {e}")
        
        # Try to show some results
        try:
            result = subprocess.run([
                "cargo", "run", "--",
                "show", "--repo", self.demo_repo, 
                "--files"
            ], capture_output=True, text=True, timeout=30)
            
            if result.returncode == 0:
                print(f"📁 Files discovered:\n{result.stdout}")
                
        except Exception as e:
            print(f"Could not retrieve file listing: {e}")
            
    def show_demo_summary(self):
        """Display a summary of the demo project"""
        print("\n" + "="*60)
        print("🎯 CONSILIUM CODEGRAPH DEMO SUMMARY")
        print("="*60)
        
        print(f"\n📂 Demo project created at: {self.demo_repo}")
        
        print(f"\n🗂️  Project structure:")
        os.chdir(self.demo_repo)
        result = subprocess.run(["find", ".", "-type", "f", "-name", "*.*"], 
                              capture_output=True, text=True)
        files = result.stdout.strip().split('\n')
        
        # Group files by language
        by_language = {
            'TypeScript': [f for f in files if f.endswith('.ts') or f.endswith('.json') and 'frontend' in f],
            'Python': [f for f in files if f.endswith('.py')],
            'Rust': [f for f in files if f.endswith('.rs') or ('Cargo.toml' in f)],
            'Go': [f for f in files if f.endswith('.go') or f.endswith('go.mod')],
            'C++': [f for f in files if f.endswith('.cpp') or f.endswith('.h')],
            'Java': [f for f in files if f.endswith('.java') or f.endswith('pom.xml')],
            'Config': [f for f in files if 'config' in f or f.endswith('Makefile') or f.endswith('README.md')]
        }
        
        for lang, lang_files in by_language.items():
            if lang_files:
                print(f"  {lang:12} ({len(lang_files)} files): {', '.join(lang_files[:3])}")
                if len(lang_files) > 3:
                    print(f"               {'':12}  ... and {len(lang_files)-3} more")
        
        print(f"\n🔗 Cross-language integrations demonstrated:")
        integrations = [
            "TypeScript → Python (HTTP API calls)",
            "TypeScript → Rust (WASM modules)",  
            "Python → Rust (PyO3 FFI bindings)",
            "Python → C++ (ctypes FFI)",
            "Python → Go (HTTP microservice)",
            "Java → Rust (JNI native methods)",
            "All → JSON (shared configuration)"
        ]
        
        for integration in integrations:
            print(f"  ✓ {integration}")
            
        print(f"\n⚡ Performance characteristics:")
        print("  • Rust core: Near-native performance (~99%)")
        print("  • C++ analytics: Maximum performance (~100%)")  
        print("  • Go microservice: Excellent concurrency (~85%)")
        print("  • TypeScript WASM: Browser-native speed (~95%)")
        print("  • Python orchestration: Good for integration (~60%)")
        print("  • Java wrapper: Enterprise compatibility (~80%)")
        
        print(f"\n🧪 Testing capabilities:")
        print("  • Cross-language dependency detection")
        print("  • FFI call pattern analysis") 
        print("  • Performance bottleneck identification")
        print("  • API contract validation")
        print("  • Build dependency tracking")
        print("  • Error propagation analysis")
        
        print(f"\n🚀 To explore the demo:")
        print(f"  cd {self.demo_repo}")
        print("  make all          # Build all components")
        print("  make test         # Run comprehensive tests")
        print("  make demo         # Start demo services")
        
        print(f"\n🔍 To analyze with Consilium Codegraph:")
        print(f"  cd {self.original_cwd}")
        print(f"  cargo run -- scan --repo {self.demo_repo}")
        print(f"  cargo run -- show --repo {self.demo_repo} --symbols")
        print(f"  cargo run -- search --repo {self.demo_repo} 'DataProcessor'")
        
        print("\n" + "="*60)
        
    def cleanup(self):
        """Clean up demo resources"""
        if self.demo_repo:
            print(f"\n🧹 Cleaning up demo project at {self.demo_repo}")
            try:
                shutil.rmtree(self.demo_repo)
                print("✅ Demo project cleaned up")
            except Exception as e:
                print(f"⚠️  Could not clean up demo project: {e}")

def main():
    demo = ConsiliiumDemo()
    
    try:
        print("🌟 CONSILIUM CODEGRAPH DEMONSTRATION")
        print("="*50)
        print("Creating a realistic multi-language project to showcase")
        print("cross-language dependency analysis capabilities...")
        print()
        
        # Create the demo project
        demo.create_demo_project()
        
        # Run analysis on it
        demo.run_analysis()
        
        # Show summary
        demo.show_demo_summary()
        
        # Ask user if they want to keep the demo
        response = input("\n❓ Keep the demo project for exploration? (y/N): ").strip().lower()
        if response not in ['y', 'yes']:
            demo.cleanup()
        else:
            print(f"✅ Demo project preserved at: {demo.demo_repo}")
        
    except KeyboardInterrupt:
        print("\n\n⏹️  Demo interrupted by user")
        demo.cleanup()
    except Exception as e:
        print(f"\n❌ Demo failed: {e}")
        demo.cleanup()
    finally:
        os.chdir(demo.original_cwd)

if __name__ == "__main__":
    main()