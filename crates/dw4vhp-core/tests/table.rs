use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;
use dw4vhp_core::table::EnemyTable;

fn mini_iso(rows: usize) -> Vec<u8> {
    let l = Layout::mini(rows, 3);
    let mut iso = vec![0u8; (l.byte_off as usize) + rows * 3];
    for r in 0..rows {
        iso[(l.hp_off as usize) + 4 * r..][..4].copy_from_slice(&((r as i32) * 100).to_le_bytes());
        for c in 0..12 {
            let v = ((r * 12 + c) as i16) - 5;
            iso[(l.stat_off as usize) + 24 * r + 2 * c..][..2].copy_from_slice(&v.to_le_bytes());
        }
        iso[(l.byte_off as usize) + 3 * r] = (r % 20) as u8; // crit
        iso[(l.byte_off as usize) + 3 * r + 1] = (r % 7) as u8; // para
        iso[(l.byte_off as usize) + 3 * r + 2] = if r == 1 { 5 } else { 0 };
    }
    iso
}

#[test]
fn reads_hp_stats_and_chargen_in_their_documented_encodings() {
    let l = Layout::mini(8, 3);
    let t = EnemyTable::read(&mini_iso(8), &l).unwrap();
    assert_eq!(t.len(), 8);
    assert_eq!(t.hp[3], 300);
    assert_eq!(t.stat[3][2], (3 * 12 + 2) as i16 - 5);
    assert_eq!(t.crit[3], 3);
    assert_eq!(t.para[3], 3);
    assert_eq!(t.rarity[1], 5);
    assert_eq!(t.rarity[0], 0);
}

#[test]
fn blocks_round_trip_and_have_the_documented_sizes() {
    let l = Layout::mini(649, 3);
    let t = EnemyTable::read(&mini_iso(649), &l).unwrap();
    let b = t.blocks();
    assert_eq!([b[0].len(), b[1].len(), b[2].len()], [2596, 15576, 1947]);
    let mut iso = mini_iso(649);
    iso[(l.hp_off as usize)..][..2596].copy_from_slice(&b[0]);
    iso[(l.stat_off as usize)..][..15576].copy_from_slice(&b[1]);
    iso[(l.byte_off as usize)..][..1947].copy_from_slice(&b[2]);
    let again = EnemyTable::read(&iso, &l).unwrap();
    assert_eq!(again.hp, t.hp);
    assert_eq!(again.stat, t.stat);
    assert_eq!(again.rarity, t.rarity);
}

#[test]
fn read_rejects_a_buffer_that_is_too_short() {
    let l = Layout::retail();
    assert!(EnemyTable::read(&[0u8; 64], &l).is_err());
}

#[test]
fn blocks_reproduce_every_field_of_the_table() {
    let l = Layout::mini(8, 3);
    let t = EnemyTable::read(&mini_iso(8), &l).unwrap();
    assert!(!t.is_empty());
    let b = t.blocks();
    assert_eq!([b[0].len(), b[1].len(), b[2].len()], [32, 192, 24]);
    let mut iso = mini_iso(8);
    iso[(l.hp_off as usize)..][..b[0].len()].copy_from_slice(&b[0]);
    iso[(l.stat_off as usize)..][..b[1].len()].copy_from_slice(&b[1]);
    iso[(l.byte_off as usize)..][..b[2].len()].copy_from_slice(&b[2]);
    assert_eq!(EnemyTable::read(&iso, &l).unwrap(), t);
}

#[test]
fn read_reports_not_this_disc_when_a_region_does_not_fit() {
    let l = Layout::retail();
    let err = EnemyTable::read(&[0u8; 64], &l).unwrap_err();
    assert!(matches!(err, Error::NotThisDisc), "{err:?}");

    // Long enough for the chargen block, one byte short of the stats block.
    let m = Layout::mini(8, 3);
    let truncated =
        EnemyTable::read(&vec![0u8; (m.stat_off as usize) + 8 * 24 - 1], &m).unwrap_err();
    assert!(matches!(truncated, Error::NotThisDisc), "{truncated:?}");
}
