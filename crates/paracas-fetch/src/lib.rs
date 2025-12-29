//! HTTP client and data fetching for paracas tick data downloader.
//!
//! This crate provides the data download pipeline:
//!
//! - [`url::tick_url`] - Constructs Dukascopy data URLs
//! - [`DownloadClient`] - HTTP client with connection pooling and retries
//! - [`decompress::decompress_bi5`] - LZMA decompression
//! - [`parse::parse_ticks`] - Binary tick data parsing
//! - [`tick_stream`] - Async streaming tick download

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod client;
mod decompress;
mod parse;
mod stream;
pub mod url;

pub use client::{ClientConfig, DownloadClient, DownloadError};
pub use decompress::{DecompressError, decompress_bi5};
pub use parse::{ParseError, parse_ticks, tick_count};
pub use stream::{TickBatch, flatten_ticks, tick_stream, tick_stream_resilient};
