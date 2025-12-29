//! Output formatters for paracas tick data downloader.
//!
//! This crate provides formatters for writing tick and OHLCV data
//! to various output formats:
//!
//! - [`CsvFormatter`] - CSV format
//! - [`JsonFormatter`] - JSON array or NDJSON format
//! - [`ParquetFormatter`] - Apache Parquet columnar format

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod csv;
mod formatter;
mod json;

#[cfg(feature = "parquet")]
mod parquet;

pub use crate::csv::CsvFormatter;
pub use formatter::{FormatError, Formatter, OutputFormat};
pub use json::{JsonFormatter, JsonStyle};

#[cfg(feature = "parquet")]
pub use crate::parquet::ParquetFormatter;
