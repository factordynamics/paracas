//! Progress tracking for daemon downloads.
//!
//! This module provides thread-safe progress tracking for daemon jobs,
//! including periodic checkpointing to disk for crash recovery.

use crate::{DownloadJob, JobStatus, StateError, StateManager};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Thread-safe progress tracker for daemon jobs.
///
/// The `DaemonProgress` struct provides a way to track download progress
/// from multiple concurrent tasks while ensuring periodic checkpoints
/// are saved to disk for crash recovery.
#[derive(Debug)]
pub struct DaemonProgress {
    /// State manager for persistence.
    state_manager: StateManager,
    /// The job being tracked (protected by RwLock for concurrent access).
    job: Arc<RwLock<DownloadJob>>,
    /// Minimum interval between saves.
    save_interval: Duration,
    /// Last time state was saved to disk.
    last_save: std::sync::Mutex<Instant>,
}

impl DaemonProgress {
    /// Default save interval for checkpointing (10 seconds).
    pub const DEFAULT_SAVE_INTERVAL: Duration = Duration::from_secs(10);

    /// Create a new progress tracker.
    ///
    /// The tracker will periodically save checkpoints to disk at the
    /// default interval of 10 seconds.
    #[must_use]
    pub fn new(state_manager: StateManager, job: DownloadJob) -> Self {
        Self {
            state_manager,
            job: Arc::new(RwLock::new(job)),
            save_interval: Self::DEFAULT_SAVE_INTERVAL,
            last_save: std::sync::Mutex::new(Instant::now()),
        }
    }

    /// Create a new progress tracker with a custom save interval.
    #[must_use]
    pub fn with_save_interval(
        state_manager: StateManager,
        job: DownloadJob,
        save_interval: Duration,
    ) -> Self {
        Self {
            state_manager,
            job: Arc::new(RwLock::new(job)),
            save_interval,
            last_save: std::sync::Mutex::new(Instant::now()),
        }
    }

    /// Update progress for a specific task.
    ///
    /// This updates the hours completed and ticks downloaded for the task
    /// at the given index. If enough time has passed since the last save,
    /// the state will be checkpointed to disk.
    ///
    /// # Arguments
    ///
    /// * `task_idx` - Index of the task to update
    /// * `hours` - Number of hours completed
    /// * `ticks` - Number of ticks downloaded
    pub async fn update_task_progress(&self, task_idx: usize, hours: u64, ticks: u64) {
        {
            let mut job = self.job.write().await;
            if let Some(task) = job.tasks.get_mut(task_idx) {
                task.hours_completed = hours as u32;
                task.ticks_downloaded = ticks;
                if task.status == JobStatus::Pending {
                    task.status = JobStatus::Running;
                }
            }
        }

        // Check if we should save
        self.maybe_save_checkpoint().await;
    }

    /// Mark a task as completed.
    ///
    /// This updates the task status to `Completed` and records the
    /// final byte count.
    ///
    /// # Arguments
    ///
    /// * `task_idx` - Index of the task to mark as completed
    /// * `bytes` - Total bytes written for this task
    pub async fn mark_task_completed(&self, task_idx: usize, bytes: u64) {
        {
            let mut job = self.job.write().await;
            if let Some(task) = job.tasks.get_mut(task_idx) {
                task.status = JobStatus::Completed;
                task.bytes_written = bytes;
                task.hours_completed = task.hours_total;
            }
        }

        // Always save on task completion
        let _ = self.save_checkpoint().await;
    }

    /// Mark a task as failed.
    ///
    /// This updates the task status to `Failed` and records the error message.
    ///
    /// # Arguments
    ///
    /// * `task_idx` - Index of the task to mark as failed
    /// * `error` - Error message describing the failure
    pub async fn mark_task_failed(&self, task_idx: usize, error: &str) {
        {
            let mut job = self.job.write().await;
            if let Some(task) = job.tasks.get_mut(task_idx) {
                task.status = JobStatus::Failed;
                task.error_message = Some(error.to_string());
            }
        }

        // Always save on task failure
        let _ = self.save_checkpoint().await;
    }

    /// Mark a task as running.
    ///
    /// This updates the task status to `Running`.
    ///
    /// # Arguments
    ///
    /// * `task_idx` - Index of the task to mark as running
    pub async fn mark_task_running(&self, task_idx: usize) {
        {
            let mut job = self.job.write().await;
            if let Some(task) = job.tasks.get_mut(task_idx) {
                task.status = JobStatus::Running;
            }
        }

        // Save when task starts
        let _ = self.save_checkpoint().await;
    }

    /// Mark the entire job as completed.
    ///
    /// Call this when all tasks have finished successfully.
    pub async fn mark_job_completed(&self) {
        {
            let mut job = self.job.write().await;
            job.mark_completed();
        }

        // Always save on job completion
        let _ = self.save_checkpoint().await;
    }

    /// Mark the entire job as failed.
    ///
    /// Call this when the job fails due to a critical error.
    ///
    /// # Arguments
    ///
    /// * `error` - Error message describing the failure
    pub async fn mark_job_failed(&self, error: &str) {
        {
            let mut job = self.job.write().await;
            job.mark_failed(Some(error.to_string()));
        }

        // Always save on job failure
        let _ = self.save_checkpoint().await;
    }

    /// Save current progress to disk (called periodically).
    ///
    /// This forces a checkpoint save regardless of the save interval.
    ///
    /// # Errors
    ///
    /// Returns an error if the state cannot be saved to disk.
    pub async fn save_checkpoint(&self) -> Result<(), StateError> {
        let job = self.job.read().await;
        self.state_manager.save_job(&job)?;

        // Update last save time
        if let Ok(mut last_save) = self.last_save.lock() {
            *last_save = Instant::now();
        }

        Ok(())
    }

    /// Check if enough time has passed and save if needed.
    async fn maybe_save_checkpoint(&self) {
        let should_save = self
            .last_save
            .lock()
            .map_or(true, |last_save| last_save.elapsed() >= self.save_interval);

        if should_save {
            let _ = self.save_checkpoint().await;
        }
    }

    /// Get current job state.
    ///
    /// Returns a clone of the current job state.
    pub async fn job(&self) -> DownloadJob {
        self.job.read().await.clone()
    }

    /// Get the number of completed tasks.
    pub async fn completed_tasks(&self) -> usize {
        let job = self.job.read().await;
        job.tasks
            .iter()
            .filter(|t| t.status == JobStatus::Completed)
            .count()
    }

    /// Get the number of failed tasks.
    pub async fn failed_tasks(&self) -> usize {
        let job = self.job.read().await;
        job.tasks
            .iter()
            .filter(|t| t.status == JobStatus::Failed)
            .count()
    }

    /// Get the total number of tasks.
    pub async fn total_tasks(&self) -> usize {
        let job = self.job.read().await;
        job.tasks.len()
    }

    /// Get the current progress percentage.
    pub async fn progress_percent(&self) -> f64 {
        let job = self.job.read().await;
        job.progress_percent()
    }

    /// Check if all tasks are finished.
    pub async fn all_tasks_finished(&self) -> bool {
        let job = self.job.read().await;
        job.tasks.iter().all(|t| t.status.is_finished())
    }

    /// Returns a reference to the state manager.
    #[must_use]
    pub const fn state_manager(&self) -> &StateManager {
        &self.state_manager
    }
}

impl Clone for DaemonProgress {
    fn clone(&self) -> Self {
        Self {
            state_manager: self.state_manager.clone(),
            job: Arc::clone(&self.job),
            save_interval: self.save_interval,
            last_save: std::sync::Mutex::new(self.last_save.lock().map_or(Instant::now(), |g| *g)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstrumentTask;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_job() -> DownloadJob {
        let tasks = vec![
            InstrumentTask::new(
                "EURUSD".to_string(),
                "2024-01-01".to_string(),
                "2024-01-02".to_string(),
                PathBuf::from("/tmp/eurusd.csv"),
                "csv".to_string(),
                "tick".to_string(),
                48,
            ),
            InstrumentTask::new(
                "GBPUSD".to_string(),
                "2024-01-01".to_string(),
                "2024-01-02".to_string(),
                PathBuf::from("/tmp/gbpusd.csv"),
                "csv".to_string(),
                "tick".to_string(),
                48,
            ),
        ];
        DownloadJob::new(tasks, 4)
    }

    #[tokio::test]
    async fn test_progress_tracker_creation() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);
        let current = progress.job().await;

        assert_eq!(current.tasks.len(), 2);
        assert_eq!(current.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_task_progress() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);

        progress.update_task_progress(0, 24, 1_000_000).await;

        let current = progress.job().await;
        assert_eq!(current.tasks[0].hours_completed, 24);
        assert_eq!(current.tasks[0].ticks_downloaded, 1_000_000);
        assert_eq!(current.tasks[0].status, JobStatus::Running);
    }

    #[tokio::test]
    async fn test_mark_task_completed() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();
        let job_id = job.id;

        let progress = DaemonProgress::new(state_manager.clone(), job);

        progress.mark_task_completed(0, 1024 * 1024).await;

        let current = progress.job().await;
        assert_eq!(current.tasks[0].status, JobStatus::Completed);
        assert_eq!(current.tasks[0].bytes_written, 1024 * 1024);
        assert_eq!(current.tasks[0].hours_completed, 48); // Should be set to total

        // Verify saved to disk
        let loaded = state_manager.load_job(job_id).unwrap();
        assert_eq!(loaded.tasks[0].status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn test_mark_task_failed() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();
        let job_id = job.id;

        let progress = DaemonProgress::new(state_manager.clone(), job);

        progress.mark_task_failed(0, "Connection timeout").await;

        let current = progress.job().await;
        assert_eq!(current.tasks[0].status, JobStatus::Failed);
        assert_eq!(
            current.tasks[0].error_message,
            Some("Connection timeout".to_string())
        );

        // Verify saved to disk
        let loaded = state_manager.load_job(job_id).unwrap();
        assert_eq!(loaded.tasks[0].status, JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_mark_job_completed() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);

        progress.mark_job_completed().await;

        let current = progress.job().await;
        assert_eq!(current.status, JobStatus::Completed);
        assert!(current.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_mark_job_failed() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);

        progress.mark_job_failed("Fatal error").await;

        let current = progress.job().await;
        assert_eq!(current.status, JobStatus::Failed);
        assert!(current.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_completed_tasks_count() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);

        assert_eq!(progress.completed_tasks().await, 0);

        progress.mark_task_completed(0, 1024).await;
        assert_eq!(progress.completed_tasks().await, 1);

        progress.mark_task_completed(1, 2048).await;
        assert_eq!(progress.completed_tasks().await, 2);
    }

    #[tokio::test]
    async fn test_all_tasks_finished() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);

        assert!(!progress.all_tasks_finished().await);

        progress.mark_task_completed(0, 1024).await;
        assert!(!progress.all_tasks_finished().await);

        progress.mark_task_failed(1, "error").await;
        assert!(progress.all_tasks_finished().await);
    }

    #[tokio::test]
    async fn test_progress_percent() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);

        // Initially 0%
        assert!((progress.progress_percent().await - 0.0).abs() < 0.001);

        // Update first task to 50%
        progress.update_task_progress(0, 24, 100).await;
        // 24/96 total hours = 25%
        assert!((progress.progress_percent().await - 25.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_custom_save_interval() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress =
            DaemonProgress::with_save_interval(state_manager, job, Duration::from_secs(1));

        assert_eq!(progress.save_interval, Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_progress_clone() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let job = create_test_job();

        let progress = DaemonProgress::new(state_manager, job);
        let cloned = progress.clone();

        // Both should share the same job state
        progress.update_task_progress(0, 10, 100).await;

        let original_job = progress.job().await;
        let cloned_job = cloned.job().await;

        assert_eq!(
            original_job.tasks[0].hours_completed,
            cloned_job.tasks[0].hours_completed
        );
    }
}
