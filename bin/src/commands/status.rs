//! Background job status command.

use anyhow::{Context, Result};
use inquire::Select;
use paracas_daemon::{DownloadJob, JobStatus, StateManager};

/// Execute the status command.
pub(crate) fn status(
    job_id: Option<&str>,
    running_only: bool,
    show_all: bool,
    follow: Option<u64>,
    cancel_id: Option<&str>,
) -> Result<()> {
    let state_manager =
        StateManager::with_default_path().context("Failed to initialize state manager")?;

    // Handle cancellation request
    // cancel_id is Some("") when --cancel is passed without a value
    // cancel_id is Some(id) when --cancel <id> is passed
    if let Some(id) = cancel_id {
        let id_opt = if id.is_empty() { None } else { Some(id) };
        return cancel_job(&state_manager, id_opt);
    }

    // Handle follow/watch mode
    if let Some(interval) = follow {
        return watch_jobs(&state_manager, job_id, interval);
    }

    // Show specific job or list jobs
    #[allow(clippy::option_if_let_else)]
    match job_id {
        Some(id) => show_job_detail(&state_manager, id),
        None => list_jobs(&state_manager, running_only, show_all),
    }
}

fn show_job_detail(state: &StateManager, job_id: &str) -> Result<()> {
    let id = job_id.parse().context("Invalid job ID format")?;

    let job = state.load_job(id).context("Job not found")?;

    println!("Job: {}", job.id);
    println!("Status: {:?}", job.status);
    println!("Created: {}", job.created_at.format("%Y-%m-%d %H:%M:%S"));

    if let Some(started) = job.started_at {
        println!("Started: {}", started.format("%Y-%m-%d %H:%M:%S"));
    }
    if let Some(completed) = job.completed_at {
        println!("Completed: {}", completed.format("%Y-%m-%d %H:%M:%S"));
    }

    println!("Progress: {:.1}%", job.progress_percent());
    println!(
        "PID: {}",
        job.pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "N/A".into())
    );
    println!(
        "Log: {}",
        job.log_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "N/A".into())
    );

    println!("\nTasks:");
    for (i, task) in job.tasks.iter().enumerate() {
        let progress = if task.hours_total > 0 {
            (task.hours_completed as f64 / task.hours_total as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "  {}. {} [{:?}] {:.1}% ({}/{} hours)",
            i + 1,
            task.instrument_id,
            task.status,
            progress,
            task.hours_completed,
            task.hours_total,
        );
        if let Some(ref err) = task.error_message {
            println!("     Error: {}", err);
        }
    }

    Ok(())
}

fn list_jobs(state: &StateManager, running_only: bool, show_all: bool) -> Result<()> {
    let jobs = state.list_jobs()?;

    let filtered: Vec<_> = jobs
        .into_iter()
        .filter(|job| {
            if running_only {
                matches!(job.status, JobStatus::Running | JobStatus::Pending)
            } else if show_all {
                true
            } else {
                // Default: show recent (last 24h) or active
                let is_recent = job.created_at > chrono::Utc::now() - chrono::Duration::hours(24);
                is_recent || matches!(job.status, JobStatus::Running | JobStatus::Pending)
            }
        })
        .collect();

    if filtered.is_empty() {
        println!("No jobs found.");
        if !show_all {
            println!("Use --all to show all historical jobs.");
        }
        return Ok(());
    }

    println!(
        "{:<36} {:<12} {:<10} {:<20}",
        "JOB ID", "STATUS", "PROGRESS", "CREATED"
    );
    println!("{}", "-".repeat(80));

    for job in &filtered {
        println!(
            "{:<36} {:<12} {:>8.1}% {:<20}",
            job.id,
            format!("{:?}", job.status),
            job.progress_percent(),
            job.created_at.format("%Y-%m-%d %H:%M"),
        );
    }

    println!("\nTotal: {} jobs", filtered.len());
    Ok(())
}

/// Prompt user to select a job from available cancellable jobs.
fn prompt_cancel_selection(state: &StateManager) -> Result<String> {
    let jobs = state.list_jobs()?;

    let filtered: Vec<_> = jobs
        .into_iter()
        .filter(|job| matches!(job.status, JobStatus::Running | JobStatus::Pending))
        .collect();

    if filtered.is_empty() {
        anyhow::bail!("No running or pending jobs found to cancel.");
    }

    let options: Vec<String> = filtered
        .iter()
        .map(|job| {
            let instruments: Vec<_> = job.tasks.iter().map(|t| t.instrument_id.as_str()).collect();
            let instruments_display = if instruments.len() > 3 {
                format!(
                    "{}, ... (+{} more)",
                    instruments[..3].join(", "),
                    instruments.len() - 3
                )
            } else {
                instruments.join(", ")
            };
            format!(
                "{} | {:?} | {:.1}% | {}",
                job.id,
                job.status,
                job.progress_percent(),
                instruments_display
            )
        })
        .collect();

    let selection = Select::new("Select a job to cancel:", options)
        .prompt()
        .context("Job selection cancelled")?;

    // Extract the job ID from the selection (first part before " | ")
    let job_id = selection
        .split(" | ")
        .next()
        .context("Failed to parse job selection")?
        .to_string();

    Ok(job_id)
}

fn cancel_job(state: &StateManager, job_id: Option<&str>) -> Result<()> {
    let id_str = match job_id {
        Some(id) => id.to_string(),
        None => prompt_cancel_selection(state)?,
    };

    let id = id_str.parse().context("Invalid job ID format")?;

    let mut job: DownloadJob = state.load_job(id).context("Job not found")?;

    if !matches!(job.status, JobStatus::Running | JobStatus::Pending) {
        anyhow::bail!("Job is not running (status: {:?})", job.status);
    }

    // Send SIGTERM to the process if running
    if let Some(pid) = job.pid {
        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
        }
        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string()])
                .status();
        }
    }

    job.status = JobStatus::Cancelled;
    state.save_job(&job)?;

    println!("Job {} cancelled.", id);
    Ok(())
}

fn watch_jobs(state: &StateManager, job_id: Option<&str>, interval_secs: u64) -> Result<()> {
    use std::io::Write;

    let interval = std::time::Duration::from_secs(interval_secs);

    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        std::io::stdout().flush()?;

        println!(
            "Watching jobs (refresh every {}s, Ctrl+C to exit)\n",
            interval_secs
        );

        match job_id {
            Some(id) => show_job_detail(state, id)?,
            None => list_jobs(state, true, false)?,
        }

        std::thread::sleep(interval);
    }
}
