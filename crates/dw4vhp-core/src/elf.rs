//! The boot ELF: its program headers, the virtual-address-to-image mapping they
//! define, and the one instruction that forces Very Hard.
//!
//! The difficulty instruction's offset in the image is never hardcoded. The
//! boot ELF's own program headers say where its loadable segment lands, and a
//! virtual address no segment maps is refused rather than guessed at — the tool
//! declines a disc whose ELF it does not recognise.

use crate::error::{Error, Result};
use crate::iso9660::IsoFile;
use crate::region::WriteRegion;

/// ISO9660 sectors are 2 048 bytes, so an extent's byte offset is its LBA times
/// this.
const SECTOR: u64 = 2048;

/// The bytes of an ELF header the patcher reads: through `e_phnum` at 0x2C.
const HEADER_LEN: usize = 0x2C + 2;

/// Offset of `e_phoff` (u32) in the ELF header.
const E_PHOFF: usize = 0x1C;

/// Offset of `e_phentsize` (u16) in the ELF header.
const E_PHENTSIZE: usize = 0x2A;

/// Offset of `e_phnum` (u16) in the ELF header.
const E_PHNUM: usize = 0x2C;

/// The bytes of a program header the patcher reads: through `p_filesz`, whose
/// four bytes end at 0x13.
const PHDR_LEN: usize = 0x10 + 4;

/// Every supported boot ELF starts with these bytes.
const ELF_MAGIC: &[u8] = b"\x7fELF";

/// `PT_LOAD`: a segment that is loaded, and so the only kind that maps a virtual
/// address to a file offset.
const PT_LOAD: u32 = 1;

/// Virtual address of the instruction that saves the chosen difficulty.
pub const DIFFICULTY_VADDR: u32 = 0x0037_1DDC;

/// A loadable program header, reduced to the fields the mapping needs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Phdr {
    pub p_type: u32,
    pub p_offset: u32,
    pub p_vaddr: u32,
    pub p_filesz: u32,
}

/// A boot ELF in the image and the `PT_LOAD` segments it declares.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ElfInfo {
    /// Start of the ELF itself in the image.
    pub iso_offset: u64,
    /// The loadable program headers, in the order the header lists them.
    pub phdrs: Vec<Phdr>,
}

impl ElfInfo {
    /// Reads the ELF header and program-header table of `boot_elf`.
    ///
    /// Anything unreadable — a truncated extent, a missing magic — is
    /// [`Error::ElfNotFound`], because it means the file at the directory record
    /// is not the boot ELF this patcher supports.
    pub fn read(iso: &[u8], boot_elf: &IsoFile) -> Result<Self> {
        let iso_offset = u64::from(boot_elf.lba) * SECTOR;
        let header = elf_slice(iso, iso_offset, HEADER_LEN)?;
        if !header.starts_with(ELF_MAGIC) {
            return Err(Error::ElfNotFound);
        }

        let phoff = u64::from(le_u32(&header[E_PHOFF..]));
        let phentsize = u64::from(le_u16(&header[E_PHENTSIZE..]));
        let phnum = u64::from(le_u16(&header[E_PHNUM..]));

        let mut phdrs = Vec::new();
        for index in 0..phnum {
            let offset = iso_offset + phoff + index * phentsize;
            let entry = elf_slice(iso, offset, PHDR_LEN)?;
            let p_type = le_u32(entry);
            if p_type == PT_LOAD {
                phdrs.push(Phdr {
                    p_type,
                    p_offset: le_u32(&entry[4..]),
                    p_vaddr: le_u32(&entry[8..]),
                    p_filesz: le_u32(&entry[16..]),
                });
            }
        }
        Ok(Self { iso_offset, phdrs })
    }

    /// The image offset of `vaddr`, through the first loadable segment that
    /// covers it, or `None` if no segment does.
    pub fn vaddr_to_iso_offset(&self, vaddr: u32) -> Option<u64> {
        self.phdrs.iter().find_map(|phdr| {
            let end = phdr.p_vaddr.checked_add(phdr.p_filesz)?;
            if phdr.p_vaddr <= vaddr && vaddr < end {
                Some(self.iso_offset + u64::from(phdr.p_offset) + u64::from(vaddr - phdr.p_vaddr))
            } else {
                None
            }
        })
    }
}

/// The one-instruction "Extreme" patch: the difficulty word the ELF is expected
/// to hold, or [`Error::UnexpectedInstruction`] with the word actually found.
///
/// `SYSsetDifficulty` begins by saving its argument to a callee-saved register
/// (`daddu s0, a0, zero`), and the store further down reads only that saved
/// register. Replacing the save with `addiu s0, zero, 1` therefore makes the
/// store write 1 whatever difficulty the player chose — four bytes over four,
/// semantically identical to the documented 15-byte source mod.
///
/// A virtual address no segment maps is [`Error::ElfNotFound`]: the retail ELF
/// always maps it, so an unmapped one is a different file.
pub fn difficulty_region(iso: &[u8], elf: &ElfInfo) -> Result<WriteRegion> {
    let offset = elf
        .vaddr_to_iso_offset(DIFFICULTY_VADDR)
        .ok_or(Error::ElfNotFound)?;
    let word = le_u32(elf_slice(iso, offset, 4)?);
    if word != mips::daddu(16, 4, 0) {
        return Err(Error::UnexpectedInstruction { offset, word });
    }
    Ok(WriteRegion {
        offset,
        bytes: mips::addiu(16, 0, 1).to_le_bytes().to_vec(),
    })
}

/// The MIPS encodings the difficulty patch is built from.
///
/// Both instruction words are computed here from their fields, so no game bytes
/// are ever written into this source file.
pub mod mips {
    /// `daddu`'s function code within SPECIAL.
    const DADDU_FUNCT: u32 = 0x2D;

    /// `addiu`'s opcode, bits 26..31.
    const ADDIU_OPCODE: u32 = 0x09;

    /// `daddu rd, rs, rt`.
    ///
    /// Its opcode, SPECIAL, is zero, so it contributes no bits: the word is
    /// `rs << 21 | rt << 16 | rd << 11 | funct`.
    pub fn daddu(rd: u32, rs: u32, rt: u32) -> u32 {
        (rs << 21) | (rt << 16) | (rd << 11) | DADDU_FUNCT
    }

    /// `addiu rt, rs, imm`.
    pub fn addiu(rt: u32, rs: u32, imm: i32) -> u32 {
        (ADDIU_OPCODE << 26) | (rs << 21) | (rt << 16) | ((imm & 0xFFFF) as u32)
    }

    /// The six-bit opcode in bits 26..31.
    pub fn opcode(word: u32) -> u32 {
        word >> 26
    }

    /// The `rs` register field in bits 21..25.
    pub fn rs(word: u32) -> u32 {
        (word >> 21) & 0x1F
    }

    /// The `rt` register field in bits 16..20.
    pub fn rt(word: u32) -> u32 {
        (word >> 16) & 0x1F
    }

    /// The `rd` register field in bits 11..15.
    pub fn rd(word: u32) -> u32 {
        (word >> 11) & 0x1F
    }

    /// The six-bit function code in bits 0..5.
    pub fn funct(word: u32) -> u32 {
        word & 0x3F
    }
}

/// The `len` bytes of the image at `offset`, or [`Error::ElfNotFound`] when the
/// image ends first: a read that runs past it cannot have found the boot ELF.
fn elf_slice(iso: &[u8], offset: u64, len: usize) -> Result<&[u8]> {
    let start = usize::try_from(offset).map_err(|_| Error::ElfNotFound)?;
    let end = start.checked_add(len).ok_or(Error::ElfNotFound)?;
    iso.get(start..end).ok_or(Error::ElfNotFound)
}

/// The little-endian `u32` at the start of `bytes`.
fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The little-endian `u16` at the start of `bytes`.
fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}
