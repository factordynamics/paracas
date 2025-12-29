//! Download benchmarks comparing paracas vs dukascopy-node.
//!
//! Run with: `cargo bench --package paracas-bench`
//!
//! Prerequisites:
//! - Build paracas: `cargo build --release`
//! - Install dukascopy-node: `npm install -g dukascopy-node` (optional)

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use paracas_bench::{
    BenchmarkConfig, check_dukascopy_node, find_paracas_binary, run_dukascopy_node, run_paracas,
};
use std::time::Duration;
use tempfile::TempDir;

/// Benchmark configurations for different data sizes.
fn benchmark_configs() -> Vec<(&'static str, BenchmarkConfig)> {
    vec![
        (
            "1-day",
            BenchmarkConfig {
                instrument: "eurusd".to_string(),
                start_date: "2024-01-02".to_string(),
                end_date: "2024-01-02".to_string(),
                format: "csv".to_string(),
            },
        ),
        (
            "3-days",
            BenchmarkConfig {
                instrument: "eurusd".to_string(),
                start_date: "2024-01-02".to_string(),
                end_date: "2024-01-04".to_string(),
                format: "csv".to_string(),
            },
        ),
    ]
}

fn download_benchmark(c: &mut Criterion) {
    let paracas_bin = find_paracas_binary()
        .expect("paracas binary not found. Run `cargo build --release` first.");
    let has_dukascopy = check_dukascopy_node();

    if !has_dukascopy {
        eprintln!(
            "Warning: dukascopy-node not found. Install with `npm install -g dukascopy-node`"
        );
        eprintln!("Benchmarking paracas only.");
    }

    let mut group = c.benchmark_group("download");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    for (name, config) in benchmark_configs() {
        // Estimate throughput based on typical EURUSD tick count (~100k ticks/day)
        let days = estimate_days(&config);
        let estimated_ticks = days * 100_000;
        group.throughput(Throughput::Elements(estimated_ticks));

        // Benchmark paracas
        group.bench_with_input(BenchmarkId::new("paracas", name), &config, |b, config| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let temp_dir = TempDir::new().unwrap();
                    let output = temp_dir.path().join("output.csv");
                    let result = run_paracas(config, output.to_str().unwrap(), &paracas_bin);
                    if result.success {
                        total += result.duration;
                    }
                }
                total
            });
        });

        // Benchmark dukascopy-node (if available)
        if has_dukascopy {
            group.bench_with_input(
                BenchmarkId::new("dukascopy-node", name),
                &config,
                |b, config| {
                    b.iter_custom(|iters| {
                        let mut total = Duration::ZERO;
                        for _ in 0..iters {
                            let temp_dir = TempDir::new().unwrap();
                            let result = run_dukascopy_node(
                                config,
                                temp_dir.path().to_str().unwrap(),
                                "npx",
                            );
                            if result.success {
                                total += result.duration;
                            }
                        }
                        total
                    });
                },
            );
        }
    }

    group.finish();
}

fn estimate_days(config: &BenchmarkConfig) -> u64 {
    use chrono::NaiveDate;
    let start = NaiveDate::parse_from_str(&config.start_date, "%Y-%m-%d").unwrap();
    let end = NaiveDate::parse_from_str(&config.end_date, "%Y-%m-%d").unwrap();
    (end - start).num_days().max(1) as u64
}

criterion_group!(benches, download_benchmark);
criterion_main!(benches);
