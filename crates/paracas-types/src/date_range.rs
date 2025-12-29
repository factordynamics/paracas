//! Date range and hour iteration.

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::DateRangeError;

/// A range of dates for data retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    /// Start date (inclusive).
    pub start: NaiveDate,
    /// End date (inclusive).
    pub end: NaiveDate,
}

impl DateRange {
    /// Creates a new date range, validating that start <= end.
    ///
    /// # Errors
    ///
    /// Returns an error if start > end.
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, DateRangeError> {
        if start > end {
            return Err(DateRangeError::InvalidRange { start, end });
        }
        Ok(Self { start, end })
    }

    /// Creates a date range for a single day.
    #[must_use]
    pub const fn single_day(date: NaiveDate) -> Self {
        Self {
            start: date,
            end: date,
        }
    }

    /// Returns an iterator over all hours in the date range.
    pub fn hours(&self) -> HourIterator {
        HourIterator::new(self.start, self.end)
    }

    /// Returns the total number of hours in the range.
    #[must_use]
    pub fn total_hours(&self) -> usize {
        let days = (self.end - self.start).num_days() + 1;
        (days * 24) as usize
    }

    /// Returns the total number of days in the range.
    #[must_use]
    pub fn total_days(&self) -> usize {
        ((self.end - self.start).num_days() + 1) as usize
    }

    /// Returns true if the range contains the given date.
    #[must_use]
    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start && date <= self.end
    }
}

impl std::fmt::Display for DateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} to {}", self.start, self.end)
    }
}

/// Iterator over all hours in a date range.
#[derive(Debug, Clone)]
pub struct HourIterator {
    current: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl HourIterator {
    /// Creates a new hour iterator for the given date range.
    fn new(start: NaiveDate, end: NaiveDate) -> Self {
        let start_dt =
            Utc.from_utc_datetime(&start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
        // End at 23:00 of the end date (last hour of the day)
        let end_dt =
            Utc.from_utc_datetime(&end.and_time(NaiveTime::from_hms_opt(23, 0, 0).unwrap()));

        Self {
            current: start_dt,
            end: end_dt,
        }
    }
}

impl Iterator for HourIterator {
    type Item = DateTime<Utc>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > self.end {
            return None;
        }

        let result = self.current;
        self.current += chrono::TimeDelta::hours(1);
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.current > self.end {
            return (0, Some(0));
        }
        let hours = (self.end - self.current).num_hours() as usize + 1;
        (hours, Some(hours))
    }
}

impl ExactSizeIterator for HourIterator {}

/// Extracts the hour start timestamp from a Dukascopy URL.
///
/// URL format: `https://datafeed.dukascopy.com/datafeed/{INSTRUMENT}/{YEAR}/{MONTH}/{DAY}/{HOUR}h_ticks.bi5`
/// Note: Month in URL is 0-indexed (January = 00).
#[must_use]
pub fn hour_from_url(url: &str) -> Option<DateTime<Utc>> {
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() < 5 {
        return None;
    }

    // Parse from the end: .../{YEAR}/{MONTH}/{DAY}/{HOUR}h_ticks.bi5
    let hour_part = parts.last()?;
    let hour: u32 = hour_part.strip_suffix("h_ticks.bi5")?.parse().ok()?;
    let day: u32 = parts.get(parts.len() - 2)?.parse().ok()?;
    let month: u32 = parts.get(parts.len() - 3)?.parse().ok()?;
    let year: i32 = parts.get(parts.len() - 4)?.parse().ok()?;

    // Month in URL is 0-indexed
    Utc.with_ymd_and_hms(year, month + 1, day, hour, 0, 0)
        .single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_date_range_new() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let range = DateRange::new(start, end).unwrap();

        assert_eq!(range.start, start);
        assert_eq!(range.end, end);
    }

    #[test]
    fn test_date_range_invalid() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(DateRange::new(start, end).is_err());
    }

    #[test]
    fn test_total_hours() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let range = DateRange::new(start, end).unwrap();

        assert_eq!(range.total_hours(), 24);
    }

    #[test]
    fn test_hour_iterator() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let range = DateRange::single_day(start);
        let hours: Vec<_> = range.hours().collect();

        assert_eq!(hours.len(), 24);
        assert_eq!(hours[0].hour(), 0);
        assert_eq!(hours[23].hour(), 23);
    }

    #[test]
    fn test_hour_from_url() {
        let url = "https://datafeed.dukascopy.com/datafeed/EURUSD/2024/00/15/12h_ticks.bi5";
        let hour = hour_from_url(url).unwrap();

        assert_eq!(hour.year(), 2024);
        assert_eq!(hour.month(), 1); // 00 -> January
        assert_eq!(hour.day(), 15);
        assert_eq!(hour.hour(), 12);
    }
}
