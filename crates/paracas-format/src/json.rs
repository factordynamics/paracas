//! JSON output format.

use paracas_aggregate::Ohlcv;
use paracas_types::Tick;
use std::io::Write;

use crate::{FormatError, Formatter};

/// JSON output style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsonStyle {
    /// JSON array (standard JSON).
    #[default]
    Array,
    /// Newline-delimited JSON (NDJSON/JSONL).
    Ndjson,
}

/// JSON formatter.
#[derive(Debug, Clone, Default)]
pub struct JsonFormatter {
    /// Output style.
    style: JsonStyle,
    /// Whether to pretty-print (only for array style).
    pretty: bool,
}

impl JsonFormatter {
    /// Creates a new JSON formatter with default settings (array style).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            style: JsonStyle::Array,
            pretty: false,
        }
    }

    /// Creates a new NDJSON formatter.
    #[must_use]
    pub const fn ndjson() -> Self {
        Self {
            style: JsonStyle::Ndjson,
            pretty: false,
        }
    }

    /// Sets whether to pretty-print output (array style only).
    #[must_use]
    pub const fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Sets the output style.
    #[must_use]
    pub const fn with_style(mut self, style: JsonStyle) -> Self {
        self.style = style;
        self
    }
}

impl Formatter for JsonFormatter {
    fn write_ticks<W: Write + Send>(
        &self,
        ticks: &[Tick],
        mut writer: W,
    ) -> Result<(), FormatError> {
        match self.style {
            JsonStyle::Array => {
                if self.pretty {
                    serde_json::to_writer_pretty(&mut writer, ticks)?;
                } else {
                    serde_json::to_writer(&mut writer, ticks)?;
                }
                writeln!(writer)?;
            }
            JsonStyle::Ndjson => {
                for tick in ticks {
                    serde_json::to_writer(&mut writer, tick)?;
                    writeln!(writer)?;
                }
            }
        }
        Ok(())
    }

    fn write_ohlcv<W: Write + Send>(
        &self,
        bars: &[Ohlcv],
        mut writer: W,
    ) -> Result<(), FormatError> {
        match self.style {
            JsonStyle::Array => {
                if self.pretty {
                    serde_json::to_writer_pretty(&mut writer, bars)?;
                } else {
                    serde_json::to_writer(&mut writer, bars)?;
                }
                writeln!(writer)?;
            }
            JsonStyle::Ndjson => {
                for bar in bars {
                    serde_json::to_writer(&mut writer, bar)?;
                    writeln!(writer)?;
                }
            }
        }
        Ok(())
    }

    fn extension(&self) -> &str {
        match self.style {
            JsonStyle::Array => "json",
            JsonStyle::Ndjson => "ndjson",
        }
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
    fn test_json_array() {
        let formatter = JsonFormatter::new();
        let ticks = vec![create_test_tick()];
        let mut output = Cursor::new(Vec::new());

        formatter.write_ticks(&ticks, &mut output).unwrap();

        let result = String::from_utf8(output.into_inner()).unwrap();
        assert!(result.starts_with('['));
        assert!(result.contains("\"ask\":1.1001"));
    }

    #[test]
    fn test_ndjson() {
        let formatter = JsonFormatter::ndjson();
        let ticks = vec![create_test_tick(), create_test_tick()];
        let mut output = Cursor::new(Vec::new());

        formatter.write_ticks(&ticks, &mut output).unwrap();

        let result = String::from_utf8(output.into_inner()).unwrap();
        let lines: Vec<_> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with('{'));
    }

    #[test]
    fn test_pretty_json() {
        let formatter = JsonFormatter::new().with_pretty(true);
        let ticks = vec![create_test_tick()];
        let mut output = Cursor::new(Vec::new());

        formatter.write_ticks(&ticks, &mut output).unwrap();

        let result = String::from_utf8(output.into_inner()).unwrap();
        assert!(result.contains('\n'));
        assert!(result.contains("  ")); // Indentation
    }
}
