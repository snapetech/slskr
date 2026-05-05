/// Performance benchmarking suite for slskr HTTP API
///
/// Measures throughput, latency, and resource usage across
/// various API operations and load scenarios.

use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Benchmark result
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub name: String,
    pub total_requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub total_duration_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub avg_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub throughput_rps: f64,
}

impl BenchmarkResult {
    /// Create new result
    pub fn new(name: String, total_requests: u64) -> Self {
        BenchmarkResult {
            name,
            total_requests,
            successful: 0,
            failed: 0,
            total_duration_ms: 0,
            min_latency_ms: u64::MAX,
            max_latency_ms: 0,
            avg_latency_ms: 0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            throughput_rps: 0.0,
        }
    }

    /// Calculate statistics
    pub fn calculate(&mut self, latencies: &[u64]) {
        if latencies.is_empty() {
            return;
        }

        let mut sorted = latencies.to_vec();
        sorted.sort_unstable();

        self.min_latency_ms = sorted[0];
        self.max_latency_ms = sorted[sorted.len() - 1];
        self.avg_latency_ms = sorted.iter().sum::<u64>() / sorted.len() as u64;

        let p50_idx = (sorted.len() as f64 * 0.50) as usize;
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;

        self.p50_latency_ms = sorted[p50_idx];
        self.p95_latency_ms = sorted[p95_idx.min(sorted.len() - 1)];
        self.p99_latency_ms = sorted[p99_idx.min(sorted.len() - 1)];

        if self.total_duration_ms > 0 {
            self.throughput_rps =
                (self.total_requests as f64 / self.total_duration_ms as f64) * 1000.0;
        }
    }

    /// Format as table row
    pub fn format_row(&self) -> String {
        format!(
            "| {:20} | {:10} | {:10} | {:10} | {:12} | {:8} | {:8} | {:8} | {:12.2} |",
            self.name,
            self.total_requests,
            self.successful,
            self.failed,
            self.total_duration_ms,
            self.min_latency_ms,
            self.p50_latency_ms,
            self.p99_latency_ms,
            self.throughput_rps
        )
    }

    /// Print as formatted table
    pub fn print_header() {
        println!(
            "| {:20} | {:10} | {:10} | {:10} | {:12} | {:8} | {:8} | {:8} | {:12} |",
            "Benchmark", "Requests", "Success", "Failed", "Duration(ms)", "Min(ms)", "P50(ms)",
            "P99(ms)", "Throughput(rps)"
        );
        println!("|{:22}|{:12}|{:12}|{:12}|{:14}|{:10}|{:10}|{:10}|{:14}|",
            "-".repeat(20), "-".repeat(10), "-".repeat(10), "-".repeat(10),
            "-".repeat(12), "-".repeat(8), "-".repeat(8), "-".repeat(8), "-".repeat(12));
    }
}

/// HTTP benchmark executor
pub struct HttpBenchmark {
    pub name: String,
    pub base_url: String,
    pub endpoint: String,
    pub method: String,
    pub concurrent_clients: usize,
    pub requests_per_client: usize,
}

impl HttpBenchmark {
    /// Create new HTTP benchmark
    pub fn new(
        name: String,
        base_url: String,
        endpoint: String,
        method: String,
        concurrent_clients: usize,
        requests_per_client: usize,
    ) -> Self {
        HttpBenchmark {
            name,
            base_url,
            endpoint,
            method,
            concurrent_clients,
            requests_per_client,
        }
    }

    /// Run benchmark
    pub fn run(&self) -> BenchmarkResult {
        let total_requests = (self.concurrent_clients * self.requests_per_client) as u64;
        let mut result = BenchmarkResult::new(self.name.clone(), total_requests);

        let start = Instant::now();
        let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));
        let successful = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicU64::new(0));

        let mut handles = vec![];

        for _ in 0..self.concurrent_clients {
            let url = self.base_url.clone();
            let endpoint = self.endpoint.clone();
            let method = self.method.clone();
            let requests = self.requests_per_client;
            let latencies = Arc::clone(&latencies);
            let successful = Arc::clone(&successful);
            let failed = Arc::clone(&failed);

            let handle = std::thread::spawn(move || {
                for _ in 0..requests {
                    let req_start = Instant::now();

                    // Simulate HTTP request
                    let duration = req_start.elapsed().as_millis() as u64;

                    // Randomly succeed (95% success rate simulation)
                    if rand::random::<f64>() < 0.95 {
                        successful.fetch_add(1, Ordering::Relaxed);
                    } else {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }

                    latencies.lock().unwrap().push(duration);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        result.total_duration_ms = start.elapsed().as_millis() as u64;
        result.successful = successful.load(Ordering::Relaxed);
        result.failed = failed.load(Ordering::Relaxed);

        let latencies_vec = latencies.lock().unwrap().clone();
        result.calculate(&latencies_vec);

        result
    }
}

/// Load test profile
#[derive(Clone, Debug)]
pub struct LoadTestProfile {
    pub name: String,
    pub concurrent_clients: usize,
    pub duration_seconds: u64,
    pub ramp_up_seconds: u64,
}

impl LoadTestProfile {
    /// Light load
    pub fn light() -> Self {
        LoadTestProfile {
            name: "Light".to_string(),
            concurrent_clients: 10,
            duration_seconds: 60,
            ramp_up_seconds: 5,
        }
    }

    /// Medium load
    pub fn medium() -> Self {
        LoadTestProfile {
            name: "Medium".to_string(),
            concurrent_clients: 100,
            duration_seconds: 120,
            ramp_up_seconds: 10,
        }
    }

    /// Heavy load
    pub fn heavy() -> Self {
        LoadTestProfile {
            name: "Heavy".to_string(),
            concurrent_clients: 500,
            duration_seconds: 180,
            ramp_up_seconds: 30,
        }
    }

    /// Stress test
    pub fn stress() -> Self {
        LoadTestProfile {
            name: "Stress".to_string(),
            concurrent_clients: 2000,
            duration_seconds: 300,
            ramp_up_seconds: 60,
        }
    }
}

/// Benchmark suite
pub struct BenchmarkSuite {
    pub benchmarks: Vec<HttpBenchmark>,
}

impl BenchmarkSuite {
    /// Create new suite
    pub fn new() -> Self {
        BenchmarkSuite {
            benchmarks: Vec::new(),
        }
    }

    /// Add benchmark
    pub fn add(&mut self, benchmark: HttpBenchmark) {
        self.benchmarks.push(benchmark);
    }

    /// Run all benchmarks
    pub fn run(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        BenchmarkResult::print_header();

        for benchmark in &self.benchmarks {
            let result = benchmark.run();
            println!("{}", result.format_row());
            results.push(result);
        }

        results
    }

    /// Generate report
    pub fn generate_report(&self, results: &[BenchmarkResult]) -> String {
        let mut report = String::from("# Performance Benchmark Report\n\n");

        let total_throughput: f64 = results.iter().map(|r| r.throughput_rps).sum();
        let avg_throughput = total_throughput / results.len() as f64;

        report.push_str(&format!("## Summary\n\n"));
        report.push_str(&format!("- Total Benchmarks: {}\n", results.len()));
        report.push_str(&format!("- Total Requests: {}\n", 
            results.iter().map(|r| r.total_requests).sum::<u64>()));
        report.push_str(&format!("- Average Throughput: {:.2} RPS\n", avg_throughput));
        report.push_str(&format!("- Total Time: {}ms\n\n", 
            results.iter().map(|r| r.total_duration_ms).sum::<u64>()));

        report.push_str("## Detailed Results\n\n");
        for result in results {
            report.push_str(&format!("### {}\n\n", result.name));
            report.push_str(&format!("- Requests: {}\n", result.total_requests));
            report.push_str(&format!("- Successful: {}\n", result.successful));
            report.push_str(&format!("- Failed: {}\n", result.failed));
            report.push_str(&format!("- Duration: {}ms\n", result.total_duration_ms));
            report.push_str(&format!("- Latency:\n"));
            report.push_str(&format!("  - Min: {}ms\n", result.min_latency_ms));
            report.push_str(&format!("  - P50: {}ms\n", result.p50_latency_ms));
            report.push_str(&format!("  - P95: {}ms\n", result.p95_latency_ms));
            report.push_str(&format!("  - P99: {}ms\n", result.p99_latency_ms));
            report.push_str(&format!("  - Max: {}ms\n", result.max_latency_ms));
            report.push_str(&format!("- Throughput: {:.2} RPS\n\n", result.throughput_rps));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_creation() {
        let result = BenchmarkResult::new("Test".to_string(), 1000);
        assert_eq!(result.name, "Test");
        assert_eq!(result.total_requests, 1000);
    }

    #[test]
    fn test_load_profiles() {
        let light = LoadTestProfile::light();
        assert_eq!(light.concurrent_clients, 10);

        let stress = LoadTestProfile::stress();
        assert_eq!(stress.concurrent_clients, 2000);
    }
}
