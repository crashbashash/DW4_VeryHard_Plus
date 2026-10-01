use dw4vhp_core::iso9660::{find_file, rewrite_name};
use dw4vhp_core::testkit;

#[test]
fn finds_a_root_directory_file_and_rewrites_its_name_in_place() {
    let mut iso = testkit::iso_with_files(&[
        ("SLUS_208.36;1", 0x1000, 4096),
        ("SYSTEM.CNF;1", 0x2000, 57),
    ]);
    let f = find_file(&iso, "SLUS_208.36").unwrap();
    assert_eq!((f.lba, f.size), (0x1000, 4096));
    assert_eq!(f.name_len, 13);
    rewrite_name(&mut iso, &f, "SLUS_000.00;1").unwrap();
    assert_eq!(&iso[f.name_offset as usize..][..13], b"SLUS_000.00;1");
    assert!(find_file(&iso, "SLUS_208.36").is_err()); // the old name is gone
    assert!(find_file(&iso, "SLUS_000.00").is_ok());
}

#[test]
fn refuses_a_name_of_a_different_length() {
    let mut iso = testkit::iso_with_files(&[("SLUS_208.36;1", 0x1000, 4096)]);
    let f = find_file(&iso, "SLUS_208.36").unwrap();
    assert!(rewrite_name(&mut iso, &f, "X;1").is_err());
}

#[test]
fn a_missing_file_is_an_error() {
    let iso = testkit::iso_with_files(&[("SYSTEM.CNF;1", 0x2000, 57)]);
    assert!(find_file(&iso, "SLUS_208.36").is_err());
}
