use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::str;

fn main() -> std::io::Result<()> {
    let mut f = File::open("/home/matt/Downloads/502nmos.fits")?;
    let metadata = fs::metadata("/home/matt/Downloads/502nmos.fits")?;
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    let mut index: usize = 0;
    let mut keyword = "";

    while keyword != "END     " {
	keyword = str::from_utf8(&buffer[index..index + 8]).unwrap();
	let content = str::from_utf8(&buffer[index+10..index+80]).unwrap();
	//println!("{keyword}: {content}");
	index += 80;
    }

    println!("{:?}", &buffer[index+80..index+16000]);

    
    Ok(())
}
