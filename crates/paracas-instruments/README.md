# paracas-instruments

Instrument registry for the paracas tick data downloader.

## Features

- Registry of 1000+ Dukascopy instruments
- Lookup by ID (case-insensitive)
- Filter by category (forex, crypto, stocks, etc.)
- Search by name pattern

## Usage

```rust
use paracas_instruments::InstrumentRegistry;

let registry = InstrumentRegistry::global();

// Lookup by ID
if let Some(instrument) = registry.get("eurusd") {
    println!("{}: decimal_factor = {}", instrument.name(), instrument.decimal_factor());
}

// Filter by category
for instrument in registry.forex() {
    println!("{}", instrument.id());
}

// Search
for instrument in registry.search("btc") {
    println!("{}", instrument.name());
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
