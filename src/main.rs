use rfitsio::parsing::parse;

fn main() -> std::io::Result<()> {
    parse("./FOCx38i0101t_c0f.fits").unwrap();
    Ok(())
}
