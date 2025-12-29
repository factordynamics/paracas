//! Error types for paracas.

use chrono::NaiveDate;
use thiserror::Error;

/// Result type alias for paracas operations.
pub type Result<T> = std::result::Result<T, ParacasError>;

/// Errors that can occur during data download and processing.
#[derive(Error, Debug)]
pub enum ParacasError {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(String),

    /// LZMA decompression failed.
    #[error("Decompression error: {0}")]
    Decompress(String),

    /// Invalid data format.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Instrument not found.
    #[error("Unknown instrument: {0}")]
    UnknownInstrument(String),

    /// Invalid date range.
    #[error(transparent)]
    DateRange(#[from] DateRangeError),

    /// No data available for the requested period.
    #[error("No data available for {instrument} in requested range")]
    NoDataAvailable {
        /// The instrument that had no data.
        instrument: String,
    },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Output format error.
    #[error("Format error: {0}")]
    Format(String),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Error for invalid date ranges.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DateRangeError {
    /// Start date is after end date.
    #[error("Invalid date range: {start} > {end}")]
    InvalidRange {
        /// The start date.
        start: NaiveDate,
        /// The end date.
        end: NaiveDate,
    },
}
