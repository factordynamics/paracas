//! High-performance Rust library for downloading Dukascopy tick data.
//!
//! This is a facade crate that re-exports functionality from the paracas
//! workspace crates for convenient access.
//!
//! # Quick Start
//!
//! ```ignore
//! use paracas_lib::prelude::*;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let registry = InstrumentRegistry::global();
//!     let instrument = registry.get("eurusd").unwrap();
//!     let client = DownloadClient::with_defaults()?;
//!
//!     let range = DateRange::new(
//!         chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
//!         chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
//!     )?;
//!
//!     let mut stream = tick_stream(&client, instrument, range);
//!     while let Some(batch) = stream.next().await {
//!         println!("Downloaded {} ticks", batch?.len());
//!     }
//!
//!     Ok(())
//! }
//! ```

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

// Re-export core types
pub use paracas_types::*;

// Re-export instrument registry
pub use paracas_instruments::InstrumentRegistry;

// Re-export fetch functionality
#[cfg(feature = "fetch")]
pub use paracas_fetch::{
    ClientConfig, DecompressError, DownloadClient, DownloadError, ParseError, TickBatch,
    tick_stream, tick_stream_resilient,
};

// Re-export aggregation
#[cfg(feature = "aggregate")]
pub use paracas_aggregate::{Ohlcv, TickAggregator};

// Re-export formatters
#[cfg(feature = "format")]
pub use paracas_format::{CsvFormatter, FormatError, Formatter, JsonFormatter, OutputFormat};

#[cfg(all(feature = "format", feature = "parquet"))]
pub use paracas_format::ParquetFormatter;

/// Prelude module for convenient imports.
///
/// ```
/// use paracas_lib::prelude::*;
/// ```
pub mod prelude {
    pub use paracas_types::{
        Category, DateRange, DateRangeError, Instrument, ParacasError, RawTick, Result, Tick,
        Timeframe,
    };

    pub use paracas_instruments::InstrumentRegistry;

    #[cfg(feature = "fetch")]
    pub use paracas_fetch::{
        ClientConfig, DownloadClient, TickBatch, tick_stream, tick_stream_resilient,
    };

    #[cfg(feature = "aggregate")]
    pub use paracas_aggregate::{Ohlcv, TickAggregator};

    #[cfg(feature = "format")]
    pub use paracas_format::{CsvFormatter, Formatter, JsonFormatter, OutputFormat};

    #[cfg(all(feature = "format", feature = "parquet"))]
    pub use paracas_format::ParquetFormatter;
}
