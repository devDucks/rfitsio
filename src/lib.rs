use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Seek};

mod data;
mod hdu;
mod headers;

pub fn small_read(img_path: &str) -> std::io::Result<()> {
    let f = File::open(img_path)?;
    //let mut f = File::open("/home/matt/data/repos/rfitsio/FOCx38i0101t_c0f.fits")?;
    let mut reader = BufReader::new(f);
    let before = reader.stream_position()?;
    let mut header = [0; 80];

    while &header[0..3] != b"END" {
        reader.read_exact(&mut header)?;
        println!("header: {:?}", &header);
        println!("{}", std::str::from_utf8(&header).unwrap());
    }

    let after_headers = reader.stream_position()?;
    let mut _buf = Vec::new();

    println!("The HDU was {} bytes long", &after_headers - &before);

    reader.read_until(0, &mut _buf)?;

    let after_headers_2 = reader.stream_position()?;

    println!("After header we are at {}", &after_headers_2 - &before);

    // we are done reading HEADERS, fetch now pixel data
    //let mut image: [u8; 1024*1024]  = [0; 1024 * 1024];
    let mut image: [u8; 1936 * 1096 - 1] = [0; 1936 * 1096 - 1];
    reader.read_exact(&mut image)?;

    let after_image = reader.stream_position()?;

    println!(
        "The Image was {} bytes long",
        &after_image - &after_headers_2
    );

    // next HDU
    let mut hdu: Vec<u8> = Vec::new();
    let size = reader.read_to_end(&mut hdu)?;
    println!("Read {} bytes", size);
    //println!("Remaining {}", std::str::from_utf8(&hdu[0..80]).unwrap());
    Ok(())
}

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
