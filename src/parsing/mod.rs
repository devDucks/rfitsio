use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Seek};

pub fn parse(img_path: &str) -> std::io::Result<()> {
    let f = File::open(img_path)?;
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
