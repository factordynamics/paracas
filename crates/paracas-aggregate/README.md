# paracas-aggregate

OHLCV aggregation for the paracas tick data downloader.

## Features

- Tick-to-OHLCV aggregation
- Multiple timeframes (1s, 1m, 5m, 15m, 30m, 1h, 4h, 1d)
- Streaming aggregation for memory efficiency

## Usage

```rust,ignore
use paracas_aggregate::{Ohlcv, TickAggregator};
use paracas_types::Timeframe;

// Create an aggregator for 1-hour bars
let mut aggregator = TickAggregator::new(Timeframe::Hour1);

// Process ticks
for tick in ticks {
    if let Some(bar) = aggregator.process(tick) {
        println!("Completed bar: {:?}", bar);
    }
}

// Get any remaining partial bar
if let Some(bar) = aggregator.finish() {
    println!("Final bar: {:?}", bar);
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
