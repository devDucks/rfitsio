pub mod data;
pub mod headers;

use crate::hdu::data::FITSData;
use crate::hdu::headers::FITSHeader;

pub struct HDU {
    pub headers: Vec<FITSHeader>,
    data: FITSData,
}

impl HDU {
    fn init() -> HDU {
        return HDU {
            headers: Vec::new(),
            data: FITSData::new(),
        };
    }

    fn raw_data(&self) -> &Vec<u8> {
        &self.data.data
    }

    fn add_data(&mut self, data: Vec<u8>) {
        self.data.add(data);
    }

    fn add_header(&mut self, key: &str, value: &str) {
        let header = FITSHeader::new(key, value);
        self.headers.push(header);
    }
}

#[cfg(test)]
mod tests {
    use crate::hdu::HDU;

    // Test that the HDU object is initialized correctly
    #[test]
    fn check_initialization() {
        let hdu = HDU::init();
        assert_eq!(hdu.headers.is_empty(), true);
        assert_eq!(hdu.raw_data().is_empty(), true);
    }

    // Make sure that add_data method actually adds the image
    // to the struct
    #[test]
    fn check_raw_data() {
        let mut hdu = HDU::init();
        hdu.add_data(vec![9; 5]);
        assert_eq!(hdu.raw_data()[0..5], [9, 9, 9, 9, 9]);
        assert_eq!(hdu.raw_data().len(), 2880);
    }

    // Make sure that add_headermethod actually adds the header
    // to `headers`
    #[test]
    fn check_adding_header() {
        let mut hdu = HDU::init();
        hdu.add_header("NEW", "FOO");
        let header = &hdu.headers[0];
        let key = header.key_as_str();
        let value = header.value_as_str();

        assert_eq!(key.contains("NEW"), true);
        assert_eq!(value.contains("FOO"), true);
    }
}
