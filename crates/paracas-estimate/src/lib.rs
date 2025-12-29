//! Download size and time estimation for paracas tick data downloader.
//!
//! This crate provides utilities for estimating download sizes and times
//! based on historical averages for different instrument categories:
//!
//! - [`EstimateDatabase`] - Database of historical size estimates per category
//! - [`CategoryEstimate`] - Size estimates for a single category
//! - [`Estimator`] - Computes download estimates for instruments and date ranges
//! - [`DownloadEstimate`] - Estimated download metrics
//! - [`EstimateConfidence`] - Confidence level of the estimate

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod data;
mod estimator;

pub use data::{CategoryEstimate, EstimateDatabase};
pub use estimator::{DownloadEstimate, EstimateConfidence, Estimator};
