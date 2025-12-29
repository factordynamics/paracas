//! LZMA decompression for bi5 files.

use lzma_rs::lzma_decompress;
use std::io::{BufReader, Cursor};
use thiserror::Error;

/// Errors that can occur during decompression.
#[derive(Error, Debug)]
pub enum DecompressError {
    /// LZMA decompression failed.
    #[error("LZMA decompression failed: {0}")]
    LzmaError(String),

    /// Empty input data.
    #[error("Empty input data")]
    EmptyInput,
}

/// Decompresses LZMA-compressed bi5 data.
///
/// Dukascopy bi5 files are LZMA-compressed binary data containing tick records.
///
/// # Errors
///
/// Returns an error if decompression fails.
///
/// # Example
///
/// ```ignore
/// use paracas_fetch::decompress_bi5;
///
/// let compressed = /* bi5 data from HTTP */;
/// let decompressed = decompress_bi5(&compressed)?;
/// ```
pub fn decompress_bi5(compressed: &[u8]) -> Result<Vec<u8>, DecompressError> {
    if compressed.is_empty() {
        return Err(DecompressError::EmptyInput);
    }

    let mut decompressed = Vec::new();
    let mut reader = BufReader::new(Cursor::new(compressed));

    lzma_decompress(&mut reader, &mut decompressed)
        .map_err(|e| DecompressError::LzmaError(e.to_string()))?;

    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let result = decompress_bi5(&[]);
        assert!(matches!(result, Err(DecompressError::EmptyInput)));
    }

    #[test]
    fn test_invalid_lzma() {
        let result = decompress_bi5(&[0x00, 0x01, 0x02, 0x03]);
        assert!(matches!(result, Err(DecompressError::LzmaError(_))));
    }
}
