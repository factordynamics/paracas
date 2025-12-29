//! Dukascopy URL construction.

use chrono::{DateTime, Datelike, Timelike, Utc};

/// Base URL for Dukascopy data feed.
pub const BASE_URL: &str = "https://datafeed.dukascopy.com/datafeed";

/// Builds the URL for a specific hour's tick data.
///
/// URL format: `{BASE_URL}/{INSTRUMENT}/{YEAR}/{MONTH}/{DAY}/{HOUR}h_ticks.bi5`
///
/// Note: Dukascopy uses 0-indexed months (January = 00).
///
/// # Example
///
/// ```
/// use paracas_fetch::url::tick_url;
/// use chrono::{TimeZone, Utc};
///
/// let hour = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
/// let url = tick_url("eurusd", hour);
/// assert_eq!(url, "https://datafeed.dukascopy.com/datafeed/EURUSD/2024/00/15/12h_ticks.bi5");
/// ```
#[must_use]
pub fn tick_url(instrument: &str, hour: DateTime<Utc>) -> String {
    format!(
        "{}/{}/{}/{:02}/{:02}/{:02}h_ticks.bi5",
        BASE_URL,
        instrument.to_uppercase(),
        hour.year(),
        hour.month() - 1, // Dukascopy uses 0-indexed months
        hour.day(),
        hour.hour()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_tick_url_january() {
        let hour = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let url = tick_url("eurusd", hour);
        assert_eq!(
            url,
            "https://datafeed.dukascopy.com/datafeed/EURUSD/2024/00/15/12h_ticks.bi5"
        );
    }

    #[test]
    fn test_tick_url_december() {
        let hour = Utc.with_ymd_and_hms(2024, 12, 31, 23, 0, 0).unwrap();
        let url = tick_url("btcusd", hour);
        assert_eq!(
            url,
            "https://datafeed.dukascopy.com/datafeed/BTCUSD/2024/11/31/23h_ticks.bi5"
        );
    }

    #[test]
    fn test_tick_url_uppercase() {
        let hour = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let url = tick_url("GBPJPY", hour);
        assert!(url.contains("GBPJPY"));
    }
}
