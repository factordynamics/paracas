//! Output format abstraction.

use paracas_aggregate::Ohlcv;
use paracas_types::Tick;
use std::io::Write;
use thiserror::Error;

/// Output format identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OutputFormat {
    /// CSV format.
    #[default]
    Csv,
    /// JSON array format.
    Json,
    /// Newline-delimited JSON format.
    Ndjson,
    /// Apache Parquet format.
    Parquet,
}

impl OutputFormat {
    /// Returns the file extension for this format.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
            Self::Parquet => "parquet",
        }
    }

    /// Returns all available formats.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Csv, Self::Json, Self::Ndjson, Self::Parquet]
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            "ndjson" | "jsonl" => Ok(Self::Ndjson),
            "parquet" | "pq" => Ok(Self::Parquet),
            _ => Err(FormatError::UnknownFormat(s.to_string())),
        }
    }
}

/// Errors that can occur during formatting.
#[derive(Error, Debug)]
pub enum FormatError {
    /// Unknown output format.
    #[error("Unknown format: {0}")]
    UnknownFormat(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Arrow/Parquet error.
    #[error("Parquet error: {0}")]
    Parquet(String),
}

/// Trait for output formatters.
pub trait Formatter: Send + Sync {
    /// Writes tick data to the output.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    fn write_ticks<W: Write + Send>(&self, ticks: &[Tick], writer: W) -> Result<(), FormatError>;

    /// Writes OHLCV data to the output.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    fn write_ohlcv<W: Write + Send>(&self, bars: &[Ohlcv], writer: W) -> Result<(), FormatError>;

    /// Returns the file extension for this format.
    fn extension(&self) -> &str;
}
