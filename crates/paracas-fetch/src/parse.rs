//! Binary tick parsing from bi5 format.

use byteorder::{BigEndian, ByteOrder};
use paracas_types::RawTick;
use thiserror::Error;

/// Errors that can occur during tick parsing.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Invalid data length (not a multiple of tick size).
    #[error("Invalid data length: {0} bytes (expected multiple of {1})")]
    InvalidLength(usize, usize),

    /// Incomplete tick record.
    #[error("Incomplete tick record at offset {0}")]
    IncompleteRecord(usize),
}

/// Parses raw ticks from decompressed bi5 data.
///
/// The bi5 format stores ticks as 20 bytes in big-endian order:
/// - `u32`: milliseconds offset from hour start (bytes 0-3)
/// - `u32`: ask price raw (bytes 4-7)
/// - `u32`: bid price raw (bytes 8-11)
/// - `f32`: ask volume (bytes 12-15)
/// - `f32`: bid volume (bytes 16-19)
///
/// # Arguments
///
/// * `data` - Decompressed bi5 data
///
/// # Returns
///
/// An iterator over parsed raw ticks.
///
/// # Errors
///
/// Returns an error if the data length is invalid.
pub fn parse_ticks(data: &[u8]) -> Result<impl Iterator<Item = RawTick> + '_, ParseError> {
    if !data.len().is_multiple_of(RawTick::SIZE) {
        return Err(ParseError::InvalidLength(data.len(), RawTick::SIZE));
    }

    Ok(data.chunks_exact(RawTick::SIZE).map(parse_single_tick))
}

/// Parses a single tick from a 20-byte chunk.
#[inline]
fn parse_single_tick(data: &[u8]) -> RawTick {
    RawTick::new(
        BigEndian::read_u32(&data[0..4]),
        BigEndian::read_u32(&data[4..8]),
        BigEndian::read_u32(&data[8..12]),
        BigEndian::read_f32(&data[12..16]),
        BigEndian::read_f32(&data[16..20]),
    )
}

/// Returns the number of ticks in the given data.
#[must_use]
pub const fn tick_count(data_len: usize) -> usize {
    data_len / RawTick::SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tick_bytes(ms: u32, ask: u32, bid: u32, ask_vol: f32, bid_vol: f32) -> Vec<u8> {
        let mut bytes = vec![0u8; 20];
        BigEndian::write_u32(&mut bytes[0..4], ms);
        BigEndian::write_u32(&mut bytes[4..8], ask);
        BigEndian::write_u32(&mut bytes[8..12], bid);
        BigEndian::write_f32(&mut bytes[12..16], ask_vol);
        BigEndian::write_f32(&mut bytes[16..20], bid_vol);
        bytes
    }

    #[test]
    fn test_parse_single_tick() {
        let bytes = create_test_tick_bytes(1000, 112345, 112340, 100.0, 200.0);
        let tick = parse_single_tick(&bytes);

        assert_eq!(tick.ms_offset, 1000);
        assert_eq!(tick.ask_raw, 112345);
        assert_eq!(tick.bid_raw, 112340);
        assert!((tick.ask_volume - 100.0).abs() < 0.001);
        assert!((tick.bid_volume - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_multiple_ticks() {
        let mut data = create_test_tick_bytes(0, 100, 99, 10.0, 20.0);
        data.extend(create_test_tick_bytes(1000, 101, 100, 15.0, 25.0));

        let ticks: Vec<_> = parse_ticks(&data).unwrap().collect();
        assert_eq!(ticks.len(), 2);
        assert_eq!(ticks[0].ms_offset, 0);
        assert_eq!(ticks[1].ms_offset, 1000);
    }

    #[test]
    fn test_invalid_length() {
        let data = vec![0u8; 25]; // Not a multiple of 20
        let result = parse_ticks(&data);
        assert!(matches!(result, Err(ParseError::InvalidLength(25, 20))));
    }

    #[test]
    fn test_empty_data() {
        let data: Vec<u8> = vec![];
        let ticks: Vec<_> = parse_ticks(&data).unwrap().collect();
        assert!(ticks.is_empty());
    }

    #[test]
    fn test_tick_count() {
        assert_eq!(tick_count(0), 0);
        assert_eq!(tick_count(20), 1);
        assert_eq!(tick_count(200), 10);
    }
}
