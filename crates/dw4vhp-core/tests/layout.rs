use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;

#[test]
fn retail_layout_matches_the_documented_constants() {
    let l = Layout::retail();
    assert_eq!(
        (l.hp_off, l.stat_off, l.byte_off),
        (0x78D64C5, 0x78DACB9, 0x78DA51D)
    );
    assert_eq!(l.rows, 649);
    assert_eq!(l.expected_copies, 665);
    assert_eq!(l.rows * 4, 2596);
    assert_eq!(l.rows * 24, 15576);
    assert_eq!(l.rows * 3, 1947);
}

#[test]
fn already_modded_message_tells_the_user_to_use_their_original() {
    let msg = Error::AlreadyModded {
        rarity_nonzero: 638,
    }
    .to_string();
    assert!(msg.contains("already"), "{msg}");
    assert!(msg.contains("original"), "{msg}");
}
