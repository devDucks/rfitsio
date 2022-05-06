const PADDING: u8 = 32;

use crate::fill_to_2880;

pub struct FITSData {
    pub data: Vec<u8>,
}

impl FITSData {
    pub fn new() -> FITSData {
        return FITSData { data: Vec::new() };
    }

    pub fn add(&mut self, mut data: Vec<u8>) {
        let bytes_to_add = fill_to_2880(data.len() as i32);

        for _ in 0..bytes_to_add {
            data.push(PADDING);
        }

        self.data = data;
    }
}

#[cfg(test)]
mod tests {
    use crate::hdu::data::FITSData;

    // In this test we are simulating adding an image which is
    // 480_000 (800px*600px) bytes long. 480_000 is not a multiple
    // of 2880 so we expect to add 960 additional bytes to fulfill
    // the FITS spec
    #[test]
    fn correct_unit_length() {
        let image = vec![255; 800 * 600];
        let mut fits_data = FITSData::new();
        fits_data.add(image);
        assert_eq!(fits_data.data.len(), 480_960);
    }

    #[test]
    fn no_bytes_added() {
        let image = vec![255; 2880];
        let mut fits_data = FITSData::new();
        fits_data.add(image);
        assert_eq!(fits_data.data.len(), 2880);
    }
}
