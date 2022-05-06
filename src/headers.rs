const PADDING: u8 = 32;
const EQUAL_SIGN: u8 = 61;

pub struct FITSHeader {
    pub key: [u8; 10],
    pub value: [u8; 70],
}

impl FITSHeader {
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
    use crate::headers::FITSHeader;

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
}
