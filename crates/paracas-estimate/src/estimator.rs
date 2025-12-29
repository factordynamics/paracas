//! Download estimation logic.

use std::sync::OnceLock;
use std::time::Duration;

use paracas_types::{DateRange, Instrument};

use crate::data::EstimateDatabase;

/// Default download speed assumption in Mbps.
const DEFAULT_DOWNLOAD_SPEED_MBPS: f64 = 10.0;

/// Compression ratio (uncompressed / compressed).
const COMPRESSION_RATIO: f64 = 10.0;

/// Static estimator instance.
static ESTIMATOR: OnceLock<Estimator> = OnceLock::new();

/// Confidence level of the estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EstimateConfidence {
    /// High confidence - well-known instrument category with good historical data.
    High,
    /// Medium confidence - known category but less data or more variability.
    Medium,
    /// Low confidence - unknown category or limited historical data.
    Low,
}

impl EstimateConfidence {
    /// Returns the confidence as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

impl std::fmt::Display for EstimateConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Estimated download metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadEstimate {
    /// Total hours of data to download.
    pub total_hours: usize,
    /// Estimated compressed bytes to download.
    pub estimated_compressed_bytes: u64,
    /// Estimated uncompressed bytes (compressed * compression ratio).
    pub estimated_uncompressed_bytes: u64,
    /// Estimated output file size in bytes.
    pub estimated_output_bytes: u64,
    /// Estimated number of ticks.
    pub estimated_ticks: u64,
    /// Estimated download duration.
    pub estimated_duration: Duration,
    /// Confidence level of the estimate.
    pub confidence: EstimateConfidence,
}

impl DownloadEstimate {
    /// Creates a new download estimate.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        total_hours: usize,
        estimated_compressed_bytes: u64,
        estimated_uncompressed_bytes: u64,
        estimated_output_bytes: u64,
        estimated_ticks: u64,
        estimated_duration: Duration,
        confidence: EstimateConfidence,
    ) -> Self {
        Self {
            total_hours,
            estimated_compressed_bytes,
            estimated_uncompressed_bytes,
            estimated_output_bytes,
            estimated_ticks,
            estimated_duration,
            confidence,
        }
    }

    /// Creates an empty estimate (zero hours, zero bytes).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            total_hours: 0,
            estimated_compressed_bytes: 0,
            estimated_uncompressed_bytes: 0,
            estimated_output_bytes: 0,
            estimated_ticks: 0,
            estimated_duration: Duration::ZERO,
            confidence: EstimateConfidence::High,
        }
    }
}

/// Download size and time estimator.
#[derive(Debug, Clone)]
pub struct Estimator {
    /// Assumed download speed in Mbps.
    assumed_download_speed_mbps: f64,
}

impl Estimator {
    /// Creates a new estimator with the specified download speed.
    #[must_use]
    pub const fn new(assumed_download_speed_mbps: f64) -> Self {
        Self {
            assumed_download_speed_mbps,
        }
    }

    /// Returns the global estimator instance with default settings.
    #[must_use]
    pub fn global() -> &'static Self {
        ESTIMATOR.get_or_init(|| Self::new(DEFAULT_DOWNLOAD_SPEED_MBPS))
    }

    /// Returns the assumed download speed in Mbps.
    #[must_use]
    pub const fn download_speed_mbps(&self) -> f64 {
        self.assumed_download_speed_mbps
    }

    /// Estimates download metrics for a single instrument and date range.
    #[must_use]
    pub fn estimate_single(
        &self,
        instrument: &Instrument,
        date_range: &DateRange,
    ) -> DownloadEstimate {
        let total_hours = date_range.total_hours();
        let category = instrument.category().as_str();

        let db = EstimateDatabase::global();
        let (cat_estimate, confidence) = db.get(category).map_or_else(
            || {
                (
                    EstimateDatabase::default_estimate(),
                    EstimateConfidence::Low,
                )
            },
            |est| (est.clone(), EstimateConfidence::High),
        );

        self.calculate_estimate(total_hours, &cat_estimate, confidence)
    }

    /// Estimates download metrics for multiple instruments and date range.
    #[must_use]
    pub fn estimate_batch(
        &self,
        instruments: &[&Instrument],
        date_range: &DateRange,
    ) -> DownloadEstimate {
        if instruments.is_empty() {
            return DownloadEstimate::empty();
        }

        let total_hours = date_range.total_hours();
        let db = EstimateDatabase::global();

        let mut total_compressed_bytes: u64 = 0;
        let mut total_ticks: u64 = 0;
        let mut min_confidence = EstimateConfidence::High;

        for instrument in instruments {
            let category = instrument.category().as_str();
            let (cat_estimate, confidence) = db.get(category).map_or_else(
                || {
                    (
                        EstimateDatabase::default_estimate(),
                        EstimateConfidence::Low,
                    )
                },
                |est| (est.clone(), EstimateConfidence::High),
            );

            total_compressed_bytes +=
                cat_estimate.avg_compressed_bytes_per_hour * total_hours as u64;
            total_ticks += cat_estimate.avg_ticks_per_hour * total_hours as u64;

            // Use the lowest confidence among all instruments
            if matches!(confidence, EstimateConfidence::Low) {
                min_confidence = EstimateConfidence::Low;
            } else if matches!(confidence, EstimateConfidence::Medium)
                && !matches!(min_confidence, EstimateConfidence::Low)
            {
                min_confidence = EstimateConfidence::Medium;
            }
        }

        let estimated_uncompressed_bytes =
            (total_compressed_bytes as f64 * COMPRESSION_RATIO) as u64;
        let estimated_output_bytes = estimated_uncompressed_bytes;
        let estimated_duration = self.calculate_duration(total_compressed_bytes);

        DownloadEstimate::new(
            total_hours * instruments.len(),
            total_compressed_bytes,
            estimated_uncompressed_bytes,
            estimated_output_bytes,
            total_ticks,
            estimated_duration,
            min_confidence,
        )
    }

    /// Calculates estimate for a given number of hours and category.
    fn calculate_estimate(
        &self,
        total_hours: usize,
        cat_estimate: &crate::data::CategoryEstimate,
        confidence: EstimateConfidence,
    ) -> DownloadEstimate {
        let estimated_compressed_bytes =
            cat_estimate.avg_compressed_bytes_per_hour * total_hours as u64;
        let estimated_uncompressed_bytes =
            (estimated_compressed_bytes as f64 * COMPRESSION_RATIO) as u64;
        let estimated_output_bytes = estimated_uncompressed_bytes;
        let estimated_ticks = cat_estimate.avg_ticks_per_hour * total_hours as u64;
        let estimated_duration = self.calculate_duration(estimated_compressed_bytes);

        DownloadEstimate::new(
            total_hours,
            estimated_compressed_bytes,
            estimated_uncompressed_bytes,
            estimated_output_bytes,
            estimated_ticks,
            estimated_duration,
            confidence,
        )
    }

    /// Calculates download duration based on compressed bytes and speed.
    fn calculate_duration(&self, compressed_bytes: u64) -> Duration {
        // Convert Mbps to bytes per second
        let bytes_per_second = self.assumed_download_speed_mbps * 1_000_000.0 / 8.0;
        let seconds = compressed_bytes as f64 / bytes_per_second;
        Duration::from_secs_f64(seconds)
    }

    /// Formats an estimate as a human-readable summary.
    #[must_use]
    pub fn format_estimate(estimate: &DownloadEstimate) -> String {
        format!(
            "Download: {} compressed, {} uncompressed\n\
             Ticks: ~{}\n\
             Duration: {} (at assumed speed)\n\
             Confidence: {}",
            Self::format_bytes(estimate.estimated_compressed_bytes),
            Self::format_bytes(estimate.estimated_uncompressed_bytes),
            Self::format_ticks(estimate.estimated_ticks),
            Self::format_duration(estimate.estimated_duration),
            estimate.confidence,
        )
    }

    /// Formats bytes in human-readable form (e.g., "1.5 GB", "250 MB").
    #[must_use]
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;
        const TB: u64 = 1024 * GB;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Formats duration in human-readable form (e.g., "2h 30m", "45m").
    #[must_use]
    pub fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            if minutes > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}h", hours)
            }
        } else if minutes > 0 {
            if seconds > 0 && minutes < 10 {
                format!("{}m {}s", minutes, seconds)
            } else {
                format!("{}m", minutes)
            }
        } else {
            format!("{}s", seconds)
        }
    }

    /// Formats tick count in human-readable form.
    fn format_ticks(ticks: u64) -> String {
        if ticks >= 1_000_000_000 {
            format!("{:.2}B", ticks as f64 / 1_000_000_000.0)
        } else if ticks >= 1_000_000 {
            format!("{:.2}M", ticks as f64 / 1_000_000.0)
        } else if ticks >= 1_000 {
            format!("{:.2}K", ticks as f64 / 1_000.0)
        } else {
            format!("{}", ticks)
        }
    }
}

impl Default for Estimator {
    fn default() -> Self {
        Self::new(DEFAULT_DOWNLOAD_SPEED_MBPS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use paracas_types::Category;

    fn create_test_instrument(category: Category) -> Instrument {
        Instrument::new(
            "test",
            "Test Instrument",
            "Test description",
            category,
            100_000,
            None,
        )
    }

    #[test]
    fn test_estimate_single_forex() {
        let estimator = Estimator::default();
        let instrument = create_test_instrument(Category::Forex);
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        let estimate = estimator.estimate_single(&instrument, &date_range);

        assert_eq!(estimate.total_hours, 24);
        assert_eq!(estimate.estimated_compressed_bytes, 75000 * 24);
        assert_eq!(estimate.estimated_ticks, 5000 * 24);
        assert_eq!(estimate.confidence, EstimateConfidence::High);
    }

    #[test]
    fn test_estimate_batch() {
        let estimator = Estimator::default();
        let forex = create_test_instrument(Category::Forex);
        let crypto = create_test_instrument(Category::Crypto);
        let instruments: Vec<&Instrument> = vec![&forex, &crypto];
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date_range = DateRange::single_day(start);

        let estimate = estimator.estimate_batch(&instruments, &date_range);

        // 24 hours * 2 instruments
        assert_eq!(estimate.total_hours, 48);
        // forex: 75000 * 24 + crypto: 150000 * 24
        assert_eq!(estimate.estimated_compressed_bytes, (75000 + 150000) * 24);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(Estimator::format_bytes(500), "500 B");
        assert_eq!(Estimator::format_bytes(1536), "1.50 KB");
        assert_eq!(Estimator::format_bytes(1_572_864), "1.50 MB");
        assert_eq!(Estimator::format_bytes(1_610_612_736), "1.50 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(Estimator::format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(
            Estimator::format_duration(Duration::from_secs(90)),
            "1m 30s"
        );
        assert_eq!(Estimator::format_duration(Duration::from_secs(3600)), "1h");
        assert_eq!(
            Estimator::format_duration(Duration::from_secs(5400)),
            "1h 30m"
        );
    }

    #[test]
    fn test_empty_batch() {
        let estimator = Estimator::default();
        let instruments: Vec<&Instrument> = vec![];
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date_range = DateRange::single_day(start);

        let estimate = estimator.estimate_batch(&instruments, &date_range);

        assert_eq!(estimate.total_hours, 0);
        assert_eq!(estimate.estimated_compressed_bytes, 0);
    }

    #[test]
    fn test_estimate_confidence() {
        assert_eq!(EstimateConfidence::High.as_str(), "high");
        assert_eq!(EstimateConfidence::Medium.as_str(), "medium");
        assert_eq!(EstimateConfidence::Low.as_str(), "low");
    }
}
