//! The disc serial: validating a player-chosen one and building the two
//! same-length rewrites that give a patched image its own identity.
//!
//! PCSX2 titles a disc from its serial, so a patched image without this step
//! keeps showing up as the retail `SLUS_208.36`. The serial appears in exactly
//! two places — the boot ELF's ISO9660 directory-record name and the `BOOT2`
//! line inside `SYSTEM.CNF` — and both are rewritten in place, each at exactly
//! the length it already has, so no byte of the image moves.
//!
//! The current serial is never hardcoded: it is read from the directory record
//! `find_file` located, so a disc revision whose serial differs is handled too.

use crate::error::{Error, Result};
use crate::iso9660::{find_file, IsoFile};
use crate::region::WriteRegion;

/// The serial the GUI pre-fills: well-formed, and used by no retail Digimon
/// World 4 disc (verified against PCSX2 2.8.2).
pub const DEFAULT_SERIAL: &str = "SLUS_000.00";

/// A serial is 11 characters: four product-code characters, an underscore,
/// three digits, a dot and two digits, as in `SLUS_208.36`.
const SERIAL_LEN: usize = 11;

/// The version suffix every stored ISO9660 name carries.
const VERSION_SUFFIX: &str = ";1";

/// The marker that begins the boot line inside `SYSTEM.CNF`.
const BOOT_MARKER: &[u8] = b"BOOT2";

/// ISO9660 sector size, so an extent's byte offset is its LBA times this.
const SECTOR: usize = 2048;

/// Accepts exactly the eleven-character shape
/// `[A-Z0-9]{4}_[0-9]{3}\.[0-9]{2}`, refusing everything else with
/// [`Error::SerialInvalid`].
///
/// Lowercase, a hyphen where the underscore belongs, and every other length are
/// all invalid.
pub fn validate_serial(s: &str) -> Result<()> {
    if is_serial(s.as_bytes()) {
        Ok(())
    } else {
        Err(Error::SerialInvalid(s.to_string()))
    }
}

/// Whether `bytes` is exactly a serial: four uppercase letters or digits, an
/// underscore, three digits, a dot and two digits.
fn is_serial(bytes: &[u8]) -> bool {
    if bytes.len() != SERIAL_LEN {
        return false;
    }
    bytes[..4]
        .iter()
        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        && bytes[4] == b'_'
        && bytes[5..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'.'
        && bytes[9..11].iter().all(u8::is_ascii_digit)
}

/// The two same-length rewrites that rename the disc `new_serial`: first the
/// boot ELF's directory-record name, then the serial inside `SYSTEM.CNF`'s
/// `BOOT2` line.
///
/// Both regions replace exactly as many bytes as they already occupy, so the
/// image's length and every other offset stay as they are. The current serial is
/// read from the directory record, and the boot line is searched for only inside
/// `SYSTEM.CNF`'s extent.
///
/// An invalid `new_serial` is [`Error::SerialInvalid`]; a missing `SYSTEM.CNF`
/// record is [`Error::FileNotFound`]; and a `SYSTEM.CNF` that lacks the boot
/// line, or the current serial in it, is [`Error::NotThisDisc`] — the retail
/// file always carries both.
pub fn serial_regions(
    iso: &[u8],
    boot_elf: &IsoFile,
    new_serial: &str,
) -> Result<Vec<WriteRegion>> {
    validate_serial(new_serial)?;

    let name = iso_slice(iso, boot_elf.name_offset, boot_elf.name_len)?;
    let old_serial = name
        .strip_suffix(VERSION_SUFFIX.as_bytes())
        .filter(|serial| serial.len() == SERIAL_LEN)
        .ok_or(Error::NotThisDisc)?;

    let cnf = find_file(iso, "SYSTEM.CNF")?;
    let extent_start = cnf.lba as usize * SECTOR;
    let extent = iso_slice(iso, extent_start as u64, cnf.size as usize)?;

    let boot = memchr::memmem::find(extent, BOOT_MARKER).ok_or(Error::NotThisDisc)?;
    let in_boot_line =
        memchr::memmem::find(&extent[boot..], old_serial).ok_or(Error::NotThisDisc)?;
    let serial_offset = (extent_start + boot + in_boot_line) as u64;

    Ok(vec![
        WriteRegion {
            offset: boot_elf.name_offset,
            bytes: format!("{new_serial}{VERSION_SUFFIX}").into_bytes(),
        },
        WriteRegion {
            offset: serial_offset,
            bytes: new_serial.as_bytes().to_vec(),
        },
    ])
}

/// The `len` bytes of the image at `offset`, or [`Error::NotThisDisc`] when the
/// image ends first: a read that runs past it cannot be the retail layout.
fn iso_slice(iso: &[u8], offset: u64, len: usize) -> Result<&[u8]> {
    let start = usize::try_from(offset).map_err(|_| Error::NotThisDisc)?;
    let end = start.checked_add(len).ok_or(Error::NotThisDisc)?;
    iso.get(start..end).ok_or(Error::NotThisDisc)
}
