//! Download command implementation.
//!
//! This module handles downloading tick data from Dukascopy and writing it to various output formats.

use crate::display::{Format, aggregate_ticks, write_ohlcv, write_ticks};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use paracas_daemon::{DaemonSpawner, DownloadJob, InstrumentTask, StateManager};
use paracas_lib::prelude::*;
use std::path::PathBuf;

/// Download tick data for an instrument.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn download(
    instrument_id: &str,
    start_str: Option<&str>,
    end_str: Option<&str>,
    output: Option<PathBuf>,
    format: Format,
    timeframe_str: Option<&str>,
    concurrency: usize,
    background: bool,
    _yes: bool,
    quiet: bool,
) -> Result<()> {
    // Handle background mode
    if background {
        return spawn_background_download(
            instrument_id,
            start_str,
            end_str,
            output,
            format,
            timeframe_str,
            concurrency,
        );
    }

    // Lookup instrument
    let registry = InstrumentRegistry::global();
    let instrument = registry
        .get(instrument_id)
        .with_context(|| format!("Unknown instrument: {instrument_id}"))?;

    // Parse start date (default to instrument's earliest available data)
    let start = match start_str {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("Invalid start date: {s}"))?,
        None => instrument
            .start_tick_date()
            .map(|dt| dt.date_naive())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2003, 5, 5).expect("valid date")),
    };

    // Parse end date (default to today)
    let end = match end_str {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("Invalid end date: {s}"))?,
        None => chrono::Utc::now().date_naive(),
    };

    let range = DateRange::new(start, end)?;

    // Determine output path (default to <instrument>.<format>)
    let output = output
        .unwrap_or_else(|| PathBuf::from(format!("{}.{}", instrument_id, format.extension())));

    // Parse timeframe
    let timeframe = match timeframe_str {
        Some(tf) => tf
            .parse::<Timeframe>()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        None => Timeframe::Tick,
    };

    // Create client
    let config = ClientConfig {
        concurrency,
        ..Default::default()
    };
    let client = DownloadClient::new(config)?;

    // Setup progress bar
    let total_hours = range.total_hours() as u64;
    let progress = if quiet {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total_hours);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} hours ({percent}%) {msg}")
                .expect("Invalid progress template")
                .progress_chars("=>-"),
        );
        pb.set_message(format!("{} {} -> {}", instrument.id(), start, end));
        pb
    };

    // Download and collect ticks using the resilient stream
    // This will retry on transient errors and skip hours that fail after retries
    let mut all_ticks: Vec<Tick> = Vec::new();
    let mut skipped_hours = 0u64;
    let mut stream = paracas_lib::tick_stream_resilient(&client, instrument, range);

    while let Some(batch) = stream.next().await {
        if batch.had_error() {
            skipped_hours += 1;
        }
        all_ticks.extend(batch.ticks);
        progress.inc(1);
    }

    let finish_msg = if skipped_hours > 0 {
        format!(
            "Downloaded {} ticks ({} hours skipped due to errors)",
            all_ticks.len(),
            skipped_hours
        )
    } else {
        format!("Downloaded {} ticks", all_ticks.len())
    };
    progress.finish_with_message(finish_msg);

    // Aggregate if needed
    if timeframe.is_tick() {
        // Write raw ticks
        write_ticks(&all_ticks, &output, format)?;
    } else {
        // Aggregate to OHLCV
        let bars = aggregate_ticks(&all_ticks, timeframe);
        write_ohlcv(&bars, &output, format)?;
    }

    if !quiet {
        println!("Output written to: {}", output.display());
    }

    Ok(())
}

/// Spawn a background download job for a single instrument.
#[allow(clippy::too_many_arguments)]
fn spawn_background_download(
    instrument_id: &str,
    start_str: Option<&str>,
    end_str: Option<&str>,
    output: Option<PathBuf>,
    format: Format,
    timeframe_str: Option<&str>,
    concurrency: usize,
) -> Result<()> {
    let registry = InstrumentRegistry::global();
    let instrument = registry
        .get(instrument_id)
        .with_context(|| format!("Unknown instrument: {instrument_id}"))?;

    // Determine start date
    let start = start_str
        .map(|s| s.to_string())
        .or_else(|| {
            instrument
                .start_tick_date()
                .map(|d| d.format("%Y-%m-%d").to_string())
        })
        .unwrap_or_else(|| "2003-05-05".to_string());

    // Determine end date
    let end = end_str
        .map(|s| s.to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());

    // Determine output path
    let output_path = output
        .unwrap_or_else(|| PathBuf::from(format!("{}.{}", instrument_id, format.extension())));

    // Make output path absolute
    let output_path = if output_path.is_absolute() {
        output_path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(output_path)
    };

    // Calculate total hours for progress tracking
    let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")?;
    let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")?;
    let range = DateRange::new(start_date, end_date)?;

    // Determine timeframe string (default to "tick")
    let timeframe = timeframe_str
        .map(|s| s.to_string())
        .unwrap_or_else(|| "tick".to_string());

    let task = InstrumentTask::new(
        instrument_id.to_string(),
        start,
        end,
        output_path,
        format.to_string(),
        timeframe,
        range.total_hours() as u32,
    );

    let mut job = DownloadJob::new(vec![task], concurrency);

    let state_manager =
        StateManager::with_default_path().context("Failed to initialize state manager")?;
    let spawner = DaemonSpawner::new(state_manager).context("Failed to create daemon spawner")?;
    let job_id = spawner
        .spawn(&mut job)
        .context("Failed to spawn background job")?;

    println!("Background download started.");
    println!("Job ID: {}", job_id);
    println!("Check status with: paracas status {}", job_id);

    Ok(())
}
