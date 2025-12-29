//! Benchmark utilities for paracas.

use std::process::Command;
use std::time::{Duration, Instant};

/// Result of a single benchmark run.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the tool being benchmarked.
    pub tool: String,
    /// Duration of the download.
    pub duration: Duration,
    /// Size of the output file in bytes.
    pub output_size: u64,
    /// Number of data points (ticks/rows).
    pub data_points: Option<u64>,
    /// Whether the run was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl BenchmarkResult {
    /// Calculate throughput in MB/s.
    pub fn throughput_mbps(&self) -> f64 {
        let bytes = self.output_size as f64;
        let secs = self.duration.as_secs_f64();
        if secs > 0.0 {
            (bytes / 1_000_000.0) / secs
        } else {
            0.0
        }
    }

    /// Calculate data points per second.
    pub fn data_points_per_sec(&self) -> Option<f64> {
        self.data_points.map(|dp| {
            let secs = self.duration.as_secs_f64();
            if secs > 0.0 { dp as f64 / secs } else { 0.0 }
        })
    }
}

/// Configuration for a benchmark run.
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Instrument to download (e.g., "eurusd").
    pub instrument: String,
    /// Start date in YYYY-MM-DD format.
    pub start_date: String,
    /// End date in YYYY-MM-DD format.
    pub end_date: String,
    /// Output format (csv, json).
    pub format: String,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            instrument: "eurusd".to_string(),
            // Default to 1 day of data for quick benchmarks
            start_date: "2024-01-02".to_string(),
            end_date: "2024-01-02".to_string(),
            format: "csv".to_string(),
        }
    }
}

/// Run paracas benchmark.
pub fn run_paracas(
    config: &BenchmarkConfig,
    output_path: &str,
    paracas_bin: &str,
) -> BenchmarkResult {
    let start = Instant::now();

    let result = Command::new(paracas_bin)
        .args([
            "download",
            &config.instrument,
            "-s",
            &config.start_date,
            "-e",
            &config.end_date,
            "-o",
            output_path,
            "-f",
            &config.format,
            "-q", // quiet mode
        ])
        .output();

    let duration = start.elapsed();

    match result {
        Ok(output) => {
            if output.status.success() {
                let output_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
                let data_points = count_csv_rows(output_path);

                BenchmarkResult {
                    tool: "paracas".to_string(),
                    duration,
                    output_size,
                    data_points,
                    success: true,
                    error: None,
                }
            } else {
                BenchmarkResult {
                    tool: "paracas".to_string(),
                    duration,
                    output_size: 0,
                    data_points: None,
                    success: false,
                    error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                }
            }
        }
        Err(e) => BenchmarkResult {
            tool: "paracas".to_string(),
            duration,
            output_size: 0,
            data_points: None,
            success: false,
            error: Some(e.to_string()),
        },
    }
}

/// Run dukascopy-node benchmark.
pub fn run_dukascopy_node(
    config: &BenchmarkConfig,
    output_path: &str,
    npx_bin: &str,
) -> BenchmarkResult {
    let start = Instant::now();

    // dukascopy-node uses lowercase instrument names
    let instrument = config.instrument.to_lowercase();

    // dukascopy-node's -to date is EXCLUSIVE, so we need to add 1 day
    // to get the same data range as paracas
    let end_date_exclusive = increment_date(&config.end_date);

    // dukascopy-node CLI: npx dukascopy-node -i eurusd -from 2024-01-02 -to 2024-01-03 -t tick -f csv
    let result = Command::new(npx_bin)
        .args([
            "dukascopy-node",
            "-i",
            &instrument,
            "-from",
            &config.start_date,
            "-to",
            &end_date_exclusive,
            "-t",
            "tick",
            "-f",
            &config.format,
            "-dir",
            output_path,
            "-s", // silent mode
        ])
        .output();

    let duration = start.elapsed();

    match result {
        Ok(output) => {
            if output.status.success() {
                // dukascopy-node outputs to a directory, find the file
                let output_file = find_output_file(output_path, &config.format);
                let (output_size, data_points) = match &output_file {
                    Some(path) => {
                        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                        let points = count_csv_rows(path);
                        (size, points)
                    }
                    None => (0, None),
                };

                BenchmarkResult {
                    tool: "dukascopy-node".to_string(),
                    duration,
                    output_size,
                    data_points,
                    success: true,
                    error: None,
                }
            } else {
                BenchmarkResult {
                    tool: "dukascopy-node".to_string(),
                    duration,
                    output_size: 0,
                    data_points: None,
                    success: false,
                    error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                }
            }
        }
        Err(e) => BenchmarkResult {
            tool: "dukascopy-node".to_string(),
            duration,
            output_size: 0,
            data_points: None,
            success: false,
            error: Some(e.to_string()),
        },
    }
}

/// Count rows in a CSV file (excluding header).
fn count_csv_rows(path: &str) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()
        .map(|content| content.lines().count().saturating_sub(1) as u64)
}

/// Find output file in directory.
fn find_output_file(dir: &str, format: &str) -> Option<String> {
    let ext = match format {
        "csv" => "csv",
        "json" => "json",
        _ => "csv",
    };

    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e == ext)
                .unwrap_or(false)
        })
        .map(|e| e.path().to_string_lossy().to_string())
}

/// Increment a date string by one day (for dukascopy-node exclusive end date).
fn increment_date(date_str: &str) -> String {
    use chrono::NaiveDate;
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").expect("valid date");
    let next_day = date + chrono::Duration::days(1);
    next_day.format("%Y-%m-%d").to_string()
}

/// Format duration for display.
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.2}s", secs)
    } else {
        let mins = secs / 60.0;
        format!("{:.1}m", mins)
    }
}

/// Format bytes for display.
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Check if dukascopy-node is available.
pub fn check_dukascopy_node() -> bool {
    // dukascopy-node --version requires an instrument, so just check --help
    Command::new("npx")
        .args(["dukascopy-node", "--help"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if paracas binary exists.
pub fn find_paracas_binary() -> Option<String> {
    // Try release binary first
    let release_path = "target/release/paracas";
    if std::path::Path::new(release_path).exists() {
        return Some(release_path.to_string());
    }

    // Try debug binary
    let debug_path = "target/debug/paracas";
    if std::path::Path::new(debug_path).exists() {
        return Some(debug_path.to_string());
    }

    // Try system PATH
    which::which("paracas")
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}
