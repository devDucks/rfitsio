use rfitsio::small_read;

fn main() -> std::io::Result<()> {
    small_read("./FOCx38i0101t_c0f.fits")?;
    Ok(())
}
