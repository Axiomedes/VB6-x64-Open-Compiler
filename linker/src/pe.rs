//! PE32+ (Portable Executable 64-bit) header validation and inspection.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeError {
    #[error("IO error reading PE file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid DOS signature: expected 0x5A4D ('MZ'), found 0x{0:04X}")]
    InvalidDosSignature(u16),
    #[error("Invalid PE signature: expected 0x00004550 ('PE\\0\\0'), found 0x{0:08X}")]
    InvalidPeSignature(u32),
    #[error("Invalid Machine type: expected 0x8664 (IMAGE_FILE_MACHINE_AMD64), found 0x{0:04X}")]
    InvalidMachine(u16),
    #[error("Invalid Optional Header magic: expected 0x020B (PE32+ / 64-bit), found 0x{0:04X}")]
    InvalidMagic(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeInfo {
    pub machine: u16,
    pub section_count: u16,
    pub magic: u16,
    pub entry_point: u32,
    pub image_base: u64,
    pub subsystem: u16,
}

impl PeInfo {
    /// Inspect and validate that a binary file is a valid PE32+ (Windows x64) executable.
    pub fn inspect<P: AsRef<Path>>(path: P) -> Result<Self, PeError> {
        let mut file = File::open(path)?;

        // 1. Read DOS Header
        let mut dos_header = [0u8; 64];
        file.read_exact(&mut dos_header)?;

        let e_magic = u16::from_le_bytes([dos_header[0], dos_header[1]]);
        if e_magic != 0x5A4D {
            return Err(PeError::InvalidDosSignature(e_magic));
        }

        // Offset to PE Header is at 0x3C
        let e_lfanew = u32::from_le_bytes([
            dos_header[0x3C],
            dos_header[0x3D],
            dos_header[0x3E],
            dos_header[0x3F],
        ]) as u64;

        // 2. Read PE Signature & COFF Header
        file.seek(SeekFrom::Start(e_lfanew))?;
        let mut pe_signature = [0u8; 4];
        file.read_exact(&mut pe_signature)?;
        let sig = u32::from_le_bytes(pe_signature);
        if sig != 0x00004550 {
            return Err(PeError::InvalidPeSignature(sig));
        }

        // COFF Header (20 bytes)
        let mut coff_header = [0u8; 20];
        file.read_exact(&mut coff_header)?;
        let machine = u16::from_le_bytes([coff_header[0], coff_header[1]]);
        if machine != 0x8664 {
            return Err(PeError::InvalidMachine(machine));
        }
        let section_count = u16::from_le_bytes([coff_header[2], coff_header[3]]);

        // 3. Read Optional Header (PE32+)
        let mut opt_magic_buf = [0u8; 2];
        file.read_exact(&mut opt_magic_buf)?;
        let magic = u16::from_le_bytes(opt_magic_buf);
        if magic != 0x020B {
            return Err(PeError::InvalidMagic(magic));
        }

        // Read up through Subsystem in PE32+ optional header
        // Offset 16 in Optional Header is AddressOfEntryPoint
        // Offset 24 in Optional Header is ImageBase (u64)
        // Offset 68 in Optional Header is Subsystem (u16)
        let mut opt_rest = [0u8; 70];
        file.read_exact(&mut opt_rest)?;

        let entry_point = u32::from_le_bytes([
            opt_rest[14],
            opt_rest[15],
            opt_rest[16],
            opt_rest[17],
        ]);

        let image_base = u64::from_le_bytes([
            opt_rest[22],
            opt_rest[23],
            opt_rest[24],
            opt_rest[25],
            opt_rest[26],
            opt_rest[27],
            opt_rest[28],
            opt_rest[29],
        ]);

        let subsystem = u16::from_le_bytes([opt_rest[66], opt_rest[67]]);

        Ok(Self {
            machine,
            section_count,
            magic,
            entry_point,
            image_base,
            subsystem,
        })
    }
}
