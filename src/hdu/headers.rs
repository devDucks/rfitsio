const PADDING: u8 = 32;
const EQUAL_SIGN: u8 = 61;
const END_HEADER_KEY: [u8; 10] = [69, 78, 68, 32, 32, 32, 32, 32, 32, 32];

/// Typed FITS header value.
///
/// FITS encodes values with strict formatting rules:
/// - `Logical` → `T` or `F`, right-justified in columns 11–30.
/// - `Integer`  → right-justified in columns 11–30.
/// - `Float`    → scientific notation, right-justified in columns 11–30.
/// - `Text`     → enclosed in single quotes, padded to a minimum of 8 chars.
pub enum FITSValue {
    Logical(bool),
    Integer(i64),
    Float(f64),
    Text(String),
}

/// The representation of a FITS header record (80 bytes total):
/// - `key`   : 10 bytes — 8-char keyword + `=` + space (or all spaces for COMMENT/HISTORY/END)
/// - `value` : 70 bytes — encoded value and optional inline comment
pub struct FITSHeader {
    pub key: [u8; 10],
    pub value: [u8; 70],
}

/// Validate a FITS keyword:
/// - 1–8 characters
/// - Uppercase A–Z, digits 0–9, hyphen `-`, or underscore `_` only
/// - `END`, `COMMENT`, `HISTORY` are reserved and must use their own constructors
fn validate_keyword(key: &str) {
    assert!(
        !key.is_empty() && key.len() <= 8,
        "FITS keyword must be 1–8 characters, got {:?}",
        key
    );
    assert_ne!(
        key, "END",
        "END is reserved; use FITSHeader::end_hdu() instead"
    );
    assert_ne!(
        key, "COMMENT",
        "COMMENT records must be created with FITSHeader::comment()"
    );
    assert_ne!(
        key, "HISTORY",
        "HISTORY records must be created with FITSHeader::history()"
    );
    for c in key.chars() {
        assert!(
            matches!(c, 'A'..='Z' | '0'..='9' | '-' | '_'),
            "invalid keyword character {:?}; only A\u{2013}Z, 0\u{2013}9, '-', '_' are allowed",
            c
        );
    }
}

/// Encode a `FITSValue` (and optional inline comment) into the 70-byte value field.
///
/// Value field layout (FITS spec §4.2):
/// - Logical / Integer / Float: right-justified in the first 20 bytes (columns 11–30).
/// - Text: `'<string>'` left-aligned, string padded to at least 8 chars inside quotes.
/// - Optional comment: ` / <text>` appended after the value, space-padded to 70 bytes.
fn encode_value(value: FITSValue, comment: Option<&str>) -> [u8; 70] {
    let val_str = match value {
        FITSValue::Logical(b) => format!("{:>20}", if b { "T" } else { "F" }),
        FITSValue::Integer(n) => format!("{:>20}", n),
        FITSValue::Float(f) => format!("{:>20.10E}", f),
        FITSValue::Text(ref s) => {
            assert!(
                s.len() <= 68,
                "FITS string value too long (max 68 chars), got {} chars",
                s.len()
            );
            // Internal single quotes are escaped by doubling them.
            let escaped = s.replace('\'', "''");
            // String content inside quotes must be at least 8 chars wide.
            let padded_len = escaped.len().max(8);
            format!("'{:<width$}'", escaped, width = padded_len)
        }
    };

    let full = match comment {
        Some(c) if !c.is_empty() => format!("{} / {}", val_str, c),
        _ => val_str,
    };

    let mut result = [PADDING; 70];
    for (i, b) in full.as_bytes().iter().take(70).enumerate() {
        result[i] = *b;
    }
    result
}

impl FITSHeader {
    /// Fast path for reading existing FITS files. The incoming slices must be
    /// exactly 10 bytes (key field) and 70 bytes (value field) respectively.
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

    /// Create a valued FITS header keyword.
    ///
    /// `key` must be 1–8 uppercase characters (A–Z, 0–9, `-`, `_`).
    /// The value is encoded according to FITS spec formatting rules.
    pub fn new(key: &str, value: FITSValue) -> FITSHeader {
        validate_keyword(key);
        let mut header_key = [PADDING; 10];
        for (i, b) in key.as_bytes().iter().enumerate() {
            header_key[i] = *b;
        }
        header_key[8] = EQUAL_SIGN;
        // header_key[9] stays PADDING (space)

        FITSHeader {
            key: header_key,
            value: encode_value(value, None),
        }
    }

    /// Create a valued FITS header keyword with an inline comment.
    pub fn new_with_comment(key: &str, value: FITSValue, comment: &str) -> FITSHeader {
        validate_keyword(key);
        let mut header_key = [PADDING; 10];
        for (i, b) in key.as_bytes().iter().enumerate() {
            header_key[i] = *b;
        }
        header_key[8] = EQUAL_SIGN;

        FITSHeader {
            key: header_key,
            value: encode_value(value, Some(comment)),
        }
    }

    /// Create a `COMMENT` record. COMMENT records have no `=` indicator;
    /// up to 70 chars of free-form ASCII text fill the value field.
    pub fn comment(text: &str) -> FITSHeader {
        let mut key = [PADDING; 10];
        for (i, b) in b"COMMENT".iter().enumerate() {
            key[i] = *b;
        }
        // Positions 8 and 9 remain spaces — no `=` for COMMENT records.

        let mut value = [PADDING; 70];
        for (i, b) in text.as_bytes().iter().take(70).enumerate() {
            value[i] = *b;
        }
        FITSHeader { key, value }
    }

    /// Create a `HISTORY` record. Like COMMENT, no `=` indicator is used.
    pub fn history(text: &str) -> FITSHeader {
        let mut key = [PADDING; 10];
        for (i, b) in b"HISTORY".iter().enumerate() {
            key[i] = *b;
        }

        let mut value = [PADDING; 70];
        for (i, b) in text.as_bytes().iter().take(70).enumerate() {
            value[i] = *b;
        }
        FITSHeader { key, value }
    }

    /// Create the mandatory `END` keyword that terminates a header section.
    /// Per FITS spec, END has no `=` and is padded with spaces to 80 bytes.
    pub fn end_hdu() -> FITSHeader {
        FITSHeader {
            key: END_HEADER_KEY,
            value: [PADDING; 70],
        }
    }

    pub fn key_as_str(&self) -> &str {
        std::str::from_utf8(&self.key).unwrap()
    }

    pub fn value_as_str(&self) -> &str {
        std::str::from_utf8(&self.value).unwrap()
    }

    pub fn as_str(&self) -> String {
        format!("{}{}", self.key_as_str(), self.value_as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::{FITSHeader, FITSValue};

    // --- key format ---

    #[test]
    fn correct_key_format() {
        let header = FITSHeader::new("FOO", FITSValue::Integer(1));
        assert_eq!(std::str::from_utf8(&header.key).unwrap(), "FOO     = ");
    }

    #[test]
    fn correct_key_length() {
        let header = FITSHeader::new("FOO", FITSValue::Integer(1));
        assert_eq!(header.key.len(), 10);
    }

    // --- value encoding ---

    #[test]
    fn integer_value_right_justified() {
        let header = FITSHeader::new("NAXIS", FITSValue::Integer(2));
        assert_eq!(&header.value_as_str()[..20], "                   2");
    }

    #[test]
    fn logical_true_right_justified() {
        let header = FITSHeader::new("SIMPLE", FITSValue::Logical(true));
        assert_eq!(&header.value_as_str()[..20], "                   T");
    }

    #[test]
    fn logical_false_right_justified() {
        let header = FITSHeader::new("SIMPLE", FITSValue::Logical(false));
        assert_eq!(&header.value_as_str()[..20], "                   F");
    }

    #[test]
    fn text_value_single_quoted_padded_to_8() {
        let header = FITSHeader::new("TELESCOP", FITSValue::Text("HST".into()));
        // Minimum 8 chars inside quotes → 'HST     '
        assert!(
            header.value_as_str().starts_with("'HST     '"),
            "got: {:?}",
            &header.value_as_str()[..12]
        );
    }

    #[test]
    fn text_value_internal_quote_doubled() {
        let header = FITSHeader::new("ORIGIN", FITSValue::Text("O'Hara".into()));
        assert!(
            header.value_as_str().starts_with("'O''Hara '"),
            "got: {:?}",
            &header.value_as_str()[..12]
        );
    }

    #[test]
    fn float_value_scientific_notation() {
        let header = FITSHeader::new("EXPTIME", FITSValue::Float(1.5e3));
        let trimmed = header.value_as_str()[..20].trim();
        assert!(
            trimmed.contains('E'),
            "expected scientific notation in {:?}",
            trimmed
        );
    }

    #[test]
    fn value_with_comment() {
        let header =
            FITSHeader::new_with_comment("BITPIX", FITSValue::Integer(16), "bits per pixel");
        assert!(
            header.value_as_str().contains("/ bits per pixel"),
            "got: {:?}",
            &header.value_as_str()[..40]
        );
    }

    #[test]
    fn value_field_is_70_bytes() {
        let header = FITSHeader::new("NAXIS1", FITSValue::Integer(1024));
        assert_eq!(header.value.len(), 70);
    }

    #[test]
    fn value_not_truncated_when_longer_than_key() {
        let header = FITSHeader::new("AB", FITSValue::Text("LONGERVAL".into()));
        assert!(
            header.value_as_str().contains("LONGERVAL"),
            "value was truncated: {:?}",
            &header.value_as_str()[..20]
        );
    }

    // --- COMMENT / HISTORY ---

    #[test]
    fn comment_record_has_no_equals() {
        let header = FITSHeader::comment("written by rfitsio");
        assert!(!header.key_as_str().contains('='));
        assert!(header.value_as_str().starts_with("written by rfitsio"));
    }

    #[test]
    fn history_record_has_no_equals() {
        let header = FITSHeader::history("calibrated 2024-01-01");
        assert!(!header.key_as_str().contains('='));
        assert!(header.value_as_str().starts_with("calibrated 2024-01-01"));
    }

    // --- END keyword ---

    #[test]
    fn check_end_header_constructed_correctly() {
        let header = FITSHeader::end_hdu();
        assert!(header.key_as_str().contains("END"));
        assert!(!header.key_as_str().contains('='));
        assert_eq!(header.value, [32; 70]);
    }

    // --- keyword validation ---

    #[test]
    #[should_panic(expected = "only A")]
    fn lowercase_keyword_rejected() {
        FITSHeader::new("simple", FITSValue::Logical(true));
    }

    #[test]
    #[should_panic(expected = "reserved")]
    fn end_keyword_rejected_via_new() {
        FITSHeader::new("END", FITSValue::Logical(true));
    }

    #[test]
    #[should_panic(expected = "1")]
    fn keyword_too_long_rejected() {
        FITSHeader::new("TOOLONGKEY", FITSValue::Integer(0));
    }

    // --- new_raw ---

    #[test]
    fn new_raw_built_correctly() {
        let key = "COOL    = ";
        let value = "          AWESOME VALUE  / COMMENT                                    ";
        let header = FITSHeader::new_raw(key.as_bytes(), value.as_bytes());
        assert_eq!(header.key_as_str(), key);
        assert_eq!(header.value_as_str(), value);
    }
}
