//! State management for persistent job storage.

use crate::{DownloadJob, JobId, JobStatus};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during state management operations.
#[derive(Error, Debug)]
pub enum StateError {
    /// Failed to determine the application data directory.
    #[error("Failed to determine application data directory")]
    NoDataDir,

    /// Failed to create a directory.
    #[error("Failed to create directory '{path}': {source}")]
    CreateDir {
        /// The path that could not be created.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to read a file.
    #[error("Failed to read file '{path}': {source}")]
    ReadFile {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to write a file.
    #[error("Failed to write file '{path}': {source}")]
    WriteFile {
        /// The path that could not be written.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to delete a file.
    #[error("Failed to delete file '{path}': {source}")]
    DeleteFile {
        /// The path that could not be deleted.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to parse JSON.
    #[error("Failed to parse job file '{path}': {source}")]
    ParseJson {
        /// The path that could not be parsed.
        path: PathBuf,
        /// The underlying JSON error.
        source: serde_json::Error,
    },

    /// Failed to serialize JSON.
    #[error("Failed to serialize job: {0}")]
    SerializeJson(#[from] serde_json::Error),

    /// Job not found.
    #[error("Job not found: {0}")]
    JobNotFound(JobId),

    /// Failed to read directory.
    #[error("Failed to read directory '{path}': {source}")]
    ReadDir {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to spawn daemon process.
    #[error("Failed to spawn daemon process '{executable}': {source}")]
    SpawnDaemon {
        /// The executable that could not be spawned.
        executable: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to determine executable path.
    #[error("Failed to determine executable path: {source}")]
    ExecutablePath {
        /// The underlying I/O error.
        source: std::io::Error,
    },
}

/// Result type for state operations.
pub type Result<T> = std::result::Result<T, StateError>;

/// Manages persistent state for download jobs.
///
/// Jobs are stored as JSON files in `~/.paracas/jobs/` with log files
/// stored in `~/.paracas/logs/`.
#[derive(Debug, Clone)]
pub struct StateManager {
    /// Base directory for state storage.
    base_path: PathBuf,
    /// Directory for job JSON files.
    jobs_path: PathBuf,
    /// Directory for job log files.
    logs_path: PathBuf,
}

impl StateManager {
    /// Creates a new state manager with the given base path.
    ///
    /// Creates the necessary subdirectories if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the directories cannot be created.
    pub fn new(base_path: PathBuf) -> Result<Self> {
        let jobs_path = base_path.join("jobs");
        let logs_path = base_path.join("logs");

        // Create directories if they don't exist
        for path in [&base_path, &jobs_path, &logs_path] {
            if !path.exists() {
                fs::create_dir_all(path).map_err(|e| StateError::CreateDir {
                    path: path.clone(),
                    source: e,
                })?;
            }
        }

        Ok(Self {
            base_path,
            jobs_path,
            logs_path,
        })
    }

    /// Returns the default path for paracas state storage.
    ///
    /// Uses the `directories` crate to find the appropriate location:
    /// - Linux: `~/.local/share/paracas/`
    /// - macOS: `~/Library/Application Support/paracas/`
    /// - Windows: `C:\Users\<User>\AppData\Roaming\paracas\`
    ///
    /// Falls back to `~/.paracas/` if the platform-specific location
    /// cannot be determined.
    #[must_use]
    pub fn default_path() -> PathBuf {
        ProjectDirs::from("", "", "paracas").map_or_else(dirs_fallback, |proj_dirs| {
            proj_dirs.data_dir().to_path_buf()
        })
    }

    /// Creates a state manager at the default path.
    ///
    /// # Errors
    ///
    /// Returns an error if the directories cannot be created.
    pub fn with_default_path() -> Result<Self> {
        Self::new(Self::default_path())
    }

    /// Returns the base path for state storage.
    #[must_use]
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Returns the path to a job's state file.
    #[must_use]
    pub fn job_state_path(&self, job_id: JobId) -> PathBuf {
        self.jobs_path.join(format!("{job_id}.json"))
    }

    /// Returns the path to a job's log file.
    #[must_use]
    pub fn job_log_path(&self, job_id: JobId) -> PathBuf {
        self.logs_path.join(format!("{job_id}.log"))
    }

    /// Saves a job to persistent storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the job cannot be serialized or written to disk.
    pub fn save_job(&self, job: &DownloadJob) -> Result<()> {
        let path = self.job_state_path(job.id);
        let json = serde_json::to_string_pretty(job)?;

        fs::write(&path, json).map_err(|e| StateError::WriteFile { path, source: e })
    }

    /// Loads a job from persistent storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the job file cannot be read or parsed.
    pub fn load_job(&self, job_id: JobId) -> Result<DownloadJob> {
        let path = self.job_state_path(job_id);

        if !path.exists() {
            return Err(StateError::JobNotFound(job_id));
        }

        let content = fs::read_to_string(&path).map_err(|e| StateError::ReadFile {
            path: path.clone(),
            source: e,
        })?;

        serde_json::from_str(&content).map_err(|e| StateError::ParseJson { path, source: e })
    }

    /// Lists all jobs in persistent storage.
    ///
    /// Returns jobs sorted by creation time (newest first).
    ///
    /// # Errors
    ///
    /// Returns an error if the jobs directory cannot be read.
    pub fn list_jobs(&self) -> Result<Vec<DownloadJob>> {
        let entries = fs::read_dir(&self.jobs_path).map_err(|e| StateError::ReadDir {
            path: self.jobs_path.clone(),
            source: e,
        })?;

        let mut jobs = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| StateError::ReadDir {
                path: self.jobs_path.clone(),
                source: e,
            })?;

            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = fs::read_to_string(&path).map_err(|e| StateError::ReadFile {
                    path: path.clone(),
                    source: e,
                })?;

                match serde_json::from_str::<DownloadJob>(&content) {
                    Ok(job) => jobs.push(job),
                    Err(e) => {
                        // Log warning but continue - don't fail on corrupt files
                        eprintln!("Warning: Failed to parse job file {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by creation time, newest first
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(jobs)
    }

    /// Deletes a job from persistent storage.
    ///
    /// Also deletes the associated log file if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the job file cannot be deleted.
    pub fn delete_job(&self, job_id: JobId) -> Result<()> {
        let state_path = self.job_state_path(job_id);

        if !state_path.exists() {
            return Err(StateError::JobNotFound(job_id));
        }

        fs::remove_file(&state_path).map_err(|e| StateError::DeleteFile {
            path: state_path,
            source: e,
        })?;

        // Also delete log file if it exists
        let log_path = self.job_log_path(job_id);
        if log_path.exists() {
            let _ = fs::remove_file(&log_path); // Ignore errors for log file
        }

        Ok(())
    }

    /// Returns all active (pending or running) jobs.
    ///
    /// # Errors
    ///
    /// Returns an error if jobs cannot be listed.
    pub fn active_jobs(&self) -> Result<Vec<DownloadJob>> {
        let jobs = self.list_jobs()?;
        Ok(jobs.into_iter().filter(|j| !j.is_finished()).collect())
    }

    /// Checks if a process with the given PID is still running.
    #[must_use]
    pub fn is_process_running(pid: u32) -> bool {
        // Use kill with signal 0 to check if process exists
        // This doesn't actually send a signal, just checks if the process exists
        #[cfg(unix)]
        {
            use std::process::Command;
            Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }

        #[cfg(windows)]
        {
            // On Windows, use tasklist to check if process exists
            use std::process::Command;
            Command::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid)])
                .output()
                .map(|output| {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.contains(&pid.to_string())
                })
                .unwrap_or(false)
        }

        #[cfg(not(any(unix, windows)))]
        {
            // On other platforms, assume the process is not running
            false
        }
    }

    /// Cleans up stale jobs where the process is no longer running.
    ///
    /// Marks running jobs as failed if their daemon process has died.
    ///
    /// # Errors
    ///
    /// Returns an error if jobs cannot be listed or updated.
    pub fn cleanup_stale_jobs(&self) -> Result<Vec<JobId>> {
        let jobs = self.list_jobs()?;
        let mut cleaned = Vec::new();

        for mut job in jobs {
            if job.status == JobStatus::Running {
                let is_stale = job.pid.is_none_or(|pid| !Self::is_process_running(pid));

                if is_stale {
                    job.mark_failed(Some("Daemon process died unexpectedly".to_string()));
                    self.save_job(&job)?;
                    cleaned.push(job.id);
                }
            }
        }

        Ok(cleaned)
    }
}

/// Fallback for determining home directory.
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".paracas")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstrumentTask;
    use tempfile::TempDir;

    fn create_test_job() -> DownloadJob {
        let tasks = vec![InstrumentTask::new(
            "EURUSD".to_string(),
            "2024-01-01".to_string(),
            "2024-01-02".to_string(),
            PathBuf::from("/tmp/eurusd.csv"),
            "csv".to_string(),
            "tick".to_string(),
            48,
        )];
        DownloadJob::new(tasks, 4)
    }

    #[test]
    fn test_state_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        assert!(manager.base_path().exists());
        assert!(temp_dir.path().join("jobs").exists());
        assert!(temp_dir.path().join("logs").exists());
    }

    #[test]
    fn test_save_and_load_job() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let job = create_test_job();
        let job_id = job.id;

        manager.save_job(&job).unwrap();

        let loaded = manager.load_job(job_id).unwrap();
        assert_eq!(loaded.id, job_id);
        assert_eq!(loaded.status, JobStatus::Pending);
        assert_eq!(loaded.tasks.len(), 1);
    }

    #[test]
    fn test_list_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let job1 = create_test_job();
        let job2 = create_test_job();

        manager.save_job(&job1).unwrap();
        manager.save_job(&job2).unwrap();

        let jobs = manager.list_jobs().unwrap();
        assert_eq!(jobs.len(), 2);
    }

    #[test]
    fn test_delete_job() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let job = create_test_job();
        let job_id = job.id;

        manager.save_job(&job).unwrap();
        assert!(manager.load_job(job_id).is_ok());

        manager.delete_job(job_id).unwrap();
        assert!(matches!(
            manager.load_job(job_id),
            Err(StateError::JobNotFound(_))
        ));
    }

    #[test]
    fn test_active_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let mut pending_job = create_test_job();
        let mut completed_job = create_test_job();
        completed_job.mark_completed();

        manager.save_job(&pending_job).unwrap();
        manager.save_job(&completed_job).unwrap();

        pending_job.mark_started(12345);
        manager.save_job(&pending_job).unwrap();

        let active = manager.active_jobs().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, pending_job.id);
    }

    #[test]
    fn test_job_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let result = manager.load_job(uuid::Uuid::new_v4());
        assert!(matches!(result, Err(StateError::JobNotFound(_))));
    }

    #[test]
    fn test_job_state_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let job_id = uuid::Uuid::new_v4();
        let path = manager.job_state_path(job_id);

        assert!(path.to_string_lossy().contains("jobs"));
        assert!(path.to_string_lossy().ends_with(".json"));
    }

    #[test]
    fn test_job_log_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let job_id = uuid::Uuid::new_v4();
        let path = manager.job_log_path(job_id);

        assert!(path.to_string_lossy().contains("logs"));
        assert!(path.to_string_lossy().ends_with(".log"));
    }
}
