use dw4vhp_core::ratio::{median4, RatioVector};
use dw4vhp_core::table::EnemyTable;

fn table_with(
    hp: &[(usize, i32)],
    stat: &[(usize, usize, i16)],
    chg: &[(usize, u8, u8)],
) -> EnemyTable {
    let mut t = EnemyTable {
        hp: vec![0; 649],
        stat: vec![[0; 12]; 649],
        crit: vec![0; 649],
        para: vec![0; 649],
        rarity: vec![0; 649],
    };
    for &(r, v) in hp {
        t.hp[r] = v;
    }
    for &(r, c, v) in stat {
        t.stat[r][c] = v;
    }
    for &(r, a, b) in chg {
        t.crit[r] = a;
        t.para[r] = b;
    }
    t
}

#[test]
fn derives_the_documented_factors_from_the_reference_rows() {
    // authored reference values (e_goburi rows 0-3 vs the rank-5 row 391)
    let hp = [(0, 50), (1, 58), (2, 74), (3, 77), (391, 650)];
    let mut stat = Vec::new();
    for (r, atk, def) in [
        (0, 16, 90),
        (1, 18, 92),
        (2, 22, 95),
        (3, 24, 98),
        (391, 190, 194),
    ] {
        stat.push((r, 0, atk));
        stat.push((r, 1, def));
    }
    let chg = [(0, 3, 3), (1, 4, 4), (2, 5, 5), (3, 6, 6), (391, 24, 24)];
    let v = RatioVector::derive(&table_with(&hp, &stat, &chg));
    assert_eq!(v.hp, 13.0); // 650 / 50
    assert!((v.stat[0] - 11.875).abs() < 1e-12); // 190 / 16
    assert!((v.stat[1] - 194.0 / 90.0).abs() < 1e-12);
    assert_eq!(v.crit_delta, 19.5);
    assert_eq!(v.para_delta, 19.5);
}

#[test]
fn median_of_four_is_the_mean_of_the_middle_two() {
    assert_eq!(median4([3, 4, 5, 6]), 4.5);
    assert_eq!(median4([10, 1, 4, 4]), 4.0);
}

#[test]
fn a_zero_denominator_yields_a_factor_of_one() {
    let v = RatioVector::derive(&table_with(&[(391, 650)], &[(391, 0, 190)], &[]));
    assert_eq!(v.hp, 650.0); // max(1, 0) guards the HP denominator
    assert_eq!(v.stat[0], 1.0); // column denominator 0 -> 1.0
}
