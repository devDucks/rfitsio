use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Seek, SeekFrom};

use crate::FITSFile;
use crate::fill_to_2880;
use crate::hdu::HDU;
use crate::hdu::data::FITSData;
use crate::hdu::headers::{FITSHeader, FITSValue};

/// Infer a `FITSValue` from the raw 70-byte value field of a record.
/// Used by `new_raw` calls in the parser where the type is not known ahead of time.
fn infer_value(has_equals: bool, val_bytes: &[u8]) -> FITSValue {
    if !has_equals {
        let text = std::str::from_utf8(val_bytes)
            .unwrap_or("")
            .trim_end()
            .to_string();
        return FITSValue::Text(text);
    }
    let raw = std::str::from_utf8(val_bytes).unwrap_or("");
    let val_str = raw.split('/').next().unwrap_or("").trim();
    if val_str == "T" || val_str == "F" {
        FITSValue::Logical(val_str == "T")
    } else if val_str.starts_with('\'') {
        let inner = val_str.trim_matches('\'').replace("''", "'");
        FITSValue::Text(inner.trim_end().to_string())
    } else if val_str.contains('.') || val_str.to_ascii_uppercase().contains('E') {
        FITSValue::Float(val_str.parse().unwrap_or(0.0))
    } else {
        FITSValue::Integer(val_str.parse().unwrap_or(0))
    }
}

/// Parse a FITS file at `img_path` and return a `FITSFile` containing all HDUs.
///
/// Spec compliance:
/// - Header records are read in 80-byte chunks until the `END` keyword is found.
/// - After the last header record the reader is advanced to the next 2880-byte
///   boundary before the data unit begins.
/// - The data unit size is computed as `GCOUNT × (PCOUNT + NAXIS1 × … × NAXISn) × |BITPIX|/8`,
///   covering both image and binary-table (BINTABLE) extensions.
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
        let mut pcount: usize = 0;
        let mut gcount: usize = 1;
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
                    "PCOUNT" => {
                        pcount = val_str.parse().unwrap_or(0);
                    }
                    "GCOUNT" => {
                        gcount = val_str.parse().unwrap_or(1);
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

            let is_end = keyword == "END";
            let kind = infer_value(record[8] == b'=', &record[10..80]);
            headers.push(FITSHeader::new_raw(&record[0..10], &record[10..80], kind));

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
        // Per FITS standard:
        // data_size = GCOUNT × (PCOUNT + NAXIS1 × … × NAXISn) × |BITPIX| / 8
        // For images GCOUNT=1 and PCOUNT=0; for BINTABLE PCOUNT holds the heap size.

        let data_size: usize = if naxis == 0 || naxisn.is_empty() {
            0
        } else {
            let pixel_count: usize = naxisn.iter().product();
            let bytes_per_pixel = (bitpix.unsigned_abs() / 8) as usize;
            gcount * (pcount + pixel_count * bytes_per_pixel)
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

#[cfg(test)]
mod tests {
    use super::parse;
    use std::io::Write;

    fn logical_record(keyword: &str, v: bool) -> [u8; 80] {
        let mut rec = [b' '; 80];
        for (i, &b) in keyword.as_bytes().iter().enumerate() {
            rec[i] = b;
        }
        rec[8] = b'=';
        let val = format!("{:>20}", if v { "T" } else { "F" });
        rec[10..30].copy_from_slice(val.as_bytes());
        rec
    }

    fn integer_record(keyword: &str, n: i64) -> [u8; 80] {
        let mut rec = [b' '; 80];
        for (i, &b) in keyword.as_bytes().iter().enumerate() {
            rec[i] = b;
        }
        rec[8] = b'=';
        let val = format!("{:>20}", n);
        rec[10..30].copy_from_slice(val.as_bytes());
        rec
    }

    fn text_record(keyword: &str, text: &str) -> [u8; 80] {
        let mut rec = [b' '; 80];
        for (i, &b) in keyword.as_bytes().iter().enumerate() {
            rec[i] = b;
        }
        rec[8] = b'=';
        let quoted = format!("'{:<8}'", text);
        for (i, &b) in quoted.as_bytes().iter().enumerate() {
            rec[10 + i] = b;
        }
        rec
    }

    fn end_record() -> [u8; 80] {
        let mut rec = [b' '; 80];
        rec[..3].copy_from_slice(b"END");
        rec
    }

    /// Build a minimal FITS primary HDU (NAXIS=0, no data) with blank-keyword
    /// records interspersed before and after a real keyword.
    fn fits_bytes_with_blanks() -> Vec<u8> {
        let records: Vec<[u8; 80]> = vec![
            logical_record("SIMPLE", true),
            integer_record("BITPIX", 8),
            integer_record("NAXIS", 0),
            [b' '; 80],                        // blank keyword before INSTRUME
            text_record("INSTRUME", "TEST"),   // keyword that must survive blank
            [b' '; 80],                        // blank keyword before END
            end_record(),
        ];

        let mut bytes: Vec<u8> = records.into_iter().flatten().collect();
        // Pad to the next 2880-byte boundary.
        let rem = bytes.len() % 2880;
        if rem != 0 {
            bytes.extend(std::iter::repeat(b' ').take(2880 - rem));
        }
        bytes
    }

    #[test]
    fn parser_reads_past_blank_keyword_records() {
        let data = fits_bytes_with_blanks();
        let path = std::env::temp_dir().join("rfitsio_blank_kw_test.fits");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&data).unwrap();
        }

        let fits = parse(path.to_str().unwrap()).expect("parse failed");
        std::fs::remove_file(&path).ok();

        assert_eq!(fits.hdus.len(), 1);

        let headers = &fits.hdus[0].headers;
        // SIMPLE, BITPIX, NAXIS, blank, INSTRUME, blank, END = 7 records
        assert_eq!(headers.len(), 7, "wrong record count — parser may have stopped early");

        let found_instrume = headers.iter().any(|h| h.key() == "INSTRUME");
        assert!(
            found_instrume,
            "INSTRUME not found — parser stopped at blank keyword record"
        );

        assert_eq!(headers.last().unwrap().key(), "END");
    }

    #[test]
    fn end_keyword_not_confused_with_keyword_starting_with_end() {
        // Build a header where a real keyword starts with "END" (e.g. "ENDTIME")
        // followed by the actual END record.  The parser must not terminate early.
        let records: Vec<[u8; 80]> = vec![
            logical_record("SIMPLE", true),
            integer_record("BITPIX", 8),
            integer_record("NAXIS", 0),
            integer_record("ENDTIME", 42), // starts with "END" — must not stop here
            end_record(),
        ];

        let mut bytes: Vec<u8> = records.into_iter().flatten().collect();
        let rem = bytes.len() % 2880;
        if rem != 0 {
            bytes.extend(std::iter::repeat(b' ').take(2880 - rem));
        }

        let path = std::env::temp_dir().join("rfitsio_endtime_test.fits");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&bytes).unwrap();
        }

        let fits = parse(path.to_str().unwrap()).expect("parse failed");
        std::fs::remove_file(&path).ok();

        let headers = &fits.hdus[0].headers;
        // SIMPLE, BITPIX, NAXIS, ENDTIME, END = 5 records
        assert_eq!(headers.len(), 5, "wrong record count — parser may have stopped at ENDTIME");

        let found_endtime = headers.iter().any(|h| h.key() == "ENDTIME");
        assert!(found_endtime, "ENDTIME not found — parser stopped at keyword starting with END");
    }
}
