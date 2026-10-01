//! The minimum ISO9660 the patcher needs: find a file in the root directory and
//! rewrite its directory-record name.
//!
//! Nothing in the patcher relocates a file, so no write may change the length of
//! a directory record: [`rewrite_name`] refuses a name whose length differs from
//! the stored one, and a rewritten name is therefore always exactly as long as
//! the name it replaces.

use crate::error::{Error, Result};

/// ISO9660 sectors are 2 048 bytes.
const SECTOR: usize = 2048;

/// The primary volume descriptor occupies sector 16 and identifies itself with
/// `CD001` at offset 1.
const PVD_OFFSET: usize = 0x8000;
const PVD_IDENTIFIER_OFFSET: usize = 1;
const PVD_IDENTIFIER: &[u8] = b"CD001";

/// Offset of the root directory's record inside the primary volume descriptor.
const ROOT_RECORD_OFFSET: usize = 156;

/// Bytes of a directory record before its name: record length, extended-attribute
/// length, the both-endian extent LBA, the both-endian data length, the seven-byte
/// timestamp, flags, unit size, interleave gap, the both-endian volume sequence
/// number and the name length.
const RECORD_FIXED_LEN: usize = 33;

/// The little-endian half of a both-endian field.
fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// A name without its `;1` version suffix, which every stored name carries and no
/// lookup includes.
fn without_version(name: &[u8]) -> &[u8] {
    match name.iter().position(|&byte| byte == b';') {
        Some(at) => &name[..at],
        None => name,
    }
}

/// Where a file lives: its directory record's offsets in the image, the length of
/// the stored name (including the `;1` suffix) and the extent the record points
/// at. All offsets are absolute ISO offsets.
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct IsoFile {
    /// Start of the file's directory record.
    pub record_offset: u64,
    /// Start of the name field inside that record.
    pub name_offset: u64,
    /// Stored name length, `;1` suffix included: `SLUS_208.36;1` is 13.
    pub name_len: usize,
    /// Extent LBA the record points at.
    pub lba: u32,
    /// Length of the extent in bytes.
    pub size: u32,
}

/// Finds `name` in the root directory, comparing without the `;1` version suffix.
///
/// Only the root directory is searched: every file the patcher touches lives
/// there, and the boot ELF's serial is the only name that is ever rewritten.
pub fn find_file(iso: &[u8], name: &str) -> Result<IsoFile> {
    let (directory_lba, directory_len) = root_directory(iso)?;
    let start = directory_lba as usize * SECTOR;
    let end = start + directory_len as usize;
    let wanted = without_version(name.as_bytes());

    let mut offset = start;
    while offset + RECORD_FIXED_LEN <= end && offset + RECORD_FIXED_LEN <= iso.len() {
        let record_len = iso[offset] as usize;
        if record_len == 0 {
            // Zero-length padding to the end of the sector: the next record, if
            // any, starts at the next sector.
            offset = (offset / SECTOR + 1) * SECTOR;
            continue;
        }
        if record_len < RECORD_FIXED_LEN {
            return Err(Error::NotThisDisc);
        }
        let record = iso
            .get(offset..offset + record_len)
            .ok_or(Error::NotThisDisc)?;

        let name_len = record[32] as usize;
        let stored = record.get(33..33 + name_len).ok_or(Error::NotThisDisc)?;
        if without_version(stored) == wanted {
            return Ok(IsoFile {
                record_offset: offset as u64,
                name_offset: (offset + RECORD_FIXED_LEN) as u64,
                name_len,
                lba: le_u32(&record[2..6]),
                size: le_u32(&record[10..14]),
            });
        }
        offset += record_len;
    }

    Err(Error::FileNotFound {
        name: name.to_string(),
    })
}

/// Writes `new_name` over the name of `file`, refusing any length but the
/// stored one so that no byte of the image moves.
pub fn rewrite_name(iso: &mut [u8], file: &IsoFile, new_name: &str) -> Result<()> {
    if new_name.len() != file.name_len {
        return Err(Error::NameLengthMismatch {
            found: new_name.len(),
            expected: file.name_len,
        });
    }
    let start = file.name_offset as usize;
    let name = iso
        .get_mut(start..start + file.name_len)
        .ok_or(Error::NotThisDisc)?;
    name.copy_from_slice(new_name.as_bytes());
    Ok(())
}

/// The root directory's extent LBA and length, read from the primary volume
/// descriptor's root record as little-endian.
fn root_directory(iso: &[u8]) -> Result<(u32, u32)> {
    let descriptor = iso
        .get(PVD_OFFSET..PVD_OFFSET + SECTOR)
        .ok_or(Error::NotThisDisc)?;
    let identifier = PVD_IDENTIFIER_OFFSET..PVD_IDENTIFIER_OFFSET + PVD_IDENTIFIER.len();
    if &descriptor[identifier] != PVD_IDENTIFIER {
        return Err(Error::NotThisDisc);
    }
    let root = &descriptor[ROOT_RECORD_OFFSET..ROOT_RECORD_OFFSET + RECORD_FIXED_LEN];
    Ok((le_u32(&root[2..6]), le_u32(&root[10..14])))
}
