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

# Download in background
paracas download eurusd -s 2024-01-01 -e 2024-12-31 --background
```

### Download All

Download all instruments (or filter by category):

```bash
# Download all forex instruments
paracas download-all --category forex -o ./data/

# Download all crypto as Parquet in background
paracas download-all --category crypto -f parquet --background
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

### Status

Check background job status:

```bash
# Show all jobs
paracas status --all

# Show only running jobs
paracas status --running

# Check specific job
paracas status <job-id>

# Watch mode (refresh every 5 seconds)
paracas status --follow 5

# Cancel a running job
paracas status --cancel <job-id>
```

### Job

Manage background jobs:

```bash
# Pause a running job
paracas job pause <job-id>

# Resume a paused job
paracas job resume <job-id>

# Kill a running or paused job
paracas job kill <job-id>

# Clean up finished jobs
paracas job clean

# Clean all finished jobs
paracas job clean --all
```

## License

MIT License - see [LICENSE](../LICENSE) for details.
