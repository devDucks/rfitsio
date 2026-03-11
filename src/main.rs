use rfitsio::parsing::parse;

fn main() -> std::io::Result<()> {
    let fits = parse("./FOCx38i0101t_c0f.fits")?;
    println!("Parsed {} HDU(s)", fits.hdus.len());
    for (i, hdu) in fits.hdus.iter().enumerate() {
        println!(
            "  HDU {}: {} header record(s), {} data byte(s)",
            i,
            hdu.headers.len(),
            hdu.raw_data().len()
        );
    }
    Ok(())
}
