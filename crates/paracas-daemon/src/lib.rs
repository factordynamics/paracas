//! Background job management for paracas tick data downloader.
//!
//! This crate provides state management and job tracking for background
//! download operations:
//!
//! - [`JobId`] - Unique identifier for download jobs
//! - [`JobStatus`] - Current status of a job
//! - [`InstrumentTask`] - Download task for a single instrument
//! - [`DownloadJob`] - Complete download job with multiple tasks
//! - [`StateManager`] - Persistent state storage and retrieval
//! - [`DaemonSpawner`] - Spawns detached daemon processes for background downloads
//! - [`DaemonProgress`] - Thread-safe progress tracking for daemon jobs

#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/paracas/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod daemon;
mod job;
mod progress;
mod state;

pub use daemon::{DAEMON_JOB_ID_ENV, DAEMON_RUN_ARG, DaemonSpawner};
pub use job::{DownloadJob, InstrumentTask, JobId, JobStatus};
pub use progress::DaemonProgress;
pub use state::{Result, StateError, StateManager};
