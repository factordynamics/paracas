# paracas-daemon

Background job management for the paracas tick data downloader.

## Features

- Job tracking with unique identifiers
- Persistent state storage
- Detached daemon process spawning
- Thread-safe progress tracking

## Types

- `JobId` - Unique identifier for download jobs
- `JobStatus` - Current status of a job (pending, running, completed, failed)
- `InstrumentTask` - Download task for a single instrument
- `DownloadJob` - Complete download job with multiple tasks
- `StateManager` - Persistent state storage and retrieval
- `DaemonSpawner` - Spawns detached daemon processes
- `DaemonProgress` - Thread-safe progress tracking

## Usage

```rust,ignore
use paracas_daemon::{StateManager, DownloadJob, JobId, JobStatus};

// Create a state manager
let manager = StateManager::new()?;

// Create a new job
let job = DownloadJob::new(tasks);
manager.save_job(&job)?;

// Retrieve job status
if let Some(job) = manager.get_job(&job.id)? {
    println!("Job status: {:?}", job.status());
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
