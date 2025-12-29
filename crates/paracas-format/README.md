# paracas-format

Output formatters for the paracas tick data downloader.

## Supported Formats

- **CSV** - Comma-separated values
- **JSON** - JSON array or newline-delimited JSON (NDJSON)
- **Parquet** - Apache Parquet columnar format (requires `parquet` feature)

## Usage

```rust,no_run
use paracas_format::{CsvFormatter, Formatter, OutputFormat};
use paracas_types::Tick;
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ticks: Vec<Tick> = vec![];
    let mut output = Cursor::new(Vec::new());

    let formatter = CsvFormatter::new();
    formatter.write_ticks(&ticks, &mut output)?;
    Ok(())
}
```

## Features

- `csv` - CSV format support (default)
- `json` - JSON format support (default)
- `parquet` - Parquet format support (default)

## License

MIT License - see [LICENSE](../../LICENSE) for details.
