# paracas-fetch

HTTP client and data fetching for the paracas tick data downloader.

## Features

- Concurrent HTTP downloads with connection pooling
- LZMA decompression for bi5 files
- Binary tick data parsing
- Streaming async API with backpressure

## Architecture

The fetch pipeline consists of:

1. **URL Builder** - Constructs Dukascopy data URLs
2. **HTTP Client** - Downloads bi5 files with retries
3. **Decompressor** - LZMA decompression
4. **Parser** - Binary tick data parsing

## Usage

```rust,ignore
use paracas_fetch::{DownloadClient, ClientConfig, tick_stream};
use paracas_instruments::InstrumentRegistry;
use paracas_types::DateRange;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let client = DownloadClient::new(ClientConfig::default()).unwrap();
    let registry = InstrumentRegistry::global();
    let instrument = registry.get("eurusd").unwrap();

    let range = DateRange::new(
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ).unwrap();

    let mut stream = tick_stream(&client, instrument, range);
    while let Some(result) = stream.next().await {
        match result {
            Ok(tick) => println!("{:?}", tick),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
