//! Apache Parquet output format.

use arrow::array::{Float32Array, Float64Array, TimestampMicrosecondArray, UInt32Array};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use paracas_aggregate::Ohlcv;
use paracas_types::Tick;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::io::Write;
use std::sync::Arc;

use crate::{FormatError, Formatter};

/// Parquet formatter.
#[derive(Debug, Clone)]
pub struct ParquetFormatter {
    /// Row group size (number of rows per group).
    row_group_size: usize,
    /// Compression codec.
    compression: Compression,
}

impl Default for ParquetFormatter {
    fn default() -> Self {
        Self {
            row_group_size: 100_000,
            compression: Compression::SNAPPY,
        }
    }
}

impl ParquetFormatter {
    /// Creates a new Parquet formatter with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the row group size.
    #[must_use]
    pub const fn with_row_group_size(mut self, size: usize) -> Self {
        self.row_group_size = size;
        self
    }

    /// Sets the compression codec.
    #[must_use]
    pub const fn with_compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    /// Creates the Arrow schema for tick data.
    fn tick_schema() -> Schema {
        Schema::new(vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
                false,
            ),
            Field::new("ask", DataType::Float64, false),
            Field::new("bid", DataType::Float64, false),
            Field::new("ask_volume", DataType::Float32, false),
            Field::new("bid_volume", DataType::Float32, false),
        ])
    }

    /// Creates the Arrow schema for OHLCV data.
    fn ohlcv_schema() -> Schema {
        Schema::new(vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
                false,
            ),
            Field::new("open", DataType::Float64, false),
            Field::new("high", DataType::Float64, false),
            Field::new("low", DataType::Float64, false),
            Field::new("close", DataType::Float64, false),
            Field::new("volume", DataType::Float64, false),
            Field::new("tick_count", DataType::UInt32, false),
        ])
    }

    /// Converts ticks to Arrow RecordBatch.
    fn ticks_to_batch(ticks: &[Tick]) -> Result<RecordBatch, FormatError> {
        let timestamps: Vec<_> = ticks
            .iter()
            .map(|t| t.timestamp.timestamp_micros())
            .collect();
        let asks: Vec<_> = ticks.iter().map(|t| t.ask).collect();
        let bids: Vec<_> = ticks.iter().map(|t| t.bid).collect();
        let ask_vols: Vec<_> = ticks.iter().map(|t| t.ask_volume).collect();
        let bid_vols: Vec<_> = ticks.iter().map(|t| t.bid_volume).collect();

        RecordBatch::try_new(
            Arc::new(Self::tick_schema()),
            vec![
                Arc::new(TimestampMicrosecondArray::from(timestamps).with_timezone("UTC")),
                Arc::new(Float64Array::from(asks)),
                Arc::new(Float64Array::from(bids)),
                Arc::new(Float32Array::from(ask_vols)),
                Arc::new(Float32Array::from(bid_vols)),
            ],
        )
        .map_err(|e| FormatError::Parquet(e.to_string()))
    }

    /// Converts OHLCV bars to Arrow RecordBatch.
    fn ohlcv_to_batch(bars: &[Ohlcv]) -> Result<RecordBatch, FormatError> {
        let timestamps: Vec<_> = bars
            .iter()
            .map(|b| b.timestamp.timestamp_micros())
            .collect();
        let opens: Vec<_> = bars.iter().map(|b| b.open).collect();
        let highs: Vec<_> = bars.iter().map(|b| b.high).collect();
        let lows: Vec<_> = bars.iter().map(|b| b.low).collect();
        let closes: Vec<_> = bars.iter().map(|b| b.close).collect();
        let volumes: Vec<_> = bars.iter().map(|b| b.volume).collect();
        let tick_counts: Vec<_> = bars.iter().map(|b| b.tick_count).collect();

        RecordBatch::try_new(
            Arc::new(Self::ohlcv_schema()),
            vec![
                Arc::new(TimestampMicrosecondArray::from(timestamps).with_timezone("UTC")),
                Arc::new(Float64Array::from(opens)),
                Arc::new(Float64Array::from(highs)),
                Arc::new(Float64Array::from(lows)),
                Arc::new(Float64Array::from(closes)),
                Arc::new(Float64Array::from(volumes)),
                Arc::new(UInt32Array::from(tick_counts)),
            ],
        )
        .map_err(|e| FormatError::Parquet(e.to_string()))
    }
}

impl Formatter for ParquetFormatter {
    fn write_ticks<W: Write + Send>(&self, ticks: &[Tick], writer: W) -> Result<(), FormatError> {
        let schema = Arc::new(Self::tick_schema());
        let props = WriterProperties::builder()
            .set_compression(self.compression)
            .set_max_row_group_size(self.row_group_size)
            .build();

        let mut arrow_writer = ArrowWriter::try_new(writer, schema, Some(props))
            .map_err(|e| FormatError::Parquet(e.to_string()))?;

        // Write in batches
        for chunk in ticks.chunks(self.row_group_size) {
            let batch = Self::ticks_to_batch(chunk)?;
            arrow_writer
                .write(&batch)
                .map_err(|e| FormatError::Parquet(e.to_string()))?;
        }

        arrow_writer
            .close()
            .map_err(|e| FormatError::Parquet(e.to_string()))?;

        Ok(())
    }

    fn write_ohlcv<W: Write + Send>(&self, bars: &[Ohlcv], writer: W) -> Result<(), FormatError> {
        let schema = Arc::new(Self::ohlcv_schema());
        let props = WriterProperties::builder()
            .set_compression(self.compression)
            .set_max_row_group_size(self.row_group_size)
            .build();

        let mut arrow_writer = ArrowWriter::try_new(writer, schema, Some(props))
            .map_err(|e| FormatError::Parquet(e.to_string()))?;

        // Write in batches
        for chunk in bars.chunks(self.row_group_size) {
            let batch = Self::ohlcv_to_batch(chunk)?;
            arrow_writer
                .write(&batch)
                .map_err(|e| FormatError::Parquet(e.to_string()))?;
        }

        arrow_writer
            .close()
            .map_err(|e| FormatError::Parquet(e.to_string()))?;

        Ok(())
    }

    fn extension(&self) -> &str {
        "parquet"
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
    fn test_parquet_ticks() {
        let formatter = ParquetFormatter::new();
        let ticks = vec![create_test_tick()];
        let mut output = Cursor::new(Vec::new());

        formatter.write_ticks(&ticks, &mut output).unwrap();

        // Parquet files start with "PAR1" magic bytes
        let data = output.into_inner();
        assert!(data.len() > 4);
        assert_eq!(&data[0..4], b"PAR1");
    }

    #[test]
    fn test_tick_schema() {
        let schema = ParquetFormatter::tick_schema();
        assert_eq!(schema.fields().len(), 5);
        assert!(schema.field_with_name("timestamp").is_ok());
        assert!(schema.field_with_name("ask").is_ok());
    }

    #[test]
    fn test_ohlcv_schema() {
        let schema = ParquetFormatter::ohlcv_schema();
        assert_eq!(schema.fields().len(), 7);
        assert!(schema.field_with_name("open").is_ok());
        assert!(schema.field_with_name("close").is_ok());
    }
}
