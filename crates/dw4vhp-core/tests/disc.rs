use dw4vhp_core::disc::{count_copies, inspect};
use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;
use dw4vhp_core::testkit;

#[test]
fn counts_every_occurrence() {
    let hay = b"abcabcabc";
    assert_eq!(count_copies(hay, b"abc"), 3);
    assert_eq!(count_copies(hay, b"zzz"), 0);
}

#[test]
fn accepts_a_clean_fixture() {
    let (dir, path, layout) = testkit::clean_disc_tempfile();
    let r = inspect(&path, &layout).unwrap();
    assert_eq!(r.copies, [3, 3, 3]);
    assert!(!r.already_modded);
    assert_eq!(r.authored.len(), 649);
    drop(dir);
}

#[test]
fn a_wrong_copy_count_is_a_revision_mismatch() {
    let (dir, path, _) = testkit::clean_disc_tempfile(); // the fixture writes 3 copies
    let err = inspect(&path, &Layout::mini(649, 665)).unwrap_err();
    assert!(matches!(err, Error::WrongRevision { .. }), "{err:?}");
    drop(dir);
}

#[test]
fn a_disc_with_crowns_already_written_is_already_modded() {
    let (dir, path, layout) = testkit::modded_disc_tempfile();
    let err = inspect(&path, &layout).unwrap_err();
    assert!(matches!(err, Error::AlreadyModded { .. }), "{err:?}");
    drop(dir);
}
