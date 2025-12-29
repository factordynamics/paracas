//! HTTP client for downloading bi5 files.

use bytes::Bytes;
use reqwest::Client;
use std::time::Duration;
use thiserror::Error;

/// Configuration for the download client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Maximum concurrent downloads.
    pub concurrency: usize,
    /// Request timeout.
    pub timeout: Duration,
    /// Maximum retry attempts for failed requests.
    pub max_retries: u32,
    /// Base delay for exponential backoff (in milliseconds).
    pub base_delay_ms: u64,
    /// Maximum delay between retries (in milliseconds).
    pub max_delay_ms: u64,
    /// User agent string.
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            concurrency: 10, // Lower concurrency to avoid overwhelming the server
            timeout: Duration::from_secs(60),
            max_retries: 10,      // More retries for transient failures
            base_delay_ms: 500,   // Start with 500ms delay
            max_delay_ms: 30_000, // Max 30 seconds between retries
            user_agent: format!("paracas/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Errors that can occur during downloads.
#[derive(Error, Debug)]
pub enum DownloadError {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Request timed out.
    #[error("Request timed out after {0} attempts")]
    Timeout(u32),

    /// Server returned an error status.
    #[error("Server error: {status}")]
    ServerError {
        /// HTTP status code.
        status: u16,
    },
}

/// HTTP client with connection pooling and retry logic.
#[derive(Debug, Clone)]
pub struct DownloadClient {
    client: Client,
    config: ClientConfig,
}

impl DownloadClient {
    /// Creates a new download client with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: ClientConfig) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            // Connection pooling - maintain up to concurrency idle connections per host
            .pool_max_idle_per_host(config.concurrency)
            // Keep connections alive for reuse (Dukascopy supports persistent connections)
            .pool_idle_timeout(Duration::from_secs(90))
            // Disable Nagle's algorithm for lower latency
            .tcp_nodelay(true)
            // Keep TCP connections alive
            .tcp_keepalive(Duration::from_secs(60))
            // Request timeout
            .timeout(config.timeout)
            // Connection timeout (separate from request timeout)
            .connect_timeout(Duration::from_secs(10))
            .user_agent(&config.user_agent)
            .gzip(true)
            .build()?;
        Ok(Self { client, config })
    }

    /// Creates a client with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn with_defaults() -> Result<Self, reqwest::Error> {
        Self::new(ClientConfig::default())
    }

    /// Returns the client configuration.
    #[must_use]
    pub const fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Downloads a single bi5 file, returning the compressed bytes.
    ///
    /// Returns `Ok(None)` if the file does not exist (404).
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails after all retries.
    pub async fn download(&self, url: &str) -> Result<Option<Bytes>, DownloadError> {
        let mut attempts = 0;

        loop {
            match self.client.get(url).send().await {
                Ok(response) => {
                    if response.status() == reqwest::StatusCode::NOT_FOUND {
                        return Ok(None); // No data for this hour
                    }

                    // Retry on server errors (5xx) and rate limiting (429)
                    if response.status().is_server_error()
                        || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                    {
                        if attempts < self.config.max_retries {
                            attempts += 1;
                            let delay = self.calculate_backoff_delay(attempts);
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                        return Err(DownloadError::ServerError {
                            status: response.status().as_u16(),
                        });
                    }

                    response.error_for_status_ref()?;
                    return Ok(Some(response.bytes().await?));
                }
                Err(e) if self.is_retryable_error(&e) && attempts < self.config.max_retries => {
                    attempts += 1;
                    let delay = self.calculate_backoff_delay(attempts);
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Calculates the backoff delay with exponential backoff and jitter.
    fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff: base_delay * 2^attempt
        let exp_delay = self
            .config
            .base_delay_ms
            .saturating_mul(1u64 << attempt.min(10));

        // Cap at max delay
        let capped_delay = exp_delay.min(self.config.max_delay_ms);

        // Add jitter (±25%)
        let jitter_range = capped_delay / 4;
        let jitter = if jitter_range > 0 {
            // Simple deterministic jitter based on attempt number
            // This avoids needing a random number generator
            let jitter_offset = (attempt as u64 * 17) % (jitter_range * 2);
            jitter_offset.saturating_sub(jitter_range)
        } else {
            0
        };

        let final_delay = (capped_delay as i64 + jitter as i64).max(100) as u64;
        Duration::from_millis(final_delay)
    }

    /// Determines if an error is retryable.
    fn is_retryable_error(&self, error: &reqwest::Error) -> bool {
        // Don't retry builder errors (configuration issues)
        if error.is_builder() {
            return false;
        }

        // Retry on timeouts, connection errors, and request errors
        error.is_timeout() || error.is_connect() || error.is_request()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.base_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 30_000);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = DownloadClient::with_defaults();
        assert!(client.is_ok());
    }

    #[test]
    fn test_backoff_delay_calculation() {
        let client = DownloadClient::with_defaults().unwrap();

        // First attempt: base_delay * 2 = 1000ms (plus jitter)
        let delay1 = client.calculate_backoff_delay(1);
        assert!(delay1.as_millis() >= 750 && delay1.as_millis() <= 1250);

        // Second attempt: base_delay * 4 = 2000ms (plus jitter)
        let delay2 = client.calculate_backoff_delay(2);
        assert!(delay2.as_millis() >= 1500 && delay2.as_millis() <= 2500);

        // High attempt should be capped at max_delay
        let delay_high = client.calculate_backoff_delay(20);
        assert!(delay_high.as_millis() <= 37500); // max_delay + 25% jitter
    }
}
