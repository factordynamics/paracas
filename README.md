# paracas

[![CI](https://github.com/factordynamics/paracas/actions/workflows/ci.yml/badge.svg)](https://github.com/factordynamics/paracas/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/paracas.svg)](https://crates.io/crates/paracas)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

High-performance Rust CLI for downloading historical tick data from Dukascopy.

## Features

- **Fast**: Concurrent downloads with connection pooling
- **Flexible**: CSV, JSON, and Parquet output formats
- **Complete**: All 1000+ Dukascopy instruments supported
- **Aggregation**: Built-in OHLCV aggregation (1m, 5m, 15m, 30m, 1h, 4h, 1d)
- **Background Jobs**: Run long downloads as detached daemon processes

## Installation

```bash
cargo install paracas
```

## Usage

### Download Tick Data

```bash
# Download all available EUR/USD tick data (outputs to eurusd.csv by default)
paracas download eurusd

# Download with specific date range
paracas download eurusd -s 2024-01-01 -e 2024-01-31

# Download BTC/USD as Parquet
paracas download btcusd -f parquet

# Download with 1-hour OHLCV aggregation
paracas download eurusd -s 2024-01-01 -e 2024-01-31 -t h1

# Specify custom output file
paracas download eurusd -o my_data.csv

# Run download in background
paracas download eurusd -s 2024-01-01 -e 2024-12-31 --background
```

### Download All Instruments

```bash
# Download all forex instruments
paracas download-all --category forex -o ./data/

# Download all crypto as Parquet in background
paracas download-all --category crypto -f parquet --background
```

### List Instruments

```bash
# List all instruments
paracas list

# Filter by category
paracas list --category forex

# Search instruments
paracas list --search btc
```

### Instrument Info

```bash
paracas info eurusd
```

### Background Jobs

```bash
# Check job status
paracas status --all

# Watch running jobs
paracas status --follow 5

# Manage jobs
paracas job pause <job-id>
paracas job resume <job-id>
paracas job kill <job-id>
paracas job clean
```

## Output Formats

| Format | Extension | Description |
|--------|-----------|-------------|
| CSV | `.csv` | Comma-separated values |
| JSON | `.json` | JSON array |
| Parquet | `.parquet` | Apache Parquet columnar format |

## Timeframes

| Timeframe | Flag | Description |
|-----------|------|-------------|
| Tick | (default) | Raw tick data |
| 1 minute | `-t m1` | 1-minute OHLCV bars |
| 5 minutes | `-t m5` | 5-minute OHLCV bars |
| 15 minutes | `-t m15` | 15-minute OHLCV bars |
| 30 minutes | `-t m30` | 30-minute OHLCV bars |
| 1 hour | `-t h1` | 1-hour OHLCV bars |
| 4 hours | `-t h4` | 4-hour OHLCV bars |
| 1 day | `-t d1` | Daily OHLCV bars |

## Performance

Benchmark comparing paracas against [dukascopy-node](https://www.dukascopy-node.app/) for downloading EUR/USD tick data:

| Data Range | paracas | dukascopy-node | Speedup |
|------------|---------|----------------|---------|
| 1 day | 5.24s | 8.26s | **1.6x** |
| 3 days | 18.97s | 24.27s | **1.3x** |

*Benchmarks run on macOS (Apple Silicon). Results may vary based on network conditions.*

### Running Benchmarks

```bash
# Run benchmark and output markdown table
just bench-table

# Run criterion benchmarks
just bench
```

To compare against dukascopy-node, install it first:

```bash
npm install -g dukascopy-node
```

## License

MIT License - see [LICENSE](LICENSE) for details.
