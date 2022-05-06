//! Fast and easy queue abstraction.
//!
//! Provides an abstraction over a queue.  When the abstraction is used
//! there are these advantages:
//! - Fast
//! - [`Easy`]
//!
//! [`Easy`]: http://thatwaseasy.example.com

/// This module makes it easy.

const PADDING: u8 = 32;
const EQUAL_SIGN: u8 = 61;
const END_HEADER_KEY: [u8; 10] = [69, 78, 68, 32, 32, 32, 32, 32, 32, 32];

/// The representation of a FITS header, this is
/// spec compliant.
pub struct FITSHeader {
    pub key: [u8; 10],
    pub value: [u8; 70],
}

impl FITSHeader {
    /// Use this method if you are looking for extra performance
    /// or you are reading an existing FITS file.
    /// DRAGONS AHEAD: This method usese memcpy so the incoming array
    /// must be the same size of the one where it will be copied; if
    /// this contract is not mantained the process will panic.
    pub fn new_raw(key: &[u8], value: &[u8]) -> FITSHeader {
        let mut header_key = [0; 10];
        let mut header_value = [0; 70];

        header_key.copy_from_slice(key);
        header_value.copy_from_slice(value);

        FITSHeader {
            key: header_key,
            value: header_value,
        }
    }

    pub fn new(key: &str, value: &str) -> FITSHeader {
        match key.len() {
            1..=8 => (),
            _ => panic!("KEY for the header is too long"),
        };

        match value.len() {
            1..=70 => (),
            _ => panic!("VALUE for the header is too long"),
        };

        let mut header_key = [0; 10];
        let mut header_value = [0; 70];

        for (i, char) in key.as_bytes().into_iter().enumerate() {
            header_key[i] = *char;
        }

        for i in key.len()..10 {
            header_key[i as usize] = PADDING;
        }

        header_key[8] = EQUAL_SIGN;

        for (i, char) in value.as_bytes().into_iter().enumerate() {
            header_value[i] = *char;
        }

        for i in key.len()..70 {
            header_value[i as usize] = PADDING;
        }

        FITSHeader {
            key: header_key,
            value: header_value,
        }
    }

    /// With this method the HDU ending header will be constructed,
    /// according to the FITS spec it started with END, doesn't
    /// contain an = sign and is padded till the end of the 80 bytes
    /// long hdu block.
    pub fn end_hdu() -> FITSHeader {
        let value = [PADDING; 70];

        FITSHeader {
            key: END_HEADER_KEY,
            value: value,
        }
    }

    pub fn key_as_str(&self) -> &str {
        return std::str::from_utf8(&self.key).unwrap();
    }

    pub fn value_as_str(&self) -> &str {
        return std::str::from_utf8(&self.value).unwrap();
    }

    pub fn as_str(&self) -> String {
        let repr = format!("{}{}", self.key_as_str(), self.value_as_str());
        return repr;
    }
}

#[cfg(test)]
mod tests {
    use crate::hdu::headers::FITSHeader;

    #[test]
    fn correct_key_format() {
        let header = FITSHeader::new("FOO", "BAR");
        assert_eq!(std::str::from_utf8(&header.key).unwrap(), "FOO     = ");
    }

    #[test]
    fn correct_key_length() {
        let header = FITSHeader::new("FOO", "BAR");
        assert_eq!(header.key.len(), 10);
    }

    #[test]
    fn correct_value_format() {
        let header = FITSHeader::new("FOO", "BAR");
        assert_eq!(
            std::str::from_utf8(&header.value).unwrap(),
            "BAR                                                                   "
        );
    }

    #[test]
    fn correct_value_length() {
        let header = FITSHeader::new("FOO", "BAR");
        assert_eq!(header.value.len(), 70);
    }

    #[test]
    fn correct_key_representation() {
        let header = FITSHeader::new("FOO", "BAR");
        let key = header.key_as_str();

        assert_eq!(key.contains("FOO"), true);
        assert_eq!(key.contains("="), true);
    }

    #[test]
    fn correct_value_representation() {
        let header = FITSHeader::new("FOO", "BAR");
        let value = header.value_as_str();

        assert_eq!(value.contains("BAR"), true);
    }

    #[test]
    fn correct_full_representation() {
        let header = FITSHeader::new("FOO", "BAR");
        let full_header = header.as_str();

        assert_eq!(full_header.contains("FOO"), true);
        assert_eq!(full_header.contains("BAR"), true);
        assert_eq!(full_header.contains("="), true);
    }

    #[test]
    fn new_raw_built_correctly() {
        let key = "COOL    = ";
        let value = "          AWESOME VALUE  / COMMENT                                    ";
        let header = FITSHeader::new_raw(key.as_bytes(), value.as_bytes());
        let header_value = header.value_as_str();
        let header_key = header.key_as_str();

        assert_eq!(header_key, key);
        assert_eq!(header_value, value);
    }

    #[test]
    fn check_end_header_constructed_correctly() {
        let header = FITSHeader::end_hdu();
        let header_key = header.key_as_str();

        assert_eq!(header_key.contains("END"), true);
        assert_eq!(header_key.contains("="), false);
        assert_eq!(header.value, [32; 70]);
    }
}
