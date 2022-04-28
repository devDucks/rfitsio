use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Seek};

pub fn small_read() -> std::io::Result<()> {
    let f = File::open("/home/matt/data/repos/asi-rs/zwo001.fits")?;
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
/// to adhere to the FITS specification
pub fn fill_to_2880(n: i32) -> i32 {
    let division = n % 2880;

    let bytes_to_add = match division {
        0 => 0,
        m => 2880 - m,
    };
    bytes_to_add
}

#[cfg(test)]
mod test {
    use crate::fill_to_2880;

    #[test]
    fn test_2880_bytes_not_add_any() {
        assert_eq!(fill_to_2880(2880), 0)
    }

    #[test]
    fn test_5760_bytes_not_add_any() {
        assert_eq!(fill_to_2880(5760), 0)
    }

    #[test]
    fn test_2885_bytes_add_5() {
        assert_eq!(fill_to_2880(2885), 2875)
    }

    #[test]
    fn test_1936_1096_bytes_not_add_any() {
        assert_eq!(fill_to_2880(1936 * 1096), 704)
    }
}
