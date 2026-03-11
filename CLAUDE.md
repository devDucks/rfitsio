# CLAUDE.md — rfitsio

## Project Overview

**rfitsio** is a pure Rust implementation of CFITSIO (Flexible Image Transport System I/O), a library for reading and writing FITS (Flexible Image Transport System) files used in astronomical data storage.

- **Language:** Rust (Edition 2021)
- **Type:** Library crate + binary
- **Version:** 0.2.0
- **Status:** Early-stage / experimental — intended for developers and testing
- **Dependencies:** Zero external crates (pure Rust standard library only)

## Repository Structure

```
rfitsio/
├── Cargo.toml              # Package manifest (no external deps)
├── Makefile                # Documentation build helpers
├── README.md               # Project overview
├── code_of_conduct.md      # Community guidelines
├── FOCx38i0101t_c0f.fits   # Test FITS file (4.2 MB, required by main.rs)
└── src/
    ├── lib.rs              # Library root: module exports + fill_to_2880()
    ├── main.rs             # Binary: parses FOCx38i0101t_c0f.fits
    ├── hdu/
    │   ├── mod.rs          # HDU struct (headers + data container)
    │   ├── hdu.rs          # (duplicate/unused — see known issues)
    │   ├── headers.rs      # FITSHeader struct: FITS-compliant header formatting
    │   └── data.rs         # FITSData struct: image data with 2880-byte padding
    └── parsing/
        └── mod.rs          # File parser (proof-of-concept, hard-coded)
```

## Build & Run

```bash
# Build the library and binary
cargo build

# Run the binary (requires FOCx38i0101t_c0f.fits in the working directory)
cargo run

# Build documentation and open in browser
make doc

# Generate documentation without opening
make build-doc
```

## Testing

Tests use Rust's built-in test framework and are colocated with source modules.

```bash
# Run all tests
cargo test
```

**18 tests pass across 4 modules:**

| Module | Tests | What's covered |
|---|---|---|
| `src/lib.rs` | 4 | `fill_to_2880()` padding calculation |
| `src/hdu/mod.rs` | 3 | HDU init, adding data and headers |
| `src/hdu/headers.rs` | 8 | Header formatting, key/value construction, END marker |
| `src/hdu/data.rs` | 2 | FITSData padding (including 800×600 pixel case) |

There are no integration tests or separate test files. All tests live in `#[cfg(test)]` blocks within each source file.

## FITS Format Constraints

The FITS specification imposes strict layout rules that are enforced throughout this codebase:

- **Block size:** All data must be aligned to **2880-byte** boundaries (padding filled with ASCII space `0x20`)
- **Header records:** Each record is exactly **80 bytes**: 10-byte key + 70-byte value
- **Key format:** 1–8 ASCII characters left-padded to 8 bytes, byte 8 = `=` (`0x3D`), byte 9 = space
- **END marker:** Every HDU header section ends with an `END` header record
- **HDU structure:** Each file contains one or more Header Data Units (HDUs), each with a header section followed by a data section

The `fill_to_2880(n: i32) -> i32` function in `src/lib.rs` is the core utility for computing required padding.

## Module Responsibilities

### `src/lib.rs`
Exports the `hdu` and `parsing` modules and defines `fill_to_2880()`.

### `src/hdu/mod.rs` — `HDU` struct
Container combining headers and data. Key methods:
- `HDU::init()` — creates an empty HDU
- `add_header(&str, &str)` — appends a key/value header
- `add_data(Vec<u8>)` — sets image data with automatic 2880-byte padding
- `raw_data()` — returns reference to raw data bytes

### `src/hdu/headers.rs` — `FITSHeader` struct
Handles all FITS header encoding. Key methods:
- `FITSHeader::new(key, value)` — creates a validated, padded header from string slices
- `FITSHeader::new_raw(key_bytes, value_bytes)` — fast path for parsing existing files (memcpy)
- `FITSHeader::end_hdu()` — creates the mandatory `END` marker
- `as_str()`, `key_as_str()`, `value_as_str()` — UTF-8 conversion helpers

Internal layout: `key: [u8; 10]`, `value: [u8; 70]`

### `src/hdu/data.rs` — `FITSData` struct
Stores raw pixel/image bytes. `add(Vec<u8>)` pads to the next 2880-byte boundary.

### `src/parsing/mod.rs` — `parse(img_path: &str)`
Proof-of-concept parser. Reads headers in 80-byte chunks until `END`, then reads image data. Currently hard-coded to a 1936×1096 pixel image. **Not production-ready.**

## Known Issues & Technical Debt

1. **`src/hdu/hdu.rs` appears to be a duplicate** of `src/hdu/mod.rs` — likely leftover from a module restructure. Verify before removing.
2. **Dead code warnings:** HDU methods (`init`, `raw_data`, `add_data`, `add_header`) are defined but never called outside of tests. The `data` field is never read.
3. **Hard-coded parser:** `src/parsing/mod.rs` uses a hard-coded array size for the test FITS file. Generalization is needed before the parser can handle arbitrary FITS files.
4. **No CI/CD:** There are no GitHub Actions workflows or other CI configuration.
5. **Broken doc link:** `src/hdu/headers.rs` references `thatwaseasy.example.com` as an example link.
6. **HDU method visibility:** HDU methods are private, limiting library usability without making them `pub`.

## Code Conventions

- **Error handling:** Use `std::io::Result<T>` for I/O operations; propagate with `?`
- **Constructors:** Use `new()` for validated construction, `new_raw()` for fast deserialization paths
- **Tests:** Colocate unit tests in `#[cfg(test)]` modules within the same file
- **FITS alignment:** Always use `fill_to_2880()` when computing padding — never hard-code alignment math
- **No unsafe:** The codebase has no `unsafe` blocks; keep it that way
- **No external dependencies:** Avoid adding crates unless absolutely necessary to preserve the zero-dependency property

## Development Workflow

Since there is no CI, run locally before pushing:

```bash
cargo build         # Ensure it compiles cleanly
cargo test          # All 18 tests must pass
cargo clippy        # Check for lints (no clippy.toml — uses defaults)
cargo fmt --check   # Ensure formatting (no rustfmt.toml — uses defaults)
```

Documentation:

```bash
make doc            # Build and open cargo docs in browser
```
