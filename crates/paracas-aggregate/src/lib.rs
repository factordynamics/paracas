//! OHLCV aggregation for paracas tick data downloader.
//!
//! This crate provides tick-to-OHLCV (candlestick) aggregation:
//!
//! - [`Ohlcv`] - OHLCV bar data structure
//! - [`TickAggregator`] - Streaming tick aggregator

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod aggregator;
mod ohlcv;

pub use aggregator::TickAggregator;
pub use ohlcv::Ohlcv;
