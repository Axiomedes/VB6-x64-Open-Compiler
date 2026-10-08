//! Loader for VB6 binary resource files (.frx).
//! Reads packed binary payloads (icons, pictures, raw buffers) by 32-bit hex offsets.

use crate::models::FrxResourceFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct FRXLoader;

impl FRXLoader {
    /// Read a .frx file from disk and extract payload entries for a given list of offsets.
    pub fn load_offsets<P: AsRef<Path>>(path: P, offsets: &[u32]) -> std::io::Result<FrxResourceFile> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)?;
        let mut raw_bytes = Vec::new();
        file.read_to_end(&mut raw_bytes)?;

        let mut entries = HashMap::new();

        for &offset in offsets {
            let off = offset as usize;
            if off + 4 <= raw_bytes.len() {
                let len_bytes: [u8; 4] = [
                    raw_bytes[off],
                    raw_bytes[off + 1],
                    raw_bytes[off + 2],
                    raw_bytes[off + 3],
                ];
                let payload_len = u32::from_le_bytes(len_bytes) as usize;
                let start = off + 4;
                let end = (start + payload_len).min(raw_bytes.len());
                let payload = raw_bytes[start..end].to_vec();
                entries.insert(offset, payload);
            }
        }

        Ok(FrxResourceFile {
            path: path_ref.to_path_buf(),
            entries,
        })
    }

    /// Load the entire raw .frx file into entry at offset 0 if offsets are not specified.
    pub fn load_raw<P: AsRef<Path>>(path: P) -> std::io::Result<FrxResourceFile> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)?;
        let mut raw_bytes = Vec::new();
        file.read_to_end(&mut raw_bytes)?;

        let mut entries = HashMap::new();
        entries.insert(0, raw_bytes);

        Ok(FrxResourceFile {
            path: path_ref.to_path_buf(),
            entries,
        })
    }
}
