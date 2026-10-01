use dw4vhp_core::error::Error;
use dw4vhp_core::iso9660::find_file;
use dw4vhp_core::serial::{serial_regions, validate_serial, DEFAULT_SERIAL};
use dw4vhp_core::testkit;

#[test]
fn accepts_the_eleven_character_shape_and_rejects_everything_else() {
    assert!(validate_serial(DEFAULT_SERIAL).is_ok());
    assert!(validate_serial("SLUS_208.36").is_ok());
    for bad in [
        "",
        "SLUS_208.3",
        "slus_208.36",
        "SLUS-208.36",
        "SLUS_2083.36",
        "SLUS_208.366",
    ] {
        assert!(validate_serial(bad).is_err(), "{bad} should be rejected");
    }
}

#[test]
fn produces_two_same_length_regions_covering_the_record_and_the_boot_line() {
    let iso = testkit::iso_with_elf_and_cnf("SLUS_208.36");
    let elf = find_file(&iso, "SLUS_208.36").unwrap();
    let rs = serial_regions(&iso, &elf, DEFAULT_SERIAL).unwrap();
    assert_eq!(rs.len(), 2);
    assert_eq!(rs[0].bytes, b"SLUS_000.00;1");
    assert_eq!(rs[1].bytes, b"SLUS_000.00");
    for r in &rs {
        let old = &iso[r.offset as usize..r.offset as usize + r.bytes.len()];
        assert_ne!(old, &r.bytes[..]);
        assert!(old.len() == r.bytes.len()); // same length: nothing moves
    }
}

#[test]
fn an_invalid_serial_never_produces_regions() {
    let iso = testkit::iso_with_elf_and_cnf("SLUS_208.36");
    let elf = find_file(&iso, "SLUS_208.36").unwrap();
    assert!(serial_regions(&iso, &elf, "nope").is_err());
}

// The ruled error mapping (Task 8 ruling 6) beyond the invalid-serial case the
// third test above covers: the wrong-shape serial's variant, a missing
// SYSTEM.CNF and a SYSTEM.CNF that is not this disc's each report their own.

#[test]
fn a_wrong_shape_is_serial_invalid() {
    let err = validate_serial("nope").unwrap_err();
    assert!(matches!(err, Error::SerialInvalid(_)), "got {err}");
}

#[test]
fn a_missing_system_cnf_is_a_file_not_found() {
    let iso = testkit::iso_with_elf();
    let elf = find_file(&iso, "SLUS_208.36").unwrap();
    let err = serial_regions(&iso, &elf, DEFAULT_SERIAL).unwrap_err();
    assert!(matches!(err, Error::FileNotFound { .. }), "got {err}");
}

#[test]
fn a_cnf_without_a_boot_line_is_not_this_disc() {
    let iso = iso_with_cnf_payload("no boot line here\r\n");
    let elf = find_file(&iso, "SLUS_208.36").unwrap();
    let err = serial_regions(&iso, &elf, DEFAULT_SERIAL).unwrap_err();
    assert!(matches!(err, Error::NotThisDisc), "got {err}");
}

#[test]
fn a_boot_line_naming_another_serial_is_not_this_disc() {
    let iso = iso_with_cnf_payload("BOOT2 = cdrom0:\\SLUS_999.99;1\r\n");
    let elf = find_file(&iso, "SLUS_208.36").unwrap();
    let err = serial_regions(&iso, &elf, DEFAULT_SERIAL).unwrap_err();
    assert!(matches!(err, Error::NotThisDisc), "got {err}");
}

/// An image whose `SLUS_208.36` record is real but whose `SYSTEM.CNF` carries
/// `payload` instead of the retail boot line, so the two identity refusals can
/// each be built without a fixture the brief does not ask for.
fn iso_with_cnf_payload(payload: &str) -> Vec<u8> {
    let lba = 0x2000u32;
    let mut iso = testkit::iso_with_files(&[
        ("SLUS_208.36;1", 0x1000, 4096),
        ("SYSTEM.CNF;1", lba, payload.len() as u32),
    ]);
    let start = lba as usize * 2048;
    iso[start..start + payload.len()].copy_from_slice(payload.as_bytes());
    iso
}
