use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::Serialize;
use tracing::{info, debug};

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceMetrics {
    pub total_duration: Duration,
    pub phase_durations: HashMap<String, Duration>,
    pub file_counts: HashMap<String, usize>,
    pub symbol_counts: HashMap<String, usize>,
    pub edge_counts: HashMap<String, usize>,
    pub occurrence_counts: HashMap<String, usize>,
    pub memory_usage: MemoryUsage,
    pub throughput_metrics: ThroughputMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryUsage {
    pub peak_memory_mb: f64,
    pub final_memory_mb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThroughputMetrics {
    pub files_per_second: f64,
    pub symbols_per_second: f64,
    pub lines_of_code: usize,
    pub processing_rate_loc_per_second: f64,
}

#[derive(Debug)]
pub struct MetricsCollector {
    start_time: Instant,
    phase_timers: HashMap<String, Instant>,
    phase_durations: HashMap<String, Duration>,
    file_counts: HashMap<String, usize>,
    symbol_counts: HashMap<String, usize>, 
    edge_counts: HashMap<String, usize>,
    occurrence_counts: HashMap<String, usize>,
    total_lines_of_code: usize,
    peak_memory_mb: f64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            phase_timers: HashMap::new(),
            phase_durations: HashMap::new(),
            file_counts: HashMap::new(),
            symbol_counts: HashMap::new(),
            edge_counts: HashMap::new(),
            occurrence_counts: HashMap::new(),
            total_lines_of_code: 0,
            peak_memory_mb: 0.0,
        }
    }

    pub fn start_phase(&mut self, phase: &str) {
        debug!("Starting phase: {}", phase);
        self.phase_timers.insert(phase.to_string(), Instant::now());
    }

    pub fn end_phase(&mut self, phase: &str) {
        if let Some(start_time) = self.phase_timers.remove(phase) {
            let duration = start_time.elapsed();
            self.phase_durations.insert(phase.to_string(), duration);
            debug!("Phase {} completed in {:?}", phase, duration);
        }
    }

    pub fn record_file_count(&mut self, language: &str, count: usize) {
        self.file_counts.insert(language.to_string(), count);
        debug!("Processed {} {} files", count, language);
    }

    pub fn record_symbol_count(&mut self, language: &str, count: usize) {
        self.symbol_counts.insert(language.to_string(), count);
        debug!("Found {} {} symbols", count, language);
    }

    pub fn record_edge_count(&mut self, language: &str, count: usize) {
        self.edge_counts.insert(language.to_string(), count);
        debug!("Found {} {} edges", count, language);
    }

    pub fn record_occurrence_count(&mut self, language: &str, count: usize) {
        self.occurrence_counts.insert(language.to_string(), count);
        debug!("Found {} {} occurrences", count, language);
    }

    pub fn record_lines_of_code(&mut self, lines: usize) {
        self.total_lines_of_code += lines;
    }

    pub fn update_memory_usage(&mut self) {
        if let Some(usage) = get_memory_usage() {
            if usage > self.peak_memory_mb {
                self.peak_memory_mb = usage;
            }
        }
    }

    pub fn finalize(self) -> PerformanceMetrics {
        let total_duration = self.start_time.elapsed();
        let final_memory = get_memory_usage().unwrap_or(0.0);

        // Calculate throughput metrics
        let total_files: usize = self.file_counts.values().sum();
        let total_symbols: usize = self.symbol_counts.values().sum();
        
        let total_seconds = total_duration.as_secs_f64();
        let files_per_second = if total_seconds > 0.0 { total_files as f64 / total_seconds } else { 0.0 };
        let symbols_per_second = if total_seconds > 0.0 { total_symbols as f64 / total_seconds } else { 0.0 };
        let processing_rate_loc_per_second = if total_seconds > 0.0 { self.total_lines_of_code as f64 / total_seconds } else { 0.0 };

        let throughput_metrics = ThroughputMetrics {
            files_per_second,
            symbols_per_second,
            lines_of_code: self.total_lines_of_code,
            processing_rate_loc_per_second,
        };

        let memory_usage = MemoryUsage {
            peak_memory_mb: self.peak_memory_mb,
            final_memory_mb: final_memory,
        };

        let metrics = PerformanceMetrics {
            total_duration,
            phase_durations: self.phase_durations,
            file_counts: self.file_counts,
            symbol_counts: self.symbol_counts,
            edge_counts: self.edge_counts,
            occurrence_counts: self.occurrence_counts,
            memory_usage,
            throughput_metrics,
        };

        Self::log_metrics(&metrics);
        metrics
    }

    fn log_metrics(metrics: &PerformanceMetrics) {
        info!("📊 Performance Summary:");
        info!("  Total duration: {:?}", metrics.total_duration);
        
        info!("  Phase timings:");
        for (phase, duration) in &metrics.phase_durations {
            info!("    {}: {:?}", phase, duration);
        }

        let total_files: usize = metrics.file_counts.values().sum();
        let total_symbols: usize = metrics.symbol_counts.values().sum();
        let total_edges: usize = metrics.edge_counts.values().sum();
        let total_occurrences: usize = metrics.occurrence_counts.values().sum();

        info!("  Processed data:");
        info!("    Files: {}", total_files);
        info!("    Symbols: {}", total_symbols);
        info!("    Edges: {}", total_edges);
        info!("    Occurrences: {}", total_occurrences);
        info!("    Lines of Code: {}", metrics.throughput_metrics.lines_of_code);

        info!("  Throughput:");
        info!("    Files/sec: {:.1}", metrics.throughput_metrics.files_per_second);
        info!("    Symbols/sec: {:.1}", metrics.throughput_metrics.symbols_per_second);
        info!("    LOC/sec: {:.1}", metrics.throughput_metrics.processing_rate_loc_per_second);

        info!("  Memory usage:");
        info!("    Peak: {:.1} MB", metrics.memory_usage.peak_memory_mb);
        info!("    Final: {:.1} MB", metrics.memory_usage.final_memory_mb);

        // Performance assessment
        if metrics.throughput_metrics.files_per_second > 10.0 {
            info!("  ✅ High performance: {} files/sec", metrics.throughput_metrics.files_per_second as u32);
        } else if metrics.throughput_metrics.files_per_second > 1.0 {
            info!("  ⚠️  Moderate performance: {} files/sec", metrics.throughput_metrics.files_per_second as u32);
        } else {
            info!("  🐌 Low performance: {:.1} files/sec", metrics.throughput_metrics.files_per_second);
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
fn get_memory_usage() -> Option<f64> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(["-o", "rss=", "-p"])
        .arg(std::process::id().to_string())
        .output()
        .ok()?;

    let memory_kb: f64 = String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse()
        .ok()?;

    Some(memory_kb / 1024.0) // Convert KB to MB
}

#[cfg(target_os = "linux")]
fn get_memory_usage() -> Option<f64> {
    use std::fs;

    let status = fs::read_to_string("/proc/self/status").ok()?;
    
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let memory_kb: f64 = line.split_whitespace()
                .nth(1)?
                .parse()
                .ok()?;
            return Some(memory_kb / 1024.0); // Convert KB to MB
        }
    }
    
    None
}

#[cfg(target_os = "windows")]
fn get_memory_usage() -> Option<f64> {
    // Windows memory usage detection would need additional dependencies
    // For now, return None on Windows
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn get_memory_usage() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();
        
        collector.start_phase("test_phase");
        std::thread::sleep(Duration::from_millis(10));
        collector.end_phase("test_phase");
        
        collector.record_file_count("rust", 5);
        collector.record_symbol_count("rust", 50);
        collector.record_edge_count("rust", 25);
        collector.record_occurrence_count("rust", 100);
        collector.record_lines_of_code(500);
        
        let metrics = collector.finalize();
        
        assert!(metrics.total_duration.as_millis() >= 10);
        assert_eq!(metrics.file_counts["rust"], 5);
        assert_eq!(metrics.symbol_counts["rust"], 50);
        assert_eq!(metrics.edge_counts["rust"], 25);
        assert_eq!(metrics.occurrence_counts["rust"], 100);
        assert_eq!(metrics.throughput_metrics.lines_of_code, 500);
        assert!(metrics.throughput_metrics.files_per_second > 0.0);
    }
    
    #[test]
    fn test_memory_usage() {
        // Memory usage function should not panic
        let _usage = get_memory_usage();
    }
}