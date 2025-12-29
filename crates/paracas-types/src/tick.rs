//! Tick data representation.

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

/// A single tick representing a price update.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    /// Timestamp of the tick (UTC).
    pub timestamp: DateTime<Utc>,
    /// Ask (offer) price.
    pub ask: f64,
    /// Bid price.
    pub bid: f64,
    /// Volume available at the ask price.
    pub ask_volume: f32,
    /// Volume available at the bid price.
    pub bid_volume: f32,
}

impl Tick {
    /// Creates a new tick.
    #[must_use]
    pub const fn new(
        timestamp: DateTime<Utc>,
        ask: f64,
        bid: f64,
        ask_volume: f32,
        bid_volume: f32,
    ) -> Self {
        Self {
            timestamp,
            ask,
            bid,
            ask_volume,
            bid_volume,
        }
    }

    /// Returns the mid price (average of ask and bid).
    #[must_use]
    pub fn mid(&self) -> f64 {
        (self.ask + self.bid) / 2.0
    }

    /// Returns the spread (ask - bid).
    #[must_use]
    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }

    /// Returns the total volume (ask + bid volume).
    #[must_use]
    pub fn total_volume(&self) -> f32 {
        self.ask_volume + self.bid_volume
    }
}

/// Raw tick as read from bi5 file (before price normalization).
///
/// The bi5 format stores ticks as 20 bytes in big-endian order:
/// - `u32`: milliseconds offset from hour start
/// - `u32`: ask price (raw, needs division by decimal factor)
/// - `u32`: bid price (raw, needs division by decimal factor)
/// - `f32`: ask volume
/// - `f32`: bid volume
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawTick {
    /// Milliseconds offset from the hour start.
    pub ms_offset: u32,
    /// Raw ask price (needs division by decimal factor).
    pub ask_raw: u32,
    /// Raw bid price (needs division by decimal factor).
    pub bid_raw: u32,
    /// Ask volume.
    pub ask_volume: f32,
    /// Bid volume.
    pub bid_volume: f32,
}

impl RawTick {
    /// Size in bytes of a raw tick record.
    pub const SIZE: usize = 20;

    /// Creates a new raw tick.
    #[must_use]
    pub const fn new(
        ms_offset: u32,
        ask_raw: u32,
        bid_raw: u32,
        ask_volume: f32,
        bid_volume: f32,
    ) -> Self {
        Self {
            ms_offset,
            ask_raw,
            bid_raw,
            ask_volume,
            bid_volume,
        }
    }

    /// Normalizes the raw tick using the instrument's decimal factor.
    ///
    /// The decimal factor converts the raw integer prices to floating-point
    /// prices. For example, EUR/USD has a decimal factor of 100,000, so a
    /// raw price of 112345 becomes 1.12345.
    #[must_use]
    pub fn normalize(self, hour_start: DateTime<Utc>, decimal_factor: f64) -> Tick {
        let timestamp = hour_start + TimeDelta::milliseconds(i64::from(self.ms_offset));
        Tick {
            timestamp,
            ask: f64::from(self.ask_raw) / decimal_factor,
            bid: f64::from(self.bid_raw) / decimal_factor,
            ask_volume: self.ask_volume,
            bid_volume: self.bid_volume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_tick_mid_price() {
        let tick = Tick::new(Utc::now(), 1.1001, 1.1000, 100.0, 200.0);
        assert!((tick.mid() - 1.10005).abs() < 1e-10);
    }

    #[test]
    fn test_tick_spread() {
        let tick = Tick::new(Utc::now(), 1.1001, 1.1000, 100.0, 200.0);
        assert!((tick.spread() - 0.0001).abs() < 1e-10);
    }

    #[test]
    fn test_raw_tick_normalize() {
        let hour_start = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let raw = RawTick::new(1000, 110010, 110000, 100.0, 200.0);
        let tick = raw.normalize(hour_start, 100_000.0);

        assert_eq!(tick.timestamp, hour_start + TimeDelta::milliseconds(1000));
        assert!((tick.ask - 1.1001).abs() < 1e-10);
        assert!((tick.bid - 1.1000).abs() < 1e-10);
        assert!((tick.ask_volume - 100.0).abs() < 1e-10);
        assert!((tick.bid_volume - 200.0).abs() < 1e-10);
    }
}
