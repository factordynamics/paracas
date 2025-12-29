# paracas

High-performance Rust CLI for downloading historical tick data from Dukascopy.

## Features

- **Fast**: Concurrent downloads with connection pooling
- **Flexible**: CSV, JSON, and Parquet output formats
- **Complete**: All 1000+ Dukascopy instruments supported
- **Aggregation**: Built-in OHLCV aggregation (1m, 5m, 15m, 30m, 1h, 4h, 1d)

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

## License

MIT License - see [LICENSE](LICENSE) for details.
