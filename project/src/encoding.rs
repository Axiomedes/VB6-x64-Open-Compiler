//! File encoding utilities for reading legacy VB6 source files.
//! Supports Windows-1252 (ANSI) and UTF-8 transparently.

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Read a file to String, automatically decoding Windows-1252 / ANSI or UTF-8.
pub fn read_source_file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    // Try UTF-8 first
    if let Ok(s) = std::str::from_utf8(&bytes) {
        return Ok(s.to_string());
    }

    // Fallback to Windows-1252 (canonical encoding for non-Unicode VB6 projects)
    let (cow, _encoding_used, _had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
    Ok(cow.into_owned())
}
