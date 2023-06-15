use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Seek};

use crate::hdu::headers::FITSHeader;

pub fn parse(img_path: &str) -> std::io::Result<()> {
    let mut f = File::open(img_path).unwrap();
    let mut f_c = f.try_clone().unwrap();
    let mut reader = BufReader::new(f);
    let mut headers: Vec<FITSHeader> = Vec::new();

    println!("File is {} bytes long", f_c.metadata().unwrap().len());
    let mut abc = [10, 10];

    while let Ok(_) = reader.read_exact(&mut abc) {
        // The first part requires to go through HEADERS
        println!("Cursor ar position {}", reader.stream_position()?);
        reader.seek_relative(-2);
        println!("Cursor ar position {}", reader.stream_position()?);
        let mut header = [0; 80];

        // Check until the END header and build them
        while &header[0..3] != b"END" {
            reader.read_exact(&mut header).unwrap();
            // println!("{:?}", &header);
            headers.push(FITSHeader::from_slices(
                &header[0..10],
                &header[10..header.len()],
            ));
            println!("{}", std::str::from_utf8(&header).unwrap());
        }

        let after_headers = reader.stream_position().unwrap();

        // Find how many bytes we need to add to close the headers as multiple of 2880
        let zeros = crate::fill_to_2880(after_headers.try_into().unwrap());
        println!(
            "The headers are {} bytes long",
            &after_headers + zeros as u64
        );

        // Seek to the first byte after the headers
        reader.seek_relative(zeros as i64);

        let after_headers_2 = reader.stream_position().unwrap();

        println!(
            "After header we are at {}, we should be at {}",
            &after_headers_2,
            &after_headers + zeros as u64
        );

        // Check if we need to pull any data, this is given by NAXIS
        let mut num_of_axis;

        for header in &headers {
            // Iterate through headers and find NAXIS, if is > 0 then pull NAXISn values
            if &header.key_as_str()[0..9] == "NAXIS   =" {
                num_of_axis = header.value_as_str();
                println!("# of axis: {}", &num_of_axis);
            }
        }

        let mut check_buf = [0; 1];
        if let Ok(_) = reader.read_exact(&mut check_buf) {
            reader.seek_relative(-2);
            println!("After header CONTENT: {:?}", &check_buf);

            if &check_buf != b"S" || &check_buf != b"X" {
                println!("CHECK: {:?}", &check_buf);
                // we are done reading HEADERS, fetch now pixel data
                let mut image: [u8; (32 * 1024 * 1024) / 8 - 1] = [0; (32 * 1024 * 1024) / 8 - 1];
                reader.read_exact(&mut image).unwrap();

                //println!("Image: {:?}", &image);
                let after_image = reader.stream_position().unwrap();

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
