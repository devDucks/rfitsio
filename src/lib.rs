pub mod hdu;
pub mod parsing;

pub use hdu::HDU;

use std::fs::File;
use std::io::Write;

/// Return how many padding bytes are needed to align `n` to the next
/// 2880-byte FITS block boundary.  Returns 0 if `n` is already aligned.
pub fn fill_to_2880(n: i32) -> i32 {
    match n % 2880 {
        0 => 0,
        m => 2880 - m,
    }
}

/// A complete FITS file: an ordered collection of Header Data Units (HDUs).
///
/// The first HDU is the primary HDU and must begin with the `SIMPLE` keyword.
/// Additional HDUs are extensions (IMAGE, BINTABLE, TABLE).
#[derive(Debug)]
pub struct FITSFile {
    pub hdus: Vec<HDU>,
}

impl FITSFile {
    pub fn new() -> Self {
        FITSFile { hdus: Vec::new() }
    }

    pub fn add_hdu(&mut self, hdu: HDU) {
        self.hdus.push(hdu);
    }

    /// Serialize the entire file to a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.hdus.iter().flat_map(|h| h.to_bytes()).collect()
    }

    /// Write the file to `path`, creating or truncating it.
    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.to_bytes())
    }
}

/// Utilities for converting between native byte order and the FITS-mandated
/// big-endian byte order.
///
/// FITS always stores multi-byte values (integers, floats) in big-endian order.
/// On little-endian hosts (e.g. x86) data must be swapped before writing and
/// after reading.
pub mod endian {
    /// Encode `i16` values as big-endian bytes (BITPIX = 16).
    pub fn i16_to_be_bytes(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// Encode `i32` values as big-endian bytes (BITPIX = 32).
    pub fn i32_to_be_bytes(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// Encode `i64` values as big-endian bytes (BITPIX = 64).
    pub fn i64_to_be_bytes(values: &[i64]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// Encode `f32` values as big-endian bytes (BITPIX = -32).
    pub fn f32_to_be_bytes(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// Encode `f64` values as big-endian bytes (BITPIX = -64).
    pub fn f64_to_be_bytes(values: &[f64]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// Decode big-endian bytes into `i16` values (BITPIX = 16).
    pub fn be_bytes_to_i16(bytes: &[u8]) -> Vec<i16> {
        bytes
            .chunks_exact(2)
            .map(|c| i16::from_be_bytes([c[0], c[1]]))
            .collect()
    }

    /// Decode big-endian bytes into `i32` values (BITPIX = 32).
    pub fn be_bytes_to_i32(bytes: &[u8]) -> Vec<i32> {
        bytes
            .chunks_exact(4)
            .map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    /// Decode big-endian bytes into `i64` values (BITPIX = 64).
    pub fn be_bytes_to_i64(bytes: &[u8]) -> Vec<i64> {
        bytes
            .chunks_exact(8)
            .map(|c| i64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect()
    }

    /// Decode big-endian bytes into `f32` values (BITPIX = -32).
    pub fn be_bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    /// Decode big-endian bytes into `f64` values (BITPIX = -64).
    pub fn be_bytes_to_f64(bytes: &[u8]) -> Vec<f64> {
        bytes
            .chunks_exact(8)
            .map(|c| f64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::endian;
    use crate::fill_to_2880;

    #[test]
    fn check_2880_bytes_hdu_not_add_any() {
        assert_eq!(fill_to_2880(2880), 0)
    }

    #[test]
    fn check_5760_bytes_hdu_not_add_any() {
        assert_eq!(fill_to_2880(5760), 0)
    }

    #[test]
    fn test_2885_bytes_hdu_add_5() {
        assert_eq!(fill_to_2880(2885), 2875)
    }

    #[test]
    fn test_1936_1096_bytes_hdu_not_add_any() {
        assert_eq!(fill_to_2880(1936 * 1096), 704)
    }

    // --- endian round-trip tests ---

    #[test]
    fn i16_roundtrip() {
        let original = vec![0_i16, 1, -1, i16::MAX, i16::MIN];
        let bytes = endian::i16_to_be_bytes(&original);
        let decoded = endian::be_bytes_to_i16(&bytes);
        assert_eq!(original, decoded);
    }

    #[test]
    fn i32_roundtrip() {
        let original = vec![0_i32, 1, -1, i32::MAX, i32::MIN];
        let bytes = endian::i32_to_be_bytes(&original);
        let decoded = endian::be_bytes_to_i32(&bytes);
        assert_eq!(original, decoded);
    }

    #[test]
    fn f32_roundtrip() {
        let original = vec![0.0_f32, 1.0, -1.0, f32::MAX, f32::MIN_POSITIVE];
        let bytes = endian::f32_to_be_bytes(&original);
        let decoded = endian::be_bytes_to_f32(&bytes);
        assert_eq!(original, decoded);
    }

    #[test]
    fn f64_roundtrip() {
        let original = vec![0.0_f64, 1.0, -1.0, f64::MAX, f64::MIN_POSITIVE];
        let bytes = endian::f64_to_be_bytes(&original);
        let decoded = endian::be_bytes_to_f64(&bytes);
        assert_eq!(original, decoded);
    }
}
