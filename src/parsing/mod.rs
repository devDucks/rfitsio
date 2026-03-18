use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Seek, SeekFrom};

use crate::FITSFile;
use crate::fill_to_2880;
use crate::hdu::HDU;
use crate::hdu::data::FITSData;
use crate::hdu::headers::FITSHeader;

/// Parse a FITS file at `img_path` and return a `FITSFile` containing all HDUs.
///
/// Spec compliance:
/// - Header records are read in 80-byte chunks until the `END` keyword is found.
/// - After the last header record the reader is advanced to the next 2880-byte
///   boundary before the data unit begins.
/// - The data unit size is computed from the parsed `BITPIX` and `NAXISn` values:
///   `|BITPIX|/8 × NAXIS1 × NAXIS2 × … × NAXISn`.
/// - After the data unit the reader is again aligned to the next 2880-byte
///   boundary for the following HDU.
/// - All HDUs in the file are read (not just the first one).
pub fn parse(img_path: &str) -> std::io::Result<FITSFile> {
    let f = File::open(img_path)?;
    let mut reader = BufReader::new(f);
    let mut fits_file = FITSFile::new();

    loop {
        // ── Read header records ────────────────────────────────────────────

        let header_start = reader.stream_position()?;
        let mut headers: Vec<FITSHeader> = Vec::new();
        let mut record = [0u8; 80];

        // Values needed to compute the data unit size.
        let mut bitpix: i32 = 0;
        let mut naxis: usize = 0;
        // naxisn[0] = NAXIS1, naxisn[1] = NAXIS2, …
        let mut naxisn: Vec<usize> = Vec::new();

        loop {
            match reader.read_exact(&mut record) {
                Ok(_) => {}
                Err(_) => break,
            }

            let keyword = std::str::from_utf8(&record[0..8]).unwrap_or("").trim_end();

            // Parse scalar keywords needed for data-unit sizing.
            // Values occupy bytes 11–30 of the record (positions 10–29).
            if record[8] == b'=' {
                let raw = std::str::from_utf8(&record[10..80]).unwrap_or("");
                // Strip any inline comment after ` / `.
                let val_str = raw.split('/').next().unwrap_or("").trim();

                match keyword {
                    "BITPIX" => {
                        bitpix = val_str.parse().unwrap_or(0);
                    }
                    "NAXIS" => {
                        naxis = val_str.parse().unwrap_or(0);
                        naxisn = vec![0; naxis];
                    }
                    k if k.starts_with("NAXIS") => {
                        if let Ok(axis_idx) = k[5..].parse::<usize>() {
                            if axis_idx >= 1 && axis_idx <= naxis {
                                naxisn[axis_idx - 1] = val_str.parse().unwrap_or(0);
                            }
                        }
                    }
                    _ => {}
                }
            }

            let is_end = &record[0..3] == b"END";
            headers.push(FITSHeader::new_raw(&record[0..10], &record[10..80]));

            if is_end {
                break;
            }
        }

        if headers.is_empty() {
            break;
        }

        // ── Align to the next 2880-byte boundary after headers ─────────────
        //
        // The header section occupies an integer number of 2880-byte blocks.
        // We may have read a non-multiple number of 80-byte records, so skip
        // the remaining bytes in the current block.

        let bytes_read_so_far = (reader.stream_position()? - header_start) as i32;
        let header_padding = fill_to_2880(bytes_read_so_far) as i64;
        if header_padding > 0 {
            reader.seek(SeekFrom::Current(header_padding))?;
        }

        // ── Compute data unit size ─────────────────────────────────────────
        //
        // data_size = |BITPIX|/8 × NAXIS1 × NAXIS2 × … × NAXISn
        // If NAXIS = 0 there is no data unit.

        let data_size: usize = if naxis == 0 || naxisn.is_empty() {
            0
        } else {
            let pixel_count: usize = naxisn.iter().product();
            let bytes_per_pixel = (bitpix.unsigned_abs() / 8) as usize;
            pixel_count * bytes_per_pixel
        };

        // ── Read the data unit ─────────────────────────────────────────────

        let mut raw_data = vec![0u8; data_size];
        if data_size > 0 {
            reader.read_exact(&mut raw_data)?;
            // Align past the data block padding.
            let data_padding = fill_to_2880(data_size as i32) as i64;
            if data_padding > 0 {
                reader.seek(SeekFrom::Current(data_padding))?;
            }
        }

        // ── Build the HDU ──────────────────────────────────────────────────

        let mut hdu = HDU::init();
        hdu.headers = headers;
        // Store raw bytes directly; callers use crate::endian to decode them.
        hdu.data = FITSData { data: raw_data };
        fits_file.add_hdu(hdu);

        // ── Check whether another HDU follows ─────────────────────────────

        let mut peek = [0u8; 1];
        match reader.read_exact(&mut peek) {
            Err(_) => break, // EOF
            Ok(_) => {
                // Un-consume the byte so the next iteration reads it as part of
                // the next HDU's header.
                reader.seek(SeekFrom::Current(-1))?;
            }
        }
    }

    Ok(fits_file)
}
