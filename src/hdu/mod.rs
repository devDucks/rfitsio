pub mod bintable;
pub mod data;
pub mod headers;

use crate::fill_to_2880;
use crate::hdu::data::FITSData;
use crate::hdu::headers::{FITSHeader, FITSValue};

#[derive(Debug)]
pub struct HDU {
    pub headers: Vec<FITSHeader>,
    pub data: FITSData,
}

impl HDU {
    pub fn init() -> HDU {
        HDU {
            headers: Vec::new(),
            data: FITSData::new(),
        }
    }

    pub fn raw_data(&self) -> &Vec<u8> {
        &self.data.data
    }

    pub fn add_data(&mut self, data: Vec<u8>) {
        self.data.add(data);
    }

    pub fn add_header(&mut self, key: &str, value: FITSValue) {
        self.headers.push(FITSHeader::new(key, value));
    }

    pub fn add_header_with_comment(&mut self, key: &str, value: FITSValue, comment: &str) {
        self.headers
            .push(FITSHeader::new_with_comment(key, value, comment));
    }

    pub fn add_comment(&mut self, text: &str) {
        self.headers.push(FITSHeader::comment(text));
    }

    pub fn add_history(&mut self, text: &str) {
        self.headers.push(FITSHeader::history(text));
    }

    /// Validate this HDU as a primary HDU (the first HDU in a FITS file).
    ///
    /// Per FITS spec the primary HDU must have:
    /// - `SIMPLE` as the first keyword
    /// - `BITPIX` present
    /// - `NAXIS` present
    /// - `END` as the last keyword
    pub fn validate_primary(&self) -> Result<(), String> {
        if self.headers.is_empty() {
            return Err("primary HDU has no headers".into());
        }

        if &self.headers[0].key[..6] != b"SIMPLE" {
            return Err(format!(
                "first keyword must be SIMPLE, found: {}",
                std::str::from_utf8(&self.headers[0].key[..6]).unwrap_or("?")
            ));
        }

        let has_bitpix = self.headers.iter().any(|h| &h.key[..6] == b"BITPIX");
        if !has_bitpix {
            return Err("BITPIX keyword is required in the primary HDU".into());
        }

        let has_naxis = self
            .headers
            .iter()
            .any(|h| &h.key[..5] == b"NAXIS" && h.key[5] == b' ');
        if !has_naxis {
            return Err("NAXIS keyword is required in the primary HDU".into());
        }

        if &self.headers.last().unwrap().key[..3] != b"END" {
            return Err("last keyword must be END".into());
        }

        Ok(())
    }

    /// Serialize this HDU to a byte vector suitable for writing to a FITS file.
    ///
    /// - Header records are written as 80-byte blocks, then padded with spaces
    ///   to the next 2880-byte boundary.
    /// - Data bytes follow (already padded with null bytes by `FITSData::add`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for header in &self.headers {
            bytes.extend_from_slice(&header.key);
            bytes.extend_from_slice(&header.value.val);
        }

        // Pad the header section to the next 2880-byte boundary with spaces.
        let header_padding = fill_to_2880(bytes.len() as i32) as usize;
        bytes.extend(vec![b' '; header_padding]);

        // Data is already null-padded by FITSData::add().
        bytes.extend_from_slice(&self.data.data);

        bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::hdu::{HDU, headers::FITSValue};

    #[test]
    fn check_initialization() {
        let hdu = HDU::init();
        assert!(hdu.headers.is_empty());
        assert!(hdu.raw_data().is_empty());
    }

    #[test]
    fn check_raw_data() {
        let mut hdu = HDU::init();
        hdu.add_data(vec![9; 5]);
        assert_eq!(hdu.raw_data()[0..5], [9, 9, 9, 9, 9]);
        assert_eq!(hdu.raw_data().len(), 2880);
    }

    #[test]
    fn check_adding_header() {
        let mut hdu = HDU::init();
        hdu.add_header("NAXIS", FITSValue::Integer(2));
        let key = hdu.headers[0].key_as_str();
        let value = hdu.headers[0].value_as_str();
        assert!(key.contains("NAXIS"));
        assert!(value.contains('2'));
    }

    #[test]
    fn to_bytes_header_section_multiple_of_2880() {
        let mut hdu = HDU::init();
        hdu.add_header("SIMPLE", FITSValue::Logical(true));
        hdu.add_header("BITPIX", FITSValue::Integer(8));
        hdu.add_header("NAXIS", FITSValue::Integer(0));
        hdu.headers.push(crate::hdu::headers::FITSHeader::end_hdu());
        let bytes = hdu.to_bytes();
        assert_eq!(
            bytes.len() % 2880,
            0,
            "serialized HDU must be a multiple of 2880 bytes"
        );
    }

    #[test]
    fn validate_primary_passes_for_valid_hdu() {
        let mut hdu = HDU::init();
        hdu.add_header("SIMPLE", FITSValue::Logical(true));
        hdu.add_header("BITPIX", FITSValue::Integer(8));
        hdu.add_header("NAXIS", FITSValue::Integer(0));
        hdu.headers.push(crate::hdu::headers::FITSHeader::end_hdu());
        assert!(hdu.validate_primary().is_ok());
    }

    #[test]
    fn validate_primary_fails_without_simple() {
        let mut hdu = HDU::init();
        hdu.add_header("BITPIX", FITSValue::Integer(8));
        assert!(hdu.validate_primary().is_err());
    }

    #[test]
    fn validate_primary_fails_without_bitpix() {
        let mut hdu = HDU::init();
        hdu.add_header("SIMPLE", FITSValue::Logical(true));
        assert!(hdu.validate_primary().is_err());
    }

    #[test]
    fn validate_primary_fails_without_end() {
        let mut hdu = HDU::init();
        hdu.add_header("SIMPLE", FITSValue::Logical(true));
        hdu.add_header("BITPIX", FITSValue::Integer(8));
        hdu.add_header("NAXIS", FITSValue::Integer(0));
        // no END keyword added
        assert!(hdu.validate_primary().is_err());
    }
}
