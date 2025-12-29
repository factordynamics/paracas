//! Streaming tick download pipeline.

use chrono::{DateTime, Utc};
use futures::stream::{self, Stream, StreamExt};
use paracas_types::{DateRange, Instrument, ParacasError, Tick};

use crate::{DownloadClient, decompress_bi5, parse_ticks, url::tick_url};

/// A batch of ticks from a single hour.
#[derive(Debug, Clone)]
pub struct TickBatch {
    /// The hour start timestamp.
    pub hour: DateTime<Utc>,
    /// The ticks in this batch.
    pub ticks: Vec<Tick>,
    /// Whether this batch had an error that was skipped.
    pub had_error: bool,
}

impl TickBatch {
    /// Creates a new tick batch.
    #[must_use]
    pub const fn new(hour: DateTime<Utc>, ticks: Vec<Tick>) -> Self {
        Self {
            hour,
            ticks,
            had_error: false,
        }
    }

    /// Creates a new tick batch that represents a skipped error.
    #[must_use]
    pub const fn skipped_error(hour: DateTime<Utc>) -> Self {
        Self {
            hour,
            ticks: Vec::new(),
            had_error: true,
        }
    }

    /// Returns true if the batch is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }

    /// Returns the number of ticks in the batch.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.ticks.len()
    }

    /// Returns true if this batch had an error that was skipped.
    #[must_use]
    pub const fn had_error(&self) -> bool {
        self.had_error
    }
}

/// Creates an async stream of tick batches for the given instrument and date range.
///
/// This function downloads, decompresses, and parses tick data concurrently
/// using the configured number of parallel connections.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for downloads
/// * `instrument` - The instrument to download data for
/// * `range` - The date range to download
///
/// # Returns
///
/// An async stream of tick batches, one per hour.
pub fn tick_stream<'a>(
    client: &'a DownloadClient,
    instrument: &'a Instrument,
    range: DateRange,
) -> impl Stream<Item = Result<TickBatch, ParacasError>> + 'a {
    let decimal_factor = instrument.decimal_factor_f64();
    let instrument_id = instrument.id().to_string();
    let concurrency = client.config().concurrency;

    stream::iter(range.hours())
        .map(move |hour| {
            let url = tick_url(&instrument_id, hour);
            let client = client.clone();
            async move {
                let result = client.download(&url).await;
                // Process immediately after download (decompression is offloaded to spawn_blocking)
                process_download_result(hour, result, decimal_factor).await
            }
        })
        .buffer_unordered(concurrency)
}

/// Processes a download result into a tick batch.
///
/// Decompression is offloaded to a blocking thread pool to avoid blocking
/// the async executor.
async fn process_download_result(
    hour: DateTime<Utc>,
    result: Result<Option<bytes::Bytes>, crate::DownloadError>,
    decimal_factor: f64,
) -> Result<TickBatch, ParacasError> {
    match result {
        Ok(Some(compressed)) => {
            // Offload CPU-intensive LZMA decompression to blocking thread pool
            let decompressed = tokio::task::spawn_blocking(move || decompress_bi5(&compressed))
                .await
                .map_err(|e| ParacasError::Decompress(format!("spawn_blocking failed: {e}")))?
                .map_err(|e| ParacasError::Decompress(e.to_string()))?;

            let ticks: Vec<Tick> = parse_ticks(&decompressed)
                .map_err(|e| ParacasError::Parse(e.to_string()))?
                .map(|raw| raw.normalize(hour, decimal_factor))
                .collect();

            Ok(TickBatch::new(hour, ticks))
        }
        Ok(None) => {
            // No data for this hour
            Ok(TickBatch::new(hour, Vec::new()))
        }
        Err(e) => Err(ParacasError::Http(e.to_string())),
    }
}

/// Creates a resilient async stream that skips failed hours instead of failing entirely.
///
/// This is useful for long-running downloads where occasional server errors
/// should not abort the entire operation.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for downloads
/// * `instrument` - The instrument to download data for
/// * `range` - The date range to download
///
/// # Returns
///
/// An async stream of tick batches. Failed hours are returned as empty batches
/// with `had_error` set to true.
pub fn tick_stream_resilient<'a>(
    client: &'a DownloadClient,
    instrument: &'a Instrument,
    range: DateRange,
) -> impl Stream<Item = TickBatch> + 'a {
    let decimal_factor = instrument.decimal_factor_f64();
    let instrument_id = instrument.id().to_string();
    let concurrency = client.config().concurrency;

    stream::iter(range.hours())
        .map(move |hour| {
            let url = tick_url(&instrument_id, hour);
            let client = client.clone();
            async move {
                let result = client.download(&url).await;
                // Process immediately after download (decompression is offloaded to spawn_blocking)
                process_download_result_resilient(hour, result, decimal_factor).await
            }
        })
        .buffer_unordered(concurrency)
}

/// Processes a download result into a tick batch, skipping errors.
///
/// Decompression is offloaded to a blocking thread pool to avoid blocking
/// the async executor.
async fn process_download_result_resilient(
    hour: DateTime<Utc>,
    result: Result<Option<bytes::Bytes>, crate::DownloadError>,
    decimal_factor: f64,
) -> TickBatch {
    match result {
        Ok(Some(compressed)) => {
            // Offload CPU-intensive LZMA decompression to blocking thread pool
            let decompress_result =
                tokio::task::spawn_blocking(move || decompress_bi5(&compressed)).await;

            match decompress_result {
                Ok(Ok(decompressed)) => parse_ticks(&decompressed).map_or_else(
                    |_| TickBatch::skipped_error(hour),
                    |raw_ticks| {
                        let ticks: Vec<Tick> = raw_ticks
                            .map(|raw| raw.normalize(hour, decimal_factor))
                            .collect();
                        TickBatch::new(hour, ticks)
                    },
                ),
                _ => {
                    // Decompression error or spawn_blocking failed - return empty batch with error flag
                    TickBatch::skipped_error(hour)
                }
            }
        }
        Ok(None) => {
            // No data for this hour
            TickBatch::new(hour, Vec::new())
        }
        Err(_) => {
            // HTTP error - return empty batch with error flag
            TickBatch::skipped_error(hour)
        }
    }
}

/// Flattens a tick batch stream into individual ticks.
///
/// This is useful when you want to process ticks one at a time rather than
/// in batches.
pub fn flatten_ticks(
    batch_stream: impl Stream<Item = Result<TickBatch, ParacasError>>,
) -> impl Stream<Item = Result<Tick, ParacasError>> {
    batch_stream.flat_map(|result| match result {
        Ok(batch) => stream::iter(batch.ticks.into_iter().map(Ok)).left_stream(),
        Err(e) => stream::once(async move { Err(e) }).right_stream(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_batch_new() {
        let hour = Utc::now();
        let batch = TickBatch::new(hour, vec![]);
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        assert!(!batch.had_error());
    }

    #[test]
    fn test_tick_batch_skipped_error() {
        let hour = Utc::now();
        let batch = TickBatch::skipped_error(hour);
        assert!(batch.is_empty());
        assert!(batch.had_error());
    }
}
