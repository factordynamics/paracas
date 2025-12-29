# paracas-lib

High-performance Rust library for downloading historical tick data from Dukascopy.

## Features

- **Fast**: Concurrent downloads with connection pooling
- **Flexible**: CSV, JSON, and Parquet output formats
- **Complete**: All 1000+ Dukascopy instruments supported
- **Aggregation**: Built-in OHLCV aggregation

## Quick Start

```rust,ignore
use paracas_lib::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get instrument
    let registry = InstrumentRegistry::global();
    let instrument = registry.get("eurusd").unwrap();

    // Create client
    let client = DownloadClient::with_defaults()?;

    // Define date range
    let range = DateRange::new(
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    )?;

    // Stream ticks
    let mut stream = tick_stream(&client, instrument, range);
    while let Some(batch) = stream.next().await {
        let batch = batch?;
        println!("Downloaded {} ticks for {:?}", batch.len(), batch.hour);
    }

    Ok(())
}
```

## Crates

This is a facade crate that re-exports functionality from:

- `paracas-types` - Core types (Tick, Instrument, DateRange)
- `paracas-instruments` - Instrument registry
- `paracas-fetch` - HTTP client and data fetching
- `paracas-aggregate` - OHLCV aggregation
- `paracas-format` - Output formatters

Related workspace crates (not re-exported):

- `paracas-daemon` - Background job management
- `paracas-estimate` - Download size estimation

## License

MIT License - see [LICENSE](../../LICENSE) for details.
