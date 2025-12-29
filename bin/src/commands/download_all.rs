//! Download all instruments command.
//!
//! This module handles batch downloading of multiple instruments, with support for
//! category filtering, parallel downloads, and download estimation.

use crate::display::{Format, aggregate_ticks, parse_category, write_ohlcv, write_ticks};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use paracas_daemon::{DaemonSpawner, DownloadJob, InstrumentTask, StateManager};
use paracas_estimate::Estimator;
use paracas_lib::prelude::*;
use std::io::Write as _;
use std::path::PathBuf;

/// Execute the download-all command.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn download_all(
    category: Option<&str>,
    start_str: Option<&str>,
    end_str: Option<&str>,
    output_dir: PathBuf,
    format: Format,
    timeframe_str: Option<&str>,
    parallel_instruments: usize,
    concurrency: usize,
    background: bool,
    yes: bool,
    quiet: bool,
) -> Result<()> {
    // 1. Get instruments based on category filter (or all)
    let registry = InstrumentRegistry::global();
    let instruments: Vec<_> = match category {
        Some(cat) => {
            let category = parse_category(cat)?;
            registry.by_category(category).collect()
        }
        None => registry.all().collect(),
    };

    if instruments.is_empty() {
        anyhow::bail!("No instruments found matching criteria");
    }

    // Parse end date (default to today)
    let today = chrono::Utc::now().date_naive();
    let end = match end_str {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("Invalid end date: {s}"))?,
        None => today,
    };

    // Parse start date or use earliest instrument date
    let start = match start_str {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("Invalid start date: {s}"))?,
        None => {
            // Use the earliest start date among all selected instruments
            instruments
                .iter()
                .filter_map(|i| i.start_tick_date())
                .map(|dt| dt.date_naive())
                .min()
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2003, 5, 5).expect("valid date"))
        }
    };

    let range = DateRange::new(start, end)?;

    // 2. Show estimate and get confirmation
    let estimator = Estimator::global();
    let estimate = estimator.estimate_batch(&instruments, &range);

    if !yes && !quiet {
        println!("Download plan:");
        println!("  Instruments: {}", instruments.len());
        println!("  Date range: {} to {}", start, end);
        println!(
            "  Estimated download size: {}",
            Estimator::format_bytes(estimate.estimated_compressed_bytes)
        );
        println!(
            "  Estimated output size: {}",
            Estimator::format_bytes(estimate.estimated_output_bytes)
        );
        println!(
            "  Estimated time: {}",
            Estimator::format_duration(estimate.estimated_duration)
        );
        println!();

        // Simple y/n confirmation
        print!("Proceed with download? [y/N] ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // 3. If background mode, spawn daemon
    if background {
        return spawn_background_download_all(
            &instruments,
            start,
            end,
            &output_dir,
            format,
            timeframe_str,
            concurrency,
        );
    }

    // 4. Create output directory if needed
    std::fs::create_dir_all(&output_dir)?;

    // 5. Parse timeframe
    let timeframe = match timeframe_str {
        Some(tf) => tf
            .parse::<Timeframe>()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        None => Timeframe::Tick,
    };

    // 6. Download instruments in parallel
    let multi_progress = MultiProgress::new();

    let results: Vec<_> = stream::iter(instruments.into_iter())
        .map(|instrument| {
            let pb = multi_progress.add(ProgressBar::new(100));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{prefix:.bold} [{bar:30.cyan/blue}] {percent}% {msg}")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            pb.set_prefix(format!("{:>12}", instrument.id()));

            download_single_instrument(
                instrument,
                start,
                end,
                output_dir.clone(),
                format,
                timeframe,
                concurrency,
                pb,
                quiet,
            )
        })
        .buffer_unordered(parallel_instruments)
        .collect()
        .await;

    // 7. Report summary
    let (successes, failures): (Vec<_>, Vec<_>) = results.iter().partition(|r| r.is_ok());

    if !quiet {
        println!("\nDownload complete:");
        println!("  Successful: {}", successes.len());
        if !failures.is_empty() {
            println!("  Failed: {}", failures.len());
            for (i, err) in failures.iter().enumerate() {
                if let Err(e) = err {
                    println!("    {}: {}", i + 1, e);
                }
            }
        }
    }

    // Return error if any downloads failed
    if !failures.is_empty() {
        anyhow::bail!(
            "{} out of {} downloads failed",
            failures.len(),
            successes.len() + failures.len()
        );
    }

    Ok(())
}

/// Download a single instrument with progress tracking.
#[allow(clippy::too_many_arguments)]
async fn download_single_instrument(
    instrument: &Instrument,
    start: NaiveDate,
    end: NaiveDate,
    output_dir: PathBuf,
    format: Format,
    timeframe: Timeframe,
    concurrency: usize,
    progress: ProgressBar,
    quiet: bool,
) -> Result<()> {
    // Adjust start date based on instrument's available data
    let effective_start = instrument
        .start_tick_date()
        .map_or(start, |instrument_start| {
            let instrument_start_date = instrument_start.date_naive();
            if start < instrument_start_date {
                instrument_start_date
            } else {
                start
            }
        });

    // Skip if the instrument has no data in the requested range
    if effective_start > end {
        progress.finish_with_message("skipped (no data)");
        return Ok(());
    }

    let range = DateRange::new(effective_start, end)?;
    let total_hours = range.total_hours() as u64;
    progress.set_length(total_hours);

    // Create client
    let config = ClientConfig {
        concurrency,
        ..Default::default()
    };
    let client = DownloadClient::new(config)?;

    // Download and collect ticks
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

    let tick_count = all_ticks.len();
    let finish_msg = if skipped_hours > 0 {
        format!("{} ticks ({} hrs skipped)", tick_count, skipped_hours)
    } else {
        format!("{} ticks", tick_count)
    };
    progress.finish_with_message(finish_msg);

    // Determine output path
    let output_path = output_dir.join(format!("{}.{}", instrument.id(), format.extension()));

    // Aggregate if needed
    if timeframe.is_tick() {
        write_ticks(&all_ticks, &output_path, format)?;
    } else {
        let bars = aggregate_ticks(&all_ticks, timeframe);
        write_ohlcv(&bars, &output_path, format)?;
    }

    if !quiet {
        progress.println(format!("  Written: {}", output_path.display()));
    }

    Ok(())
}

/// Spawn a background download job for multiple instruments.
#[allow(clippy::too_many_arguments)]
fn spawn_background_download_all(
    instruments: &[&Instrument],
    start: NaiveDate,
    end: NaiveDate,
    output_dir: &PathBuf,
    format: Format,
    timeframe_str: Option<&str>,
    concurrency: usize,
) -> Result<()> {
    // Make output directory absolute
    let output_dir = if output_dir.is_absolute() {
        output_dir.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(output_dir)
    };

    // Create output directory if needed
    std::fs::create_dir_all(&output_dir)?;

    // Determine timeframe string (default to "tick")
    let timeframe = timeframe_str
        .map(|s| s.to_string())
        .unwrap_or_else(|| "tick".to_string());

    // Create tasks for each instrument
    let mut tasks = Vec::with_capacity(instruments.len());

    for instrument in instruments {
        // Adjust start date based on instrument's available data
        let effective_start = instrument
            .start_tick_date()
            .map_or(start, |instrument_start| {
                let instrument_start_date = instrument_start.date_naive();
                if start < instrument_start_date {
                    instrument_start_date
                } else {
                    start
                }
            });

        // Skip if the instrument has no data in the requested range
        if effective_start > end {
            continue;
        }

        let range = DateRange::new(effective_start, end)?;
        let output_path = output_dir.join(format!("{}.{}", instrument.id(), format.extension()));

        let task = InstrumentTask::new(
            instrument.id().to_string(),
            effective_start.format("%Y-%m-%d").to_string(),
            end.format("%Y-%m-%d").to_string(),
            output_path,
            format.to_string(),
            timeframe.clone(),
            range.total_hours() as u32,
        );

        tasks.push(task);
    }

    if tasks.is_empty() {
        anyhow::bail!("No instruments with data in the specified date range");
    }

    let mut job = DownloadJob::new(tasks, concurrency);

    let state_manager =
        StateManager::with_default_path().context("Failed to initialize state manager")?;
    let spawner = DaemonSpawner::new(state_manager).context("Failed to create daemon spawner")?;
    let job_id = spawner
        .spawn(&mut job)
        .context("Failed to spawn background job")?;

    println!("Background download started.");
    println!("Job ID: {}", job_id);
    println!("Instruments: {}", job.tasks.len());
    println!("Check status with: paracas status {}", job_id);

    Ok(())
}
