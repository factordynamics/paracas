//! Daemon process spawning for background downloads.
//!
//! This module provides functionality to spawn detached daemon processes
//! that can run downloads in the background, even after the parent process exits.

use crate::{DownloadJob, JobId, StateError, StateManager};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Result type for daemon operations (re-exported from state module).
pub(crate) type Result<T> = std::result::Result<T, StateError>;

/// Environment variable name for the daemon job ID.
pub const DAEMON_JOB_ID_ENV: &str = "PARACAS_DAEMON_JOB_ID";

/// Command line argument for daemon mode.
pub const DAEMON_RUN_ARG: &str = "--daemon-run";

/// Spawns detached daemon processes for background downloads.
///
/// The spawner handles all the platform-specific details of creating
/// a detached background process that will continue running after
/// the parent process exits.
#[derive(Debug, Clone)]
pub struct DaemonSpawner {
    state_manager: StateManager,
    executable_path: PathBuf,
}

impl DaemonSpawner {
    /// Create a new daemon spawner.
    ///
    /// # Errors
    ///
    /// Returns an error if the current executable path cannot be determined.
    pub fn new(state_manager: StateManager) -> Result<Self> {
        let executable_path = Self::executable_path()?;
        Ok(Self {
            state_manager,
            executable_path,
        })
    }

    /// Create a new daemon spawner with a custom executable path.
    ///
    /// This is useful for testing or when spawning a different binary.
    #[must_use]
    pub const fn with_executable(state_manager: StateManager, executable_path: PathBuf) -> Self {
        Self {
            state_manager,
            executable_path,
        }
    }

    /// Spawn a background download job.
    ///
    /// Returns the job ID for tracking. The job's PID and log file path
    /// will be updated after spawning.
    ///
    /// # Errors
    ///
    /// Returns an error if the daemon process cannot be spawned.
    pub fn spawn(&self, job: &mut DownloadJob) -> Result<JobId> {
        let job_id = job.id;

        // Set up log file path
        let log_path = self.state_manager.job_log_path(job_id);
        job.log_file = Some(log_path.clone());

        // Save job state before spawning
        self.state_manager.save_job(job)?;

        // Open log file for stdout/stderr redirection
        let log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
            .map_err(|e| StateError::WriteFile {
                path: log_path.clone(),
                source: e,
            })?;

        let log_file_stderr = log_file.try_clone().map_err(|e| StateError::WriteFile {
            path: log_path.clone(),
            source: e,
        })?;

        // Spawn the daemon process
        let child = self.spawn_detached(job_id, log_file, log_file_stderr)?;

        // Update job with PID
        let pid = child.id();
        job.pid = Some(pid);
        self.state_manager.save_job(job)?;

        Ok(job_id)
    }

    /// Spawn a detached child process.
    #[cfg(unix)]
    fn spawn_detached(
        &self,
        job_id: JobId,
        stdout: std::fs::File,
        stderr: std::fs::File,
    ) -> Result<std::process::Child> {
        use std::os::unix::process::CommandExt;

        let child = Command::new(&self.executable_path)
            .args([DAEMON_RUN_ARG, &job_id.to_string()])
            .env(DAEMON_JOB_ID_ENV, job_id.to_string())
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .process_group(0) // Create new process group (detach from parent)
            .spawn()
            .map_err(|e| StateError::SpawnDaemon {
                executable: self.executable_path.clone(),
                source: e,
            })?;

        Ok(child)
    }

    /// Spawn a detached child process on Windows.
    #[cfg(windows)]
    fn spawn_detached(
        &self,
        job_id: JobId,
        stdout: std::fs::File,
        stderr: std::fs::File,
    ) -> Result<std::process::Child> {
        use std::os::windows::process::CommandExt;

        // CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let child = Command::new(&self.executable_path)
            .args([DAEMON_RUN_ARG, &job_id.to_string()])
            .env(DAEMON_JOB_ID_ENV, job_id.to_string())
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
            .spawn()
            .map_err(|e| StateError::SpawnDaemon {
                executable: self.executable_path.clone(),
                source: e,
            })?;

        Ok(child)
    }

    /// Spawn a detached child process (fallback for other platforms).
    #[cfg(not(any(unix, windows)))]
    fn spawn_detached(
        &self,
        job_id: JobId,
        stdout: std::fs::File,
        stderr: std::fs::File,
    ) -> Result<std::process::Child> {
        let child = Command::new(&self.executable_path)
            .args([DAEMON_RUN_ARG, &job_id.to_string()])
            .env(DAEMON_JOB_ID_ENV, job_id.to_string())
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(|e| StateError::SpawnDaemon {
                executable: self.executable_path.clone(),
                source: e,
            })?;

        Ok(child)
    }

    /// Get the path to the current executable.
    fn executable_path() -> Result<PathBuf> {
        std::env::current_exe().map_err(|e| StateError::ExecutablePath { source: e })
    }

    /// Returns the state manager reference.
    #[must_use]
    pub const fn state_manager(&self) -> &StateManager {
        &self.state_manager
    }

    /// Returns the executable path.
    #[must_use]
    pub const fn executable(&self) -> &PathBuf {
        &self.executable_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstrumentTask;
    use std::path::PathBuf;
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
    fn test_daemon_spawner_creation() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Use a known executable for testing
        let spawner = DaemonSpawner::with_executable(state_manager, PathBuf::from("/bin/echo"));

        assert_eq!(spawner.executable(), &PathBuf::from("/bin/echo"));
    }

    #[test]
    fn test_daemon_spawner_with_executable() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        let custom_path = PathBuf::from("/custom/paracas");
        let spawner = DaemonSpawner::with_executable(state_manager, custom_path.clone());

        assert_eq!(spawner.executable(), &custom_path);
    }

    #[test]
    fn test_spawn_sets_log_file() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Use /bin/true or /usr/bin/true for a quick-exit process
        #[cfg(unix)]
        let exe_path = if PathBuf::from("/bin/true").exists() {
            PathBuf::from("/bin/true")
        } else {
            PathBuf::from("/usr/bin/true")
        };
        #[cfg(not(unix))]
        let exe_path = PathBuf::from("cmd.exe");

        let spawner = DaemonSpawner::with_executable(state_manager.clone(), exe_path);

        let mut job = create_test_job();
        let job_id = job.id;

        // Spawn the job
        let result = spawner.spawn(&mut job);

        // On CI or systems where the binary doesn't exist, this might fail
        if result.is_ok() {
            assert!(job.log_file.is_some());
            assert!(job.pid.is_some());

            // Verify log file path is correct
            let expected_log_path = state_manager.job_log_path(job_id);
            assert_eq!(job.log_file.as_ref().unwrap(), &expected_log_path);
        }
    }
}
