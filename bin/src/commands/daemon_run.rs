//! Hidden daemon entry point for background downloads.
//!
//! This module provides the entry point for daemon processes spawned
//! with `--daemon-run <job_id>`. It loads the job from disk and executes
//! the download tasks.

use crate::display::{Format, aggregate_ticks, write_ohlcv, write_ticks};
use anyhow::{Context, Result, bail};
use futures::StreamExt;
use paracas_daemon::{DaemonProgress, JobId, JobStatus, StateManager};
use paracas_lib::prelude::*;
use std::path::PathBuf;

/// Execute a background download job.
///
/// This is called when paracas is spawned with `--daemon-run <job_id>`.
/// The function loads the job from disk, executes all pending tasks,
/// and saves progress periodically.
pub(crate) async fn daemon_run(job_id_str: &str) -> Result<()> {
    let job_id: JobId = job_id_str.parse().context("Invalid job ID")?;

    let state_manager =
        StateManager::with_default_path().context("Failed to initialize state manager")?;

    let job = state_manager.load_job(job_id).context("Job not found")?;

    if !matches!(job.status, JobStatus::Pending | JobStatus::Running) {
        bail!("Job is not in a runnable state: {:?}", job.status);
    }

    let progress = DaemonProgress::new(state_manager.clone(), job);

    // Mark job as running
    {
        let mut job = progress.job().await;
        job.mark_started(std::process::id());
        state_manager.save_job(&job)?;
    }

    // Process each task
    let job = progress.job().await;
    for (task_idx, task) in job.tasks.iter().enumerate() {
        if matches!(task.status, JobStatus::Completed) {
            continue; // Skip already completed tasks
        }

        if let Err(e) = execute_task(&progress, task_idx).await {
            progress.mark_task_failed(task_idx, &e.to_string()).await;
        }

        progress.save_checkpoint().await?;
    }

    // Mark job as completed or failed based on task results
    if progress.all_tasks_finished().await {
        if progress.failed_tasks().await == 0 {
            progress.mark_job_completed().await;
        } else {
            let failed_count = progress.failed_tasks().await;
            let msg = format!("{} tasks failed", failed_count);
            progress.mark_job_failed(&msg).await;
        }
    }

    progress.save_checkpoint().await?;

    Ok(())
}

/// Execute a single download task.
async fn execute_task(progress: &DaemonProgress, task_idx: usize) -> Result<()> {
    progress.mark_task_running(task_idx).await;

    let job = progress.job().await;
    let task = &job.tasks[task_idx];

    // Get instrument
    let registry = InstrumentRegistry::global();
    let instrument = registry
        .get(&task.instrument_id)
        .context("Unknown instrument")?;

    // Parse date range
    let start = chrono::NaiveDate::parse_from_str(&task.start_date, "%Y-%m-%d")?;
    let end = chrono::NaiveDate::parse_from_str(&task.end_date, "%Y-%m-%d")?;
    let range = DateRange::new(start, end)?;

    // Create client
    let config = ClientConfig {
        concurrency: job.concurrency,
        ..Default::default()
    };
    let client = DownloadClient::new(config)?;

    // Download ticks
    let mut all_ticks: Vec<Tick> = Vec::new();
    let mut stream = paracas_lib::tick_stream_resilient(&client, instrument, range);
    let mut hours_completed = 0u64;

    while let Some(batch) = stream.next().await {
        all_ticks.extend(batch.ticks);
        hours_completed += 1;

        // Update progress periodically (every 10 hours)
        if hours_completed.is_multiple_of(10) {
            progress
                .update_task_progress(task_idx, hours_completed, all_ticks.len() as u64)
                .await;
        }
    }

    // Parse timeframe and aggregate if needed
    let timeframe = task
        .timeframe
        .parse::<Timeframe>()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Parse format
    let format = parse_format(&task.format)?;

    // Write output
    let output_path = task.output_path.clone();
    write_output(&all_ticks, &output_path, format, timeframe)?;

    let bytes_written = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    progress.mark_task_completed(task_idx, bytes_written).await;

    Ok(())
}

/// Parse a format string into a Format enum.
fn parse_format(format: &str) -> Result<Format> {
    match format.to_lowercase().as_str() {
        "csv" => Ok(Format::Csv),
        "json" => Ok(Format::Json),
        "ndjson" => Ok(Format::Ndjson),
        "parquet" => Ok(Format::Parquet),
        _ => bail!("Unknown format: {}", format),
    }
}

/// Write ticks or OHLCV data to the output file.
fn write_output(
    ticks: &[Tick],
    output: &PathBuf,
    format: Format,
    timeframe: Timeframe,
) -> Result<()> {
    if timeframe.is_tick() {
        write_ticks(ticks, output, format)?;
    } else {
        let bars = aggregate_ticks(ticks, timeframe);
        write_ohlcv(&bars, output, format)?;
    }
    Ok(())
}
