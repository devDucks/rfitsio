use crate::hdu::HDU;

/// FITS binary table column type codes (TFORM).
#[derive(Debug, Clone, PartialEq)]
pub enum TFormType {
    Logical,        // L - boolean (T/F)
    Bit,            // X - packed bits
    UInt8,          // B - unsigned byte
    Int16,          // I - 16-bit integer
    Int32,          // J - 32-bit integer
    Int64,          // K - 64-bit integer
    Char,           // A - ASCII character(s)
    Float32,        // E - single-precision float
    Float64,        // D - double-precision float
    ComplexFloat32, // C - complex (2×f32)
    ComplexFloat64, // M - complex (2×f64)
    VarArrayInt32,  // P - variable-length array descriptor (2×i32 in data unit)
    VarArrayInt64,  // Q - variable-length array descriptor (2×i64 in data unit)
}

/// Parsed TFORM descriptor: repeat count + type code.
///
/// Format: `rT` where `r` is the optional repeat count (defaults to 1) and `T` is the type code.
/// For `X`, `r` is the number of bits. For `P`/`Q`, `r` is the maximum heap element count (a
/// hint only — the data unit always stores exactly one 8- or 16-byte descriptor per field).
#[derive(Debug, Clone)]
pub struct TForm {
    pub repeat: usize,
    pub type_code: TFormType,
}

impl TForm {
    /// Parse a TFORM string such as `"1E"`, `"4J"`, `"16A"`, `"8X"`.
    pub fn parse(s: &str) -> Option<TForm> {
        let s = s.trim();
        let alpha = s.find(|c: char| c.is_alphabetic())?;
        let repeat = if alpha == 0 {
            1
        } else {
            s[..alpha].parse::<usize>().ok()?
        };
        let type_code = match s[alpha..].to_ascii_uppercase().as_str() {
            "L" => TFormType::Logical,
            "X" => TFormType::Bit,
            "B" => TFormType::UInt8,
            "I" => TFormType::Int16,
            "J" => TFormType::Int32,
            "K" => TFormType::Int64,
            "A" => TFormType::Char,
            "E" => TFormType::Float32,
            "D" => TFormType::Float64,
            "C" => TFormType::ComplexFloat32,
            "M" => TFormType::ComplexFloat64,
            "P" => TFormType::VarArrayInt32,
            "Q" => TFormType::VarArrayInt64,
            _ => return None,
        };
        Some(TForm { repeat, type_code })
    }

    /// Bytes occupied by one field (one row, this column) in the data unit.
    pub fn bytes_per_field(&self) -> usize {
        match self.type_code {
            TFormType::Logical => self.repeat,
            TFormType::Bit => (self.repeat + 7) / 8,
            TFormType::UInt8 => self.repeat,
            TFormType::Int16 => self.repeat * 2,
            TFormType::Int32 => self.repeat * 4,
            TFormType::Int64 => self.repeat * 8,
            TFormType::Char => self.repeat,
            TFormType::Float32 => self.repeat * 4,
            TFormType::Float64 => self.repeat * 8,
            TFormType::ComplexFloat32 => self.repeat * 8,
            TFormType::ComplexFloat64 => self.repeat * 16,
            // P/Q always store exactly one (length, offset) descriptor per field,
            // regardless of repeat (which is only a heap-size hint).
            TFormType::VarArrayInt32 => 8,
            TFormType::VarArrayInt64 => 16,
        }
    }
}

/// A decoded field value for one row in a binary table column.
#[derive(Debug)]
pub enum ColumnValue {
    Logical(Vec<bool>),
    Bits(Vec<u8>),
    UInt8(Vec<u8>),
    Int16(Vec<i16>),
    Int32(Vec<i32>),
    Int64(Vec<i64>),
    Chars(String),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    ComplexFloat32(Vec<(f32, f32)>),
    ComplexFloat64(Vec<(f64, f64)>),
    /// (length, heap_offset) — actual data lives in the heap area after the main data unit.
    VarArrayInt32(i32, i32),
    /// (length, heap_offset) — actual data lives in the heap area after the main data unit.
    VarArrayInt64(i64, i64),
}

/// Metadata for one column in a binary table.
#[derive(Debug)]
pub struct ColumnDescriptor {
    pub name: String,
    pub unit: String,
    pub tform: TForm,
    pub(crate) row_offset: usize,
}

/// A view over a binary table HDU (XTENSION = 'BINTABLE').
///
/// Constructed via `BinaryTable::from_hdu`. Borrows the HDU for its lifetime.
#[derive(Debug)]
pub struct BinaryTable<'a> {
    hdu: &'a HDU,
    /// Number of rows (NAXIS2).
    pub nrows: usize,
    /// Bytes per row (NAXIS1).
    pub row_bytes: usize,
    columns: Vec<ColumnDescriptor>,
}

impl<'a> BinaryTable<'a> {
    /// Build a `BinaryTable` view from an HDU.
    /// Returns `None` if the HDU is not a BINTABLE extension or is missing required keywords.
    pub fn from_hdu(hdu: &'a HDU) -> Option<Self> {
        let xtension = header_str(hdu, "XTENSION")?;
        if xtension.to_ascii_uppercase() != "BINTABLE" {
            return None;
        }

        let row_bytes = header_int(hdu, "NAXIS1")? as usize;
        let nrows = header_int(hdu, "NAXIS2")? as usize;
        let tfields = header_int(hdu, "TFIELDS")? as usize;

        let mut columns = Vec::with_capacity(tfields);
        let mut offset = 0usize;

        for i in 1..=tfields {
            let tform_str = header_str(hdu, &format!("TFORM{}", i))?;
            let tform = TForm::parse(&tform_str)?;
            let name = header_str(hdu, &format!("TTYPE{}", i)).unwrap_or_default();
            let unit = header_str(hdu, &format!("TUNIT{}", i)).unwrap_or_default();
            let field_bytes = tform.bytes_per_field();
            columns.push(ColumnDescriptor {
                name,
                unit,
                tform,
                row_offset: offset,
            });
            offset += field_bytes;
        }

        Some(BinaryTable {
            hdu,
            nrows,
            row_bytes,
            columns,
        })
    }

    pub fn columns(&self) -> &[ColumnDescriptor] {
        &self.columns
    }

    /// Decode all rows for the named column. Returns `None` if the name is not found.
    pub fn column(&self, name: &str) -> Option<Vec<ColumnValue>> {
        let col = self.columns.iter().find(|c| c.name == name)?;
        Some(self.decode_column(col))
    }

    /// Decode all rows for the column at the given zero-based index.
    pub fn column_at(&self, index: usize) -> Option<Vec<ColumnValue>> {
        let col = self.columns.get(index)?;
        Some(self.decode_column(col))
    }

    fn decode_column(&self, col: &ColumnDescriptor) -> Vec<ColumnValue> {
        let data = &self.hdu.data.data;
        let field_bytes = col.tform.bytes_per_field();
        let mut values = Vec::with_capacity(self.nrows);

        for row in 0..self.nrows {
            let start = row * self.row_bytes + col.row_offset;
            let end = start + field_bytes;
            if end > data.len() {
                break;
            }
            values.push(decode_field(&data[start..end], &col.tform));
        }

        values
    }
}

fn decode_field(bytes: &[u8], tform: &TForm) -> ColumnValue {
    match tform.type_code {
        TFormType::Logical => ColumnValue::Logical(bytes.iter().map(|&b| b == b'T').collect()),
        TFormType::Bit => ColumnValue::Bits(bytes.to_vec()),
        TFormType::UInt8 => ColumnValue::UInt8(bytes.to_vec()),
        TFormType::Int16 => ColumnValue::Int16(
            bytes
                .chunks_exact(2)
                .map(|b| i16::from_be_bytes([b[0], b[1]]))
                .collect(),
        ),
        TFormType::Int32 => ColumnValue::Int32(
            bytes
                .chunks_exact(4)
                .map(|b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
        ),
        TFormType::Int64 => ColumnValue::Int64(
            bytes
                .chunks_exact(8)
                .map(|b| i64::from_be_bytes(b.try_into().unwrap()))
                .collect(),
        ),
        TFormType::Char => ColumnValue::Chars(
            std::str::from_utf8(bytes)
                .unwrap_or("")
                .trim_end_matches('\0')
                .to_string(),
        ),
        TFormType::Float32 => ColumnValue::Float32(
            bytes
                .chunks_exact(4)
                .map(|b| f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
        ),
        TFormType::Float64 => ColumnValue::Float64(
            bytes
                .chunks_exact(8)
                .map(|b| f64::from_be_bytes(b.try_into().unwrap()))
                .collect(),
        ),
        TFormType::ComplexFloat32 => ColumnValue::ComplexFloat32(
            bytes
                .chunks_exact(8)
                .map(|b| {
                    (
                        f32::from_be_bytes([b[0], b[1], b[2], b[3]]),
                        f32::from_be_bytes([b[4], b[5], b[6], b[7]]),
                    )
                })
                .collect(),
        ),
        TFormType::ComplexFloat64 => ColumnValue::ComplexFloat64(
            bytes
                .chunks_exact(16)
                .map(|b| {
                    (
                        f64::from_be_bytes(b[..8].try_into().unwrap()),
                        f64::from_be_bytes(b[8..].try_into().unwrap()),
                    )
                })
                .collect(),
        ),
        TFormType::VarArrayInt32 => ColumnValue::VarArrayInt32(
            i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        ),
        TFormType::VarArrayInt64 => ColumnValue::VarArrayInt64(
            i64::from_be_bytes(bytes[..8].try_into().unwrap()),
            i64::from_be_bytes(bytes[8..].try_into().unwrap()),
        ),
    }
}

/// Extract and trim a string value from a named header keyword.
fn header_str(hdu: &HDU, key: &str) -> Option<String> {
    let padded = format!("{:<8}", key);
    hdu.headers
        .iter()
        .find(|h| std::str::from_utf8(&h.key[..8]).ok().as_deref() == Some(&padded))
        .map(|h| {
            let raw = h.value_as_str().trim();
            let raw = raw.split('/').next().unwrap_or("").trim();
            if raw.starts_with('\'') {
                raw.trim_matches('\'').trim().to_string()
            } else {
                raw.to_string()
            }
        })
}

/// Extract an integer value from a named header keyword.
fn header_int(hdu: &HDU, key: &str) -> Option<i64> {
    let padded = format!("{:<8}", key);
    hdu.headers
        .iter()
        .find(|h| std::str::from_utf8(&h.key[..8]).ok().as_deref() == Some(&padded))
        .and_then(|h| {
            let raw = h.value_as_str().trim();
            raw.split('/')
                .next()
                .unwrap_or("")
                .trim()
                .parse::<i64>()
                .ok()
        })
}

#[cfg(test)]
mod tests {
    use super::{TForm, TFormType};

    #[test]
    fn parse_float32() {
        let t = TForm::parse("1E").unwrap();
        assert_eq!(t.repeat, 1);
        assert_eq!(t.type_code, TFormType::Float32);
        assert_eq!(t.bytes_per_field(), 4);
    }

    #[test]
    fn parse_repeated_int32() {
        let t = TForm::parse("4J").unwrap();
        assert_eq!(t.repeat, 4);
        assert_eq!(t.type_code, TFormType::Int32);
        assert_eq!(t.bytes_per_field(), 16);
    }

    #[test]
    fn parse_char() {
        let t = TForm::parse("16A").unwrap();
        assert_eq!(t.repeat, 16);
        assert_eq!(t.type_code, TFormType::Char);
        assert_eq!(t.bytes_per_field(), 16);
    }

    #[test]
    fn parse_bits() {
        let t = TForm::parse("9X").unwrap();
        assert_eq!(t.repeat, 9);
        assert_eq!(t.type_code, TFormType::Bit);
        assert_eq!(t.bytes_per_field(), 2); // ceil(9/8)
    }

    #[test]
    fn parse_var_array_int32() {
        let t = TForm::parse("512P").unwrap();
        assert_eq!(t.type_code, TFormType::VarArrayInt32);
        assert_eq!(t.bytes_per_field(), 8); // always 8, repeat is heap hint
    }

    #[test]
    fn parse_var_array_int64() {
        let t = TForm::parse("512Q").unwrap();
        assert_eq!(t.type_code, TFormType::VarArrayInt64);
        assert_eq!(t.bytes_per_field(), 16); // always 16
    }

    #[test]
    fn parse_implicit_repeat_one() {
        let t = TForm::parse("E").unwrap();
        assert_eq!(t.repeat, 1);
        assert_eq!(t.type_code, TFormType::Float32);
    }

    #[test]
    fn parse_unknown_type_returns_none() {
        assert!(TForm::parse("1Z").is_none());
    }

    #[test]
    fn complex_float32_bytes() {
        let t = TForm::parse("2C").unwrap();
        assert_eq!(t.bytes_per_field(), 16); // 2 × 8
    }

    #[test]
    fn complex_float64_bytes() {
        let t = TForm::parse("1M").unwrap();
        assert_eq!(t.bytes_per_field(), 16); // 1 × 16
    }
}
