//! Download job definitions and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for a download job.
pub type JobId = Uuid;

/// Status of a download job or task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is queued but not yet started.
    #[default]
    Pending,
    /// Job is currently running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed with an error.
    Failed,
    /// Job was cancelled by the user.
    Cancelled,
}

impl JobStatus {
    /// Returns true if the job is in a terminal state.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Returns the status as a string identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A download task for a single instrument within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentTask {
    /// The instrument identifier (e.g., "EURUSD").
    pub instrument_id: String,
    /// Start date for the download (inclusive).
    pub start_date: String,
    /// End date for the download (inclusive).
    pub end_date: String,
    /// Output file path for this instrument's data.
    pub output_path: PathBuf,
    /// Output format (e.g., "csv", "json", "parquet").
    pub format: String,
    /// Timeframe for aggregation (e.g., "tick", "m1", "h1").
    pub timeframe: String,
    /// Current status of this task.
    pub status: JobStatus,
    /// Number of hours completed for this task.
    pub hours_completed: u32,
    /// Total number of hours to download.
    pub hours_total: u32,
    /// Number of ticks downloaded so far.
    pub ticks_downloaded: u64,
    /// Number of bytes written to output file.
    pub bytes_written: u64,
    /// Error message if the task failed.
    pub error_message: Option<String>,
}

impl InstrumentTask {
    /// Creates a new instrument task.
    #[must_use]
    pub const fn new(
        instrument_id: String,
        start_date: String,
        end_date: String,
        output_path: PathBuf,
        format: String,
        timeframe: String,
        hours_total: u32,
    ) -> Self {
        Self {
            instrument_id,
            start_date,
            end_date,
            output_path,
            format,
            timeframe,
            status: JobStatus::Pending,
            hours_completed: 0,
            hours_total,
            ticks_downloaded: 0,
            bytes_written: 0,
            error_message: None,
        }
    }

    /// Returns the progress percentage for this task.
    #[must_use]
    pub fn progress_percent(&self) -> f64 {
        if self.hours_total == 0 {
            return 0.0;
        }
        (self.hours_completed as f64 / self.hours_total as f64) * 100.0
    }
}

/// A complete download job containing one or more instrument tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    /// Unique identifier for this job.
    pub id: JobId,
    /// Timestamp when the job was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the job started running.
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when the job completed (success, failure, or cancellation).
    pub completed_at: Option<DateTime<Utc>>,
    /// Current status of the job.
    pub status: JobStatus,
    /// List of instrument download tasks.
    pub tasks: Vec<InstrumentTask>,
    /// Number of concurrent downloads.
    pub concurrency: usize,
    /// Process ID of the daemon running this job.
    pub pid: Option<u32>,
    /// Path to the log file for this job.
    pub log_file: Option<PathBuf>,
}

impl DownloadJob {
    /// Creates a new download job with the given tasks.
    #[must_use]
    pub fn new(tasks: Vec<InstrumentTask>, concurrency: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            status: JobStatus::Pending,
            tasks,
            concurrency,
            pid: None,
            log_file: None,
        }
    }

    /// Returns the overall progress percentage across all tasks.
    #[must_use]
    pub fn progress_percent(&self) -> f64 {
        let total_hours: u32 = self.tasks.iter().map(|t| t.hours_total).sum();
        let completed_hours: u32 = self.tasks.iter().map(|t| t.hours_completed).sum();

        if total_hours == 0 {
            return 0.0;
        }
        (completed_hours as f64 / total_hours as f64) * 100.0
    }

    /// Returns true if the job is in a terminal state.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.status.is_finished()
    }

    /// Marks the job as started with the current timestamp and process ID.
    pub fn mark_started(&mut self, pid: u32) {
        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
        self.pid = Some(pid);
    }

    /// Marks the job as completed successfully.
    pub fn mark_completed(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Marks the job as failed with an optional error message.
    pub fn mark_failed(&mut self, error: Option<String>) {
        self.status = JobStatus::Failed;
        self.completed_at = Some(Utc::now());

        // If an error message is provided, set it on any running tasks
        if let Some(ref msg) = error {
            for task in &mut self.tasks {
                if task.status == JobStatus::Running {
                    task.status = JobStatus::Failed;
                    task.error_message = Some(msg.clone());
                }
            }
        }
    }

    /// Marks the job as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = JobStatus::Cancelled;
        self.completed_at = Some(Utc::now());

        // Cancel any pending or running tasks
        for task in &mut self.tasks {
            if !task.status.is_finished() {
                task.status = JobStatus::Cancelled;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_is_finished() {
        assert!(!JobStatus::Pending.is_finished());
        assert!(!JobStatus::Running.is_finished());
        assert!(JobStatus::Completed.is_finished());
        assert!(JobStatus::Failed.is_finished());
        assert!(JobStatus::Cancelled.is_finished());
    }

    #[test]
    fn test_instrument_task_progress() {
        let mut task = InstrumentTask::new(
            "EURUSD".to_string(),
            "2024-01-01".to_string(),
            "2024-01-02".to_string(),
            PathBuf::from("/tmp/eurusd.csv"),
            "csv".to_string(),
            "tick".to_string(),
            48,
        );

        assert_eq!(task.progress_percent(), 0.0);

        task.hours_completed = 24;
        assert!((task.progress_percent() - 50.0).abs() < 0.001);

        task.hours_completed = 48;
        assert!((task.progress_percent() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_download_job_progress() {
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

        let mut job = DownloadJob::new(tasks, 4);
        assert_eq!(job.progress_percent(), 0.0);

        job.tasks[0].hours_completed = 48;
        assert!((job.progress_percent() - 50.0).abs() < 0.001);

        job.tasks[1].hours_completed = 48;
        assert!((job.progress_percent() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_download_job_lifecycle() {
        let tasks = vec![InstrumentTask::new(
            "EURUSD".to_string(),
            "2024-01-01".to_string(),
            "2024-01-02".to_string(),
            PathBuf::from("/tmp/eurusd.csv"),
            "csv".to_string(),
            "tick".to_string(),
            48,
        )];

        let mut job = DownloadJob::new(tasks, 4);
        assert_eq!(job.status, JobStatus::Pending);
        assert!(job.started_at.is_none());
        assert!(!job.is_finished());

        job.mark_started(12345);
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.started_at.is_some());
        assert_eq!(job.pid, Some(12345));
        assert!(!job.is_finished());

        job.mark_completed();
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.completed_at.is_some());
        assert!(job.is_finished());
    }
}
