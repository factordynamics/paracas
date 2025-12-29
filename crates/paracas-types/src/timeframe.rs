//! OHLCV aggregation timeframe definitions.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// OHLCV aggregation timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    /// Tick-by-tick (no aggregation).
    #[default]
    Tick,
    /// 1-second bars.
    #[serde(rename = "s1")]
    Second1,
    /// 1-minute bars.
    #[serde(rename = "m1")]
    Minute1,
    /// 5-minute bars.
    #[serde(rename = "m5")]
    Minute5,
    /// 15-minute bars.
    #[serde(rename = "m15")]
    Minute15,
    /// 30-minute bars.
    #[serde(rename = "m30")]
    Minute30,
    /// 1-hour bars.
    #[serde(rename = "h1")]
    Hour1,
    /// 4-hour bars.
    #[serde(rename = "h4")]
    Hour4,
    /// Daily bars.
    #[serde(rename = "d1")]
    Day1,
}

impl Timeframe {
    /// Returns the duration in seconds, or None for tick data.
    #[must_use]
    pub const fn seconds(&self) -> Option<u64> {
        match self {
            Self::Tick => None,
            Self::Second1 => Some(1),
            Self::Minute1 => Some(60),
            Self::Minute5 => Some(300),
            Self::Minute15 => Some(900),
            Self::Minute30 => Some(1800),
            Self::Hour1 => Some(3600),
            Self::Hour4 => Some(14400),
            Self::Day1 => Some(86400),
        }
    }

    /// Returns the duration in milliseconds, or None for tick data.
    #[must_use]
    pub const fn milliseconds(&self) -> Option<u64> {
        match self.seconds() {
            Some(s) => Some(s * 1000),
            None => None,
        }
    }

    /// Returns true if this is tick data (no aggregation).
    #[must_use]
    pub const fn is_tick(&self) -> bool {
        matches!(self, Self::Tick)
    }

    /// Returns the timeframe as a string identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Tick => "tick",
            Self::Second1 => "s1",
            Self::Minute1 => "m1",
            Self::Minute5 => "m5",
            Self::Minute15 => "m15",
            Self::Minute30 => "m30",
            Self::Hour1 => "h1",
            Self::Hour4 => "h4",
            Self::Day1 => "d1",
        }
    }

    /// Returns all available timeframes.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Tick,
            Self::Second1,
            Self::Minute1,
            Self::Minute5,
            Self::Minute15,
            Self::Minute30,
            Self::Hour1,
            Self::Hour4,
            Self::Day1,
        ]
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Timeframe {
    type Err = TimeframeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tick" => Ok(Self::Tick),
            "s1" | "1s" | "second" | "second1" => Ok(Self::Second1),
            "m1" | "1m" | "minute" | "minute1" => Ok(Self::Minute1),
            "m5" | "5m" | "minute5" => Ok(Self::Minute5),
            "m15" | "15m" | "minute15" => Ok(Self::Minute15),
            "m30" | "30m" | "minute30" => Ok(Self::Minute30),
            "h1" | "1h" | "hour" | "hour1" => Ok(Self::Hour1),
            "h4" | "4h" | "hour4" => Ok(Self::Hour4),
            "d1" | "1d" | "day" | "day1" | "daily" => Ok(Self::Day1),
            _ => Err(TimeframeParseError(s.to_string())),
        }
    }
}

/// Error returned when parsing an invalid timeframe string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeframeParseError(String);

impl std::fmt::Display for TimeframeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid timeframe '{}', expected one of: tick, s1, m1, m5, m15, m30, h1, h4, d1",
            self.0
        )
    }
}

impl std::error::Error for TimeframeParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeframe_seconds() {
        assert_eq!(Timeframe::Tick.seconds(), None);
        assert_eq!(Timeframe::Minute1.seconds(), Some(60));
        assert_eq!(Timeframe::Hour1.seconds(), Some(3600));
        assert_eq!(Timeframe::Day1.seconds(), Some(86400));
    }

    #[test]
    fn test_timeframe_parse() {
        assert_eq!("m1".parse::<Timeframe>().unwrap(), Timeframe::Minute1);
        assert_eq!("1h".parse::<Timeframe>().unwrap(), Timeframe::Hour1);
        assert_eq!("H4".parse::<Timeframe>().unwrap(), Timeframe::Hour4);
        assert!("invalid".parse::<Timeframe>().is_err());
    }
}
