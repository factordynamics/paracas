//! Display utilities and output formatting for the paracas CLI.

use anyhow::{Result, bail};
use clap::ValueEnum;
use paracas_lib::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

/// Output format for downloaded data.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Format {
    Csv,
    Json,
    Ndjson,
    Parquet,
}

impl Format {
    /// Returns the file extension for this format.
    pub(crate) const fn extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
            Self::Parquet => "parquet",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

/// Aggregate ticks into OHLCV bars using the given timeframe.
pub(crate) fn aggregate_ticks(ticks: &[Tick], timeframe: Timeframe) -> Vec<Ohlcv> {
    let mut aggregator = TickAggregator::new(timeframe);
    let mut bars = Vec::new();

    for tick in ticks {
        if let Some(bar) = aggregator.process(*tick) {
            bars.push(bar);
        }
    }

    if let Some(bar) = aggregator.finish() {
        bars.push(bar);
    }

    bars
}

/// Write ticks to a file in the specified format.
pub(crate) fn write_ticks(ticks: &[Tick], output: &PathBuf, format: Format) -> Result<()> {
    let file = File::create(output)?;
    let writer = BufWriter::new(file);

    match format {
        Format::Csv => {
            let formatter = CsvFormatter::new();
            formatter.write_ticks(ticks, writer)?;
        }
        Format::Json => {
            let formatter = JsonFormatter::new();
            formatter.write_ticks(ticks, writer)?;
        }
        Format::Ndjson => {
            let formatter = JsonFormatter::ndjson();
            formatter.write_ticks(ticks, writer)?;
        }
        Format::Parquet => {
            #[cfg(feature = "parquet")]
            {
                let formatter = ParquetFormatter::new();
                formatter.write_ticks(ticks, writer)?;
            }
            #[cfg(not(feature = "parquet"))]
            {
                bail!("Parquet support not compiled in");
            }
        }
    }

    Ok(())
}

/// Write OHLCV bars to a file in the specified format.
pub(crate) fn write_ohlcv(bars: &[Ohlcv], output: &PathBuf, format: Format) -> Result<()> {
    let file = File::create(output)?;
    let writer = BufWriter::new(file);

    match format {
        Format::Csv => {
            let formatter = CsvFormatter::new();
            formatter.write_ohlcv(bars, writer)?;
        }
        Format::Json => {
            let formatter = JsonFormatter::new();
            formatter.write_ohlcv(bars, writer)?;
        }
        Format::Ndjson => {
            let formatter = JsonFormatter::ndjson();
            formatter.write_ohlcv(bars, writer)?;
        }
        Format::Parquet => {
            #[cfg(feature = "parquet")]
            {
                let formatter = ParquetFormatter::new();
                formatter.write_ohlcv(bars, writer)?;
            }
            #[cfg(not(feature = "parquet"))]
            {
                bail!("Parquet support not compiled in");
            }
        }
    }

    Ok(())
}

/// Parse a category string into a Category enum.
pub(crate) fn parse_category(s: &str) -> Result<Category> {
    match s.to_lowercase().as_str() {
        "forex" => Ok(Category::Forex),
        "crypto" => Ok(Category::Crypto),
        "index" => Ok(Category::Index),
        "stock" => Ok(Category::Stock),
        "commodity" => Ok(Category::Commodity),
        "etf" => Ok(Category::Etf),
        "bond" => Ok(Category::Bond),
        _ => bail!(
            "Unknown category: {}. Valid options: forex, crypto, index, stock, commodity, etf, bond",
            s
        ),
    }
}
