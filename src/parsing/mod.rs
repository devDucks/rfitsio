use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Seek};

use crate::hdu::headers::FITSHeader;

pub fn parse(img_path: &str) -> std::io::Result<()> {
    let mut f = File::open(img_path)?;
    let mut f_c = f.try_clone()?;
    let mut reader = BufReader::new(f);

    println!("FIle is {} bytes long", f_c.metadata().unwrap().len());

    while let Ok(_) = reader.read_exact(&mut [0, 1]) {
        println!("Cursor ar position {}", reader.stream_position()?);
        reader.seek_relative(-2);
        println!("Cursor ar position {}", reader.stream_position()?);
        let mut header = [0; 80];
        let mut headers: Vec<FITSHeader> = Vec::new();

        while &header[0..3] != b"END" {
            reader.read_exact(&mut header)?;
            // println!("{:?}", &header);
            headers.push(FITSHeader::from_slices(
                &header[0..10],
                &header[10..header.len()],
            ));
            println!("{}", std::str::from_utf8(&header).unwrap());
        }

        let after_headers = reader.stream_position()?;

        // Find how many bytes we need to add to close the headers as multiple of 2880
        let zeros = crate::fill_to_2880(after_headers.try_into().unwrap());
        println!(
            "The headers are {} bytes long",
            &after_headers + zeros as u64
        );

        // Seek to the first byte after the headers
        reader.seek_relative(zeros as i64);

        let after_headers_2 = reader.stream_position()?;

        println!("After header we are at {}", &after_headers_2);

        let mut check_buf = [0; 1];
        if let Ok(_) = reader.read_exact(&mut check_buf) {
            reader.seek_relative(-2);
            println!("After header CONTENT: {:?}", &check_buf);

            if &check_buf != b"S" || &check_buf != b"X" {
                // we are done reading HEADERS, fetch now pixel data
                let mut image: [u8; (32 * 1024 * 1024) / 8 - 1] = [0; (32 * 1024 * 1024) / 8 - 1];
                reader.read_exact(&mut image)?;

                //println!("Image: {:?}", &image);
                let after_image = reader.stream_position()?;

                println!(
                    "The Image was {} bytes long",
                    &after_image - &after_headers_2
                );

                let zeros = crate::fill_to_2880(reader.stream_position()? as i32);
                reader.seek_relative(zeros as i64);
                println!("After header we are at {}", reader.stream_position()?);
            }
        }
    }

    Ok(())
}
