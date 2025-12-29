//! OHLCV (candlestick) data structure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OHLCV bar (candlestick) data.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ohlcv {
    /// Bar open time (start of the period).
    pub timestamp: DateTime<Utc>,
    /// Opening price (first tick's mid price).
    pub open: f64,
    /// Highest price during the period.
    pub high: f64,
    /// Lowest price during the period.
    pub low: f64,
    /// Closing price (last tick's mid price).
    pub close: f64,
    /// Total volume (sum of ask + bid volumes).
    pub volume: f64,
    /// Number of ticks in the bar.
    pub tick_count: u32,
}

impl Ohlcv {
    /// Creates a new OHLCV bar.
    #[must_use]
    pub const fn new(
        timestamp: DateTime<Utc>,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        tick_count: u32,
    ) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            tick_count,
        }
    }

    /// Returns the price range (high - low).
    #[must_use]
    pub fn range(&self) -> f64 {
        self.high - self.low
    }

    /// Returns the body size (|close - open|).
    #[must_use]
    pub fn body(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Returns true if this is a bullish (green) bar.
    #[must_use]
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Returns true if this is a bearish (red) bar.
    #[must_use]
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Returns the typical price ((high + low + close) / 3).
    #[must_use]
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }

    /// Returns the weighted close ((high + low + 2*close) / 4).
    #[must_use]
    pub fn weighted_close(&self) -> f64 {
        (self.high + self.low + 2.0 * self.close) / 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_test_bar() -> Ohlcv {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        Ohlcv::new(timestamp, 1.1000, 1.1050, 1.0980, 1.1020, 1000.0, 500)
    }

    #[test]
    fn test_range() {
        let bar = create_test_bar();
        assert!((bar.range() - 0.0070).abs() < 1e-10);
    }

    #[test]
    fn test_body() {
        let bar = create_test_bar();
        assert!((bar.body() - 0.0020).abs() < 1e-10);
    }

    #[test]
    fn test_bullish() {
        let bar = create_test_bar();
        assert!(bar.is_bullish());
        assert!(!bar.is_bearish());
    }

    #[test]
    fn test_bearish() {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let bar = Ohlcv::new(timestamp, 1.1020, 1.1050, 1.0980, 1.1000, 1000.0, 500);
        assert!(!bar.is_bullish());
        assert!(bar.is_bearish());
    }

    #[test]
    fn test_typical_price() {
        let bar = create_test_bar();
        let expected = (1.1050 + 1.0980 + 1.1020) / 3.0;
        assert!((bar.typical_price() - expected).abs() < 1e-10);
    }
}
