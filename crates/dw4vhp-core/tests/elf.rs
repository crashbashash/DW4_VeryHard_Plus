use dw4vhp_core::elf::{difficulty_region, mips, ElfInfo, DIFFICULTY_VADDR};
use dw4vhp_core::error::Error;
use dw4vhp_core::testkit;

#[test]
fn the_two_instructions_are_encoded_from_their_fields() {
    assert_eq!(mips::daddu(16, 4, 0), 0x0080_802D); // daddu s0, a0, zero
    assert_eq!(mips::addiu(16, 0, 1), 0x2410_0001); // addiu s0, zero, 1
}

#[test]
fn decodes_the_fields_it_encoded() {
    let w = mips::daddu(16, 4, 0);
    assert_eq!(
        (
            mips::opcode(w),
            mips::funct(w),
            mips::rd(w),
            mips::rs(w),
            mips::rt(w)
        ),
        (0, 0x2D, 16, 4, 0)
    );
}

#[test]
fn maps_a_vaddr_through_the_program_headers() {
    let iso = testkit::iso_with_elf();
    let elf = ElfInfo::read(&iso, &testkit::boot_elf(&iso)).unwrap();
    assert_eq!(elf.vaddr_to_iso_offset(0x0010_0000), Some(0x80 + 0x1000));
    assert!(elf.vaddr_to_iso_offset(0x0037_1DDC).is_some());
    assert_eq!(elf.vaddr_to_iso_offset(0x9000_0000), None);
}

#[test]
fn produces_a_four_byte_replacement_at_the_mapped_offset() {
    let iso = testkit::iso_with_elf();
    let elf = ElfInfo::read(&iso, &testkit::boot_elf(&iso)).unwrap();
    let r = difficulty_region(&iso, &elf).unwrap();
    assert_eq!(r.bytes.len(), 4);
    assert_eq!(r.bytes, 0x2410_0001u32.to_le_bytes());
    assert_eq!(r.offset, elf.vaddr_to_iso_offset(DIFFICULTY_VADDR).unwrap());
}

#[test]
fn refuses_when_the_instruction_is_not_what_it_expects() {
    let mut iso = testkit::iso_with_elf();
    let elf = ElfInfo::read(&iso, &testkit::boot_elf(&iso)).unwrap();
    let off = elf.vaddr_to_iso_offset(DIFFICULTY_VADDR).unwrap() as usize;
    iso[off..off + 4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
    assert!(matches!(
        difficulty_region(&iso, &elf).unwrap_err(),
        Error::UnexpectedInstruction { .. }
    ));
}
