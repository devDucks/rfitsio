# rfitsio

Pure Rust implementation of CFITSIO — a library for reading and writing FITS files used in astronomical data storage.

## Status

Early-stage / experimental. Intended for developers and testing, with no external dependencies.

## Usage

```toml
[dependencies]
rfitsio = "0.3.0"
```

```rust
use rfitsio::{FITSFile, HDU};

let mut file = FITSFile::new();
let mut hdu = HDU::init();
hdu.add_header("SIMPLE", "T");
hdu.add_data(pixel_bytes);
file.add_hdu(hdu);
file.write_to_file("output.fits")?;
```

## Features

- Build and serialize FITS files (HDU headers + image data)
- FITS-compliant 2880-byte block alignment
- Big-endian encoding/decoding helpers (`rfitsio::endian`)
- Basic FITS file parser

## Build

```bash
cargo build
cargo test
```

## License

GPL-3.0-or-later
