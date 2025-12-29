//! Streaming tick-to-OHLCV aggregation.

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use paracas_types::{Tick, Timeframe};

use crate::Ohlcv;

/// Streaming tick aggregator.
///
/// Aggregates ticks into OHLCV bars based on the configured timeframe.
#[derive(Debug)]
pub struct TickAggregator {
    timeframe: Timeframe,
    current_bar: Option<OhlcvBuilder>,
}

impl TickAggregator {
    /// Creates a new aggregator for the given timeframe.
    #[must_use]
    pub const fn new(timeframe: Timeframe) -> Self {
        Self {
            timeframe,
            current_bar: None,
        }
    }

    /// Returns the timeframe being aggregated to.
    #[must_use]
    pub const fn timeframe(&self) -> Timeframe {
        self.timeframe
    }

    /// Processes a tick, potentially emitting a completed bar.
    ///
    /// Returns `Some(bar)` when a bar is completed by this tick,
    /// `None` otherwise.
    pub fn process(&mut self, tick: Tick) -> Option<Ohlcv> {
        let bar_start = self.bar_start_for(tick.timestamp);

        match self.current_bar.take() {
            Some(mut builder) if builder.timestamp == bar_start => {
                // Same bar, update it
                builder.update(&tick);
                self.current_bar = Some(builder);
                None
            }
            Some(builder) => {
                // New bar started, finish the old one
                let completed = builder.finish();
                self.current_bar = Some(OhlcvBuilder::new(bar_start, &tick));
                Some(completed)
            }
            None => {
                // First tick
                self.current_bar = Some(OhlcvBuilder::new(bar_start, &tick));
                None
            }
        }
    }

    /// Finishes aggregation, returning any remaining partial bar.
    #[must_use]
    pub fn finish(self) -> Option<Ohlcv> {
        self.current_bar.map(|b| b.finish())
    }

    /// Calculates the bar start time for a given timestamp.
    fn bar_start_for(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        match self.timeframe {
            Timeframe::Tick => timestamp,
            Timeframe::Second1 => truncate_to_seconds(timestamp, 1),
            Timeframe::Minute1 => truncate_to_minutes(timestamp, 1),
            Timeframe::Minute5 => truncate_to_minutes(timestamp, 5),
            Timeframe::Minute15 => truncate_to_minutes(timestamp, 15),
            Timeframe::Minute30 => truncate_to_minutes(timestamp, 30),
            Timeframe::Hour1 => truncate_to_hours(timestamp, 1),
            Timeframe::Hour4 => truncate_to_hours(timestamp, 4),
            Timeframe::Day1 => truncate_to_day(timestamp),
        }
    }
}

/// Builder for OHLCV bars.
#[derive(Debug)]
struct OhlcvBuilder {
    timestamp: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    tick_count: u32,
}

impl OhlcvBuilder {
    /// Creates a new builder from the first tick.
    fn new(timestamp: DateTime<Utc>, tick: &Tick) -> Self {
        let mid = tick.mid();
        let volume = f64::from(tick.total_volume());
        Self {
            timestamp,
            open: mid,
            high: mid,
            low: mid,
            close: mid,
            volume,
            tick_count: 1,
        }
    }

    /// Updates the builder with a new tick.
    fn update(&mut self, tick: &Tick) {
        let mid = tick.mid();
        self.high = self.high.max(mid);
        self.low = self.low.min(mid);
        self.close = mid;
        self.volume += f64::from(tick.total_volume());
        self.tick_count += 1;
    }

    /// Finishes building and returns the OHLCV bar.
    const fn finish(self) -> Ohlcv {
        Ohlcv::new(
            self.timestamp,
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume,
            self.tick_count,
        )
    }
}

/// Truncates a timestamp to the start of a second boundary.
fn truncate_to_seconds(dt: DateTime<Utc>, interval: u32) -> DateTime<Utc> {
    let second = dt.second() / interval * interval;
    Utc.with_ymd_and_hms(
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        second,
    )
    .unwrap()
}

/// Truncates a timestamp to the start of a minute boundary.
fn truncate_to_minutes(dt: DateTime<Utc>, interval: u32) -> DateTime<Utc> {
    let minute = dt.minute() / interval * interval;
    Utc.with_ymd_and_hms(dt.year(), dt.month(), dt.day(), dt.hour(), minute, 0)
        .unwrap()
}

/// Truncates a timestamp to the start of an hour boundary.
fn truncate_to_hours(dt: DateTime<Utc>, interval: u32) -> DateTime<Utc> {
    let hour = dt.hour() / interval * interval;
    Utc.with_ymd_and_hms(dt.year(), dt.month(), dt.day(), hour, 0, 0)
        .unwrap()
}

/// Truncates a timestamp to the start of the day.
fn truncate_to_day(dt: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(dt.year(), dt.month(), dt.day(), 0, 0, 0)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn make_tick(hour: u32, minute: u32, second: u32, millis: u32, ask: f64, bid: f64) -> Tick {
        let timestamp = Utc
            .with_ymd_and_hms(2024, 1, 1, hour, minute, second)
            .unwrap()
            + TimeDelta::milliseconds(i64::from(millis));
        Tick::new(timestamp, ask, bid, 100.0, 100.0)
    }

    #[test]
    fn test_minute_aggregation() {
        let mut agg = TickAggregator::new(Timeframe::Minute1);

        // First tick at 12:00:00
        let tick1 = make_tick(12, 0, 0, 0, 1.1001, 1.1000);
        assert!(agg.process(tick1).is_none());

        // Second tick at 12:00:30 (same minute)
        let tick2 = make_tick(12, 0, 30, 0, 1.1010, 1.1005);
        assert!(agg.process(tick2).is_none());

        // Third tick at 12:01:00 (new minute, completes first bar)
        let tick3 = make_tick(12, 1, 0, 0, 1.0990, 1.0985);
        let bar = agg.process(tick3).unwrap();

        assert_eq!(bar.tick_count, 2);
        assert!((bar.open - 1.10005).abs() < 1e-10); // mid of first tick
        assert!((bar.close - 1.10075).abs() < 1e-10); // mid of second tick
    }

    #[test]
    fn test_hour_aggregation() {
        let mut agg = TickAggregator::new(Timeframe::Hour1);

        let tick1 = make_tick(12, 0, 0, 0, 1.1001, 1.1000);
        assert!(agg.process(tick1).is_none());

        let tick2 = make_tick(12, 30, 0, 0, 1.1050, 1.1045);
        assert!(agg.process(tick2).is_none());

        let tick3 = make_tick(13, 0, 0, 0, 1.0990, 1.0985);
        let bar = agg.process(tick3).unwrap();

        assert_eq!(bar.tick_count, 2);
        assert_eq!(bar.timestamp.hour(), 12);
    }

    #[test]
    fn test_finish() {
        let mut agg = TickAggregator::new(Timeframe::Hour1);

        let tick1 = make_tick(12, 0, 0, 0, 1.1001, 1.1000);
        agg.process(tick1);

        let bar = agg.finish().unwrap();
        assert_eq!(bar.tick_count, 1);
    }

    #[test]
    fn test_truncate_functions() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 14, 37, 45).unwrap();

        assert_eq!(truncate_to_minutes(dt, 5).minute(), 35);
        assert_eq!(truncate_to_minutes(dt, 15).minute(), 30);
        assert_eq!(truncate_to_hours(dt, 4).hour(), 12);
        assert_eq!(truncate_to_day(dt).hour(), 0);
    }
}
