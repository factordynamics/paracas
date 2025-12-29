//! CSV output format.

use paracas_aggregate::Ohlcv;
use paracas_types::Tick;
use std::io::Write;

use crate::{FormatError, Formatter};

/// CSV formatter.
#[derive(Debug, Clone, Default)]
pub struct CsvFormatter {
    /// Field delimiter (default: comma).
    delimiter: char,
    /// Whether to include header row.
    include_header: bool,
}

impl CsvFormatter {
    /// Creates a new CSV formatter with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            delimiter: ',',
            include_header: true,
        }
    }

    /// Sets the field delimiter.
    #[must_use]
    pub const fn with_delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Sets whether to include a header row.
    #[must_use]
    pub const fn with_header(mut self, include: bool) -> Self {
        self.include_header = include;
        self
    }

    /// Creates a tab-separated values (TSV) formatter.
    #[must_use]
    pub const fn tsv() -> Self {
        Self {
            delimiter: '\t',
            include_header: true,
        }
    }
}

impl Formatter for CsvFormatter {
    fn write_ticks<W: Write + Send>(
        &self,
        ticks: &[Tick],
        mut writer: W,
    ) -> Result<(), FormatError> {
        let d = self.delimiter;

        if self.include_header {
            writeln!(writer, "timestamp{d}ask{d}bid{d}ask_volume{d}bid_volume")?;
        }

        for tick in ticks {
            writeln!(
                writer,
                "{}{d}{}{d}{}{d}{}{d}{}",
                tick.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                tick.ask,
                tick.bid,
                tick.ask_volume,
                tick.bid_volume
            )?;
        }

        Ok(())
    }

    fn write_ohlcv<W: Write + Send>(
        &self,
        bars: &[Ohlcv],
        mut writer: W,
    ) -> Result<(), FormatError> {
        let d = self.delimiter;

        if self.include_header {
            writeln!(
                writer,
                "timestamp{d}open{d}high{d}low{d}close{d}volume{d}tick_count"
            )?;
        }

        for bar in bars {
            writeln!(
                writer,
                "{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}",
                bar.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
                bar.open,
                bar.high,
                bar.low,
                bar.close,
                bar.volume,
                bar.tick_count
            )?;
        }

        Ok(())
    }

    fn extension(&self) -> &str {
        "csv"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::io::Cursor;

    fn create_test_tick() -> Tick {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 45).unwrap();
        Tick::new(timestamp, 1.1001, 1.1000, 100.0, 200.0)
    }

    #[test]
    fn test_csv_ticks() {
        let formatter = CsvFormatter::new();
        let ticks = vec![create_test_tick()];
        let mut output = Cursor::new(Vec::new());

        formatter.write_ticks(&ticks, &mut output).unwrap();

        let result = String::from_utf8(output.into_inner()).unwrap();
        assert!(result.contains("timestamp,ask,bid,ask_volume,bid_volume"));
        assert!(result.contains("2024-01-15T12:30:45.000Z"));
        assert!(result.contains("1.1001"));
    }

    #[test]
    fn test_csv_no_header() {
        let formatter = CsvFormatter::new().with_header(false);
        let ticks = vec![create_test_tick()];
        let mut output = Cursor::new(Vec::new());

        formatter.write_ticks(&ticks, &mut output).unwrap();

        let result = String::from_utf8(output.into_inner()).unwrap();
        assert!(!result.contains("timestamp,ask"));
    }

    #[test]
    fn test_tsv() {
        let formatter = CsvFormatter::tsv();
        let ticks = vec![create_test_tick()];
        let mut output = Cursor::new(Vec::new());

        formatter.write_ticks(&ticks, &mut output).unwrap();

        let result = String::from_utf8(output.into_inner()).unwrap();
        assert!(result.contains("timestamp\task\tbid"));
    }
}
