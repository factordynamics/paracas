//! Benchmark runner that outputs a markdown table for the README.
//!
//! Run with: `cargo run --package paracas-bench --bin benchmark_table --release`
//!
//! Prerequisites:
//! - Build paracas: `cargo build --release`
//! - Install dukascopy-node: `npm install -g dukascopy-node` (optional)

use paracas_bench::{
    BenchmarkConfig, BenchmarkResult, check_dukascopy_node, find_paracas_binary, format_bytes,
    format_duration, run_dukascopy_node, run_paracas,
};
use std::io::Write;

/// Number of iterations per benchmark for statistical significance.
const ITERATIONS: usize = 3;

fn main() {
    println!("Paracas vs dukascopy-node Benchmark");
    println!("====================================\n");

    let paracas_bin = match find_paracas_binary() {
        Some(bin) => {
            println!("Found paracas binary: {}", bin);
            bin
        }
        None => {
            eprintln!("Error: paracas binary not found.");
            eprintln!("Run `cargo build --release` first.");
            std::process::exit(1);
        }
    };

    let has_dukascopy = check_dukascopy_node();
    if has_dukascopy {
        println!("Found dukascopy-node");
    } else {
        println!("Warning: dukascopy-node not found");
        println!("Install with: npm install -g dukascopy-node");
        println!("Benchmarking paracas only.\n");
    }

    println!("\nRunning benchmarks ({} iterations each)...\n", ITERATIONS);

    let configs = vec![
        (
            "1 day",
            BenchmarkConfig {
                instrument: "eurusd".to_string(),
                start_date: "2024-01-02".to_string(),
                end_date: "2024-01-02".to_string(),
                format: "csv".to_string(),
            },
        ),
        (
            "3 days",
            BenchmarkConfig {
                instrument: "eurusd".to_string(),
                start_date: "2024-01-02".to_string(),
                end_date: "2024-01-04".to_string(),
                format: "csv".to_string(),
            },
        ),
    ];

    let mut results: Vec<(String, Vec<BenchmarkResult>, Vec<BenchmarkResult>)> = Vec::new();

    for (name, config) in &configs {
        print!("Benchmarking {} data... ", name);
        std::io::stdout().flush().unwrap();

        // Run paracas multiple times
        let mut paracas_results = Vec::new();
        for i in 0..ITERATIONS {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let output = temp_dir.path().join("output.csv");
            let result = run_paracas(config, output.to_str().unwrap(), &paracas_bin);
            paracas_results.push(result);
            print!("P{} ", i + 1);
            std::io::stdout().flush().unwrap();
        }

        // Run dukascopy-node multiple times (if available)
        let mut dukascopy_results = Vec::new();
        if has_dukascopy {
            for i in 0..ITERATIONS {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let result = run_dukascopy_node(config, temp_dir.path().to_str().unwrap(), "npx");
                dukascopy_results.push(result);
                print!("D{} ", i + 1);
                std::io::stdout().flush().unwrap();
            }
        }

        results.push((name.to_string(), paracas_results, dukascopy_results));
        println!("done");
    }

    println!("\n## Results\n");

    // Print markdown table
    if has_dukascopy {
        println!("| Data Range | paracas | dukascopy-node | Speedup |");
        println!("|------------|---------|----------------|---------|");
    } else {
        println!("| Data Range | paracas | Data Points | Throughput |");
        println!("|------------|---------|-------------|------------|");
    }

    for (name, paracas_results, dukascopy_results) in &results {
        let paracas_avg = average_results(paracas_results);

        if has_dukascopy && !dukascopy_results.is_empty() {
            let dukascopy_avg = average_results(dukascopy_results);
            let speedup = if paracas_avg.duration.as_secs_f64() > 0.0 {
                dukascopy_avg.duration.as_secs_f64() / paracas_avg.duration.as_secs_f64()
            } else {
                0.0
            };

            println!(
                "| {} | {} | {} | **{:.1}x** |",
                name,
                format_duration(paracas_avg.duration),
                format_duration(dukascopy_avg.duration),
                speedup
            );
        } else {
            let data_points = paracas_avg
                .data_points
                .map(|dp| format!("{:.0}k", dp as f64 / 1000.0))
                .unwrap_or_else(|| "N/A".to_string());
            let throughput = format!("{:.1} MB/s", paracas_avg.throughput_mbps());

            println!(
                "| {} | {} | {} | {} |",
                name,
                format_duration(paracas_avg.duration),
                data_points,
                throughput
            );
        }
    }

    println!("\n### Details\n");

    for (name, paracas_results, dukascopy_results) in &results {
        println!("**{}:**", name);

        // Paracas details
        if let Some(result) = paracas_results.first() {
            if result.success {
                println!(
                    "- paracas: {} ({} ticks, {})",
                    format_duration(average_results(paracas_results).duration),
                    result.data_points.unwrap_or(0),
                    format_bytes(result.output_size)
                );
            } else {
                println!("- paracas: FAILED - {:?}", result.error);
            }
        }

        // dukascopy-node details
        if has_dukascopy && let Some(result) = dukascopy_results.first() {
            if result.success {
                println!(
                    "- dukascopy-node: {} ({} ticks, {})",
                    format_duration(average_results(dukascopy_results).duration),
                    result.data_points.unwrap_or(0),
                    format_bytes(result.output_size)
                );
            } else {
                println!("- dukascopy-node: FAILED - {:?}", result.error);
            }
        }
        println!();
    }

    // Print environment info
    println!("### Environment\n");
    println!("- OS: {}", std::env::consts::OS);
    println!("- Arch: {}", std::env::consts::ARCH);
    println!(
        "- paracas version: {}",
        get_paracas_version(&paracas_bin).unwrap_or_else(|| "unknown".to_string())
    );
    if has_dukascopy {
        println!(
            "- dukascopy-node version: {}",
            get_dukascopy_version().unwrap_or_else(|| "unknown".to_string())
        );
    }
}

fn average_results(results: &[BenchmarkResult]) -> BenchmarkResult {
    let successful: Vec<_> = results.iter().filter(|r| r.success).collect();

    if successful.is_empty() {
        return results.first().cloned().unwrap_or(BenchmarkResult {
            tool: "unknown".to_string(),
            duration: std::time::Duration::ZERO,
            output_size: 0,
            data_points: None,
            success: false,
            error: Some("No successful runs".to_string()),
        });
    }

    let avg_duration = std::time::Duration::from_secs_f64(
        successful
            .iter()
            .map(|r| r.duration.as_secs_f64())
            .sum::<f64>()
            / successful.len() as f64,
    );

    let avg_size = successful.iter().map(|r| r.output_size).sum::<u64>() / successful.len() as u64;

    let avg_points = if successful.iter().all(|r| r.data_points.is_some()) {
        Some(successful.iter().filter_map(|r| r.data_points).sum::<u64>() / successful.len() as u64)
    } else {
        None
    };

    BenchmarkResult {
        tool: successful
            .first()
            .map(|r| r.tool.clone())
            .unwrap_or_default(),
        duration: avg_duration,
        output_size: avg_size,
        data_points: avg_points,
        success: true,
        error: None,
    }
}

fn get_paracas_version(bin: &str) -> Option<String> {
    std::process::Command::new(bin)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        })
}

fn get_dukascopy_version() -> Option<String> {
    // dukascopy-node doesn't have a simple --version flag, check npm package version
    std::process::Command::new("npm")
        .args(["list", "-g", "dukascopy-node", "--depth=0"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout).ok().and_then(|s| {
                s.lines()
                    .find(|line| line.contains("dukascopy-node"))
                    .map(|line| line.trim().to_string())
            })
        })
}
