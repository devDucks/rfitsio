const NULL_BYTE: u8 = 0;

use crate::fill_to_2880;

pub struct FITSData {
    pub data: Vec<u8>,
}

impl FITSData {
    pub fn new() -> FITSData {
        FITSData { data: Vec::new() }
    }

    /// Store raw pixel bytes and pad to the next 2880-byte boundary with null bytes.
    pub fn add(&mut self, mut data: Vec<u8>) {
        let bytes_to_add = fill_to_2880(data.len() as i32);
        for _ in 0..bytes_to_add {
            data.push(NULL_BYTE);
        }
        self.data = data;
    }

    /// Store `u8` pixels (BITPIX = 8).  No byte-swapping needed for single bytes.
    pub fn add_u8(&mut self, values: &[u8]) {
        self.add(values.to_vec());
    }

    /// Store `i16` pixels (BITPIX = 16) in FITS big-endian byte order.
    pub fn add_i16(&mut self, values: &[i16]) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_be_bytes()).collect();
        self.add(bytes);
    }

    /// Store `i32` pixels (BITPIX = 32) in FITS big-endian byte order.
    pub fn add_i32(&mut self, values: &[i32]) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_be_bytes()).collect();
        self.add(bytes);
    }

    /// Store `i64` pixels (BITPIX = 64) in FITS big-endian byte order.
    pub fn add_i64(&mut self, values: &[i64]) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_be_bytes()).collect();
        self.add(bytes);
    }

    /// Store `f32` pixels (BITPIX = -32) in FITS big-endian byte order.
    pub fn add_f32(&mut self, values: &[f32]) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_be_bytes()).collect();
        self.add(bytes);
    }

    /// Store `f64` pixels (BITPIX = -64) in FITS big-endian byte order.
    pub fn add_f64(&mut self, values: &[f64]) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_be_bytes()).collect();
        self.add(bytes);
    }
}

#[cfg(test)]
mod tests {
    use crate::hdu::data::FITSData;

    // 800×600 px image: 480_000 bytes, not a multiple of 2880 → 960 padding bytes
    #[test]
    fn correct_unit_length() {
        let image = vec![255; 800 * 600];
        let mut fits_data = FITSData::new();
        fits_data.add(image);
        assert_eq!(fits_data.data.len(), 480_960);
        assert_eq!(fits_data.data[480_000], 0u8);
    }

    #[test]
    fn no_bytes_added() {
        let image = vec![255; 2880];
        let mut fits_data = FITSData::new();
        fits_data.add(image);
        assert_eq!(fits_data.data.len(), 2880);
    }

    #[test]
    fn add_i16_stores_big_endian() {
        let pixels: Vec<i16> = vec![256, -1];
        let mut fits_data = FITSData::new();
        fits_data.add_i16(&pixels);
        // 256 big-endian = [0x01, 0x00]
        assert_eq!(fits_data.data[0], 0x01);
        assert_eq!(fits_data.data[1], 0x00);
        // -1 big-endian = [0xFF, 0xFF]
        assert_eq!(fits_data.data[2], 0xFF);
        assert_eq!(fits_data.data[3], 0xFF);
    }

    #[test]
    fn add_f32_stores_big_endian() {
        let pixels: Vec<f32> = vec![1.0_f32];
        let mut fits_data = FITSData::new();
        fits_data.add_f32(&pixels);
        // 1.0f32 big-endian IEEE 754 = [0x3F, 0x80, 0x00, 0x00]
        assert_eq!(&fits_data.data[..4], &[0x3F, 0x80, 0x00, 0x00]);
    }
}
