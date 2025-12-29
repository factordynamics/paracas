//! Job management commands (pause, resume, kill, clean).

use anyhow::{Context, Result};
use paracas_daemon::{DaemonSpawner, DownloadJob, JobStatus, StateManager};

/// Pause a running job by sending SIGSTOP to its process.
pub(crate) fn pause_job(state: &StateManager, job_id: &str) -> Result<()> {
    let id = job_id.parse().context("Invalid job ID format")?;

    let mut job: DownloadJob = state.load_job(id).context("Job not found")?;

    if job.status != JobStatus::Running {
        anyhow::bail!("Job is not running (status: {})", job.status);
    }

    let Some(pid) = job.pid else {
        anyhow::bail!("Job has no associated process");
    };

    // Send SIGSTOP to pause the process
    #[cfg(unix)]
    {
        use std::process::Command;
        let status = Command::new("kill")
            .args(["-STOP", &pid.to_string()])
            .status()
            .context("Failed to send SIGSTOP")?;

        if !status.success() {
            anyhow::bail!("Failed to pause process {}", pid);
        }
    }

    #[cfg(windows)]
    {
        // Windows doesn't have SIGSTOP equivalent, we'll just update the state
        eprintln!("Warning: Pause is not fully supported on Windows. Job state updated but process continues.");
    }

    job.mark_paused();
    state.save_job(&job)?;

    println!("Job {} paused.", id);
    Ok(())
}

/// Resume a paused job by sending SIGCONT to its process.
pub(crate) fn resume_job(state: &StateManager, job_id: &str) -> Result<()> {
    let id = job_id.parse().context("Invalid job ID format")?;

    let mut job: DownloadJob = state.load_job(id).context("Job not found")?;

    if job.status != JobStatus::Paused {
        anyhow::bail!("Job is not paused (status: {})", job.status);
    }

    let Some(pid) = job.pid else {
        anyhow::bail!("Job has no associated process");
    };

    // Check if the process is still alive
    if !StateManager::is_process_running(pid) {
        // Process is dead, need to respawn
        println!("Process {} is no longer running. Respawning daemon...", pid);
        return respawn_job(state, &mut job);
    }

    // Send SIGCONT to resume the process
    #[cfg(unix)]
    {
        use std::process::Command;
        let status = Command::new("kill")
            .args(["-CONT", &pid.to_string()])
            .status()
            .context("Failed to send SIGCONT")?;

        if !status.success() {
            anyhow::bail!("Failed to resume process {}", pid);
        }
    }

    #[cfg(windows)]
    {
        eprintln!("Warning: Resume is not fully supported on Windows.");
    }

    job.mark_resumed(pid);
    state.save_job(&job)?;

    println!("Job {} resumed.", id);
    Ok(())
}

/// Respawn a job that needs to be resumed but whose process is dead.
fn respawn_job(state: &StateManager, job: &mut DownloadJob) -> Result<()> {
    let spawner = DaemonSpawner::new(state.clone()).context("Failed to create daemon spawner")?;

    // Reset job status to pending so it can be picked up
    job.status = JobStatus::Pending;
    job.pid = None;

    spawner.spawn(job).context("Failed to respawn daemon")?;

    println!("Job {} respawned with PID {:?}.", job.id, job.pid);
    Ok(())
}

/// Kill a running or paused job by sending SIGKILL to its process.
pub(crate) fn kill_job(state: &StateManager, job_id: &str) -> Result<()> {
    let id = job_id.parse().context("Invalid job ID format")?;

    let mut job: DownloadJob = state.load_job(id).context("Job not found")?;

    if !matches!(
        job.status,
        JobStatus::Running | JobStatus::Pending | JobStatus::Paused
    ) {
        anyhow::bail!("Job is not active (status: {})", job.status);
    }

    // Send SIGKILL to the process if it exists
    if let Some(pid) = job.pid {
        #[cfg(unix)]
        {
            use std::process::Command;
            // First try SIGTERM for graceful shutdown
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();

            // Wait briefly then force kill if still running
            std::thread::sleep(std::time::Duration::from_millis(500));

            if StateManager::is_process_running(pid) {
                let _ = Command::new("kill")
                    .args(["-KILL", &pid.to_string()])
                    .status();
            }
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .status();
        }
    }

    job.mark_cancelled();
    state.save_job(&job)?;

    println!("Job {} killed.", id);
    Ok(())
}

/// Clean up completed, failed, or cancelled jobs from storage.
pub(crate) fn clean_jobs(state: &StateManager, all: bool) -> Result<()> {
    let jobs = state.list_jobs()?;

    let mut cleaned_count = 0;

    for job in jobs {
        let should_clean = if all {
            job.is_finished()
        } else {
            // By default, only clean jobs older than 24 hours that are finished
            let is_old = job.created_at < chrono::Utc::now() - chrono::Duration::hours(24);
            is_old && job.is_finished()
        };

        if should_clean {
            state.delete_job(job.id)?;
            cleaned_count += 1;
        }
    }

    if cleaned_count == 0 {
        println!("No jobs to clean.");
    } else {
        println!("Cleaned {} job(s).", cleaned_count);
    }

    Ok(())
}

/// Execute the job management command.
pub(crate) fn job_command(
    action: &str,
    job_id: Option<&str>,
    all: bool,
) -> Result<()> {
    let state_manager =
        StateManager::with_default_path().context("Failed to initialize state manager")?;

    match action {
        "pause" => {
            let id = job_id.context("Job ID required for pause")?;
            pause_job(&state_manager, id)
        }
        "resume" => {
            let id = job_id.context("Job ID required for resume")?;
            resume_job(&state_manager, id)
        }
        "kill" => {
            let id = job_id.context("Job ID required for kill")?;
            kill_job(&state_manager, id)
        }
        "clean" => clean_jobs(&state_manager, all),
        _ => anyhow::bail!("Unknown action: {}", action),
    }
}
