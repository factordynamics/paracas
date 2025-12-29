//! Core types for paracas tick data downloader.
//!
//! This crate provides the fundamental data structures used throughout paracas:
//!
//! - [`Tick`] - A single price tick with timestamp, ask, bid, and volumes
//! - [`RawTick`] - Raw tick from bi5 binary format before price normalization
//! - [`Instrument`] - Financial instrument with metadata
//! - [`Timeframe`] - OHLCV aggregation timeframe
//! - [`DateRange`] - Date range for data retrieval

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod date_range;
mod error;
mod instrument;
mod tick;
mod timeframe;

pub use date_range::{DateRange, HourIterator, hour_from_url};
pub use error::{DateRangeError, ParacasError, Result};
pub use instrument::{Category, Instrument};
pub use tick::{RawTick, Tick};
pub use timeframe::{Timeframe, TimeframeParseError};
