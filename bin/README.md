# paracas

Command-line interface for downloading Dukascopy tick data.

## Installation

```bash
cargo install paracas
```

## Commands

### Download

Download tick data for an instrument:

```bash
# Download EUR/USD ticks as CSV
paracas download eurusd -s 2024-01-01 -e 2024-01-31 -o data.csv

# Download as Parquet with 1-hour aggregation
paracas download btcusd -s 2024-01-01 -e 2024-12-31 -o data.parquet -f parquet -t h1
```

### List

List available instruments:

```bash
# List all instruments
paracas list

# Filter by category
paracas list --category forex

# Search
paracas list --search btc
```

### Info

Show instrument details:

```bash
paracas info eurusd
```

## License

MIT License - see [LICENSE](../LICENSE) for details.
