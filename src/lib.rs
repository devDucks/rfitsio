use hdu::data::FITSData;

pub mod hdu;
pub mod parsing;

/// Return a number indicating how many bytes should be added as padding
/// to adhere to the FITS specification for a given block of header/data unit.
/// The FITS spec says that any header/data must be 2880 bytes long or a multiple
/// of 2880, so if it is less or more than 2880 and is not a multiple of it, the
/// remaining space must be filled with space chars (62)
pub fn fill_to_2880(n: i32) -> i32 {
    let division = n % 2880;

    let bytes_to_add = match division {
        0 => 0,
        m => 2880 - m,
    };
    bytes_to_add
}

pub struct FITSImage {
    hdus: Vec<hdu::HDU>,
}

#[cfg(test)]
mod tests {
    use crate::fill_to_2880;

    // Assert that if we have a HDU which is 2880 bytes long
    // we are not going to add any padding
    #[test]
    fn check_2880_bytes_hdu_not_add_any() {
        assert_eq!(fill_to_2880(2880), 0)
    }

    // Assert that if we have a HDU which is 5760 bytes long
    // we are not going to add any padding (2880 * 2)
    #[test]
    fn check_5760_bytes_hdu_not_add_any() {
        assert_eq!(fill_to_2880(5760), 0)
    }

    // Assert that if we have a HDU which is 2885 bytes long
    // we are going to add 5 bytes padding
    #[test]
    fn test_2885_bytes_hdu_add_5() {
        assert_eq!(fill_to_2880(2885), 2875)
    }

    // Assert that if we have a HDU which is 1936*1096 bytes long
    // we are going to add 704 bytes of padding
    #[test]
    fn test_1936_1096_bytes_hdu_not_add_any() {
        assert_eq!(fill_to_2880(1936 * 1096), 704)
    }
}
