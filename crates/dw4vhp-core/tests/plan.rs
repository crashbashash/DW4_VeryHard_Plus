use dw4vhp_core::plan::{
    transform, PatchPlan, Preset, DESTRUCTIBLE_ROWS, PRACTICE_MAP, TRIPWIRE_ROWS,
};
use dw4vhp_core::ratio::RatioVector;
use dw4vhp_core::table::EnemyTable;

fn blank() -> EnemyTable {
    EnemyTable {
        hp: vec![100; 649],
        stat: vec![[10; 12]; 649],
        crit: vec![2; 649],
        para: vec![2; 649],
        rarity: vec![0; 649],
    }
}

#[test]
fn default_plan_is_the_very_hard_plus_preset() {
    let p = PatchPlan::default();
    assert_eq!(Preset::detect(&p), Preset::VeryHardPlus);
    assert!(p.practice_buff && p.exclude_destructibles && p.exclude_tripwire);
    assert!(!p.force_very_hard && p.serial.is_none() && p.crown_rank == 5);
    assert!(Preset::Extreme.plan().force_very_hard);
    assert_eq!(Preset::detect(&Preset::Extreme.plan()), Preset::Extreme);
}

#[test]
fn exclusions_are_left_byte_identical_and_every_other_row_gets_the_crown() {
    let t = blank();
    let (out, s) = transform(
        &t,
        &PatchPlan::default(),
        &RatioVector {
            hp: 2.0,
            stat: [2.0; 12],
            crit_delta: 0.0,
            para_delta: 0.0,
        },
    );
    for r in DESTRUCTIBLE_ROWS.chain(TRIPWIRE_ROWS) {
        assert_eq!(
            (out.hp[r], out.rarity[r]),
            (100, 0),
            "row {r} must be untouched"
        );
    }
    assert_eq!(out.rarity[0], 5);
    assert_eq!(out.rarity[300], 5); // an empty row still gets the crown
    assert_eq!(s.rows_changed, 649 - 11);
}

#[test]
fn the_i16_clamp_pins_high_values_and_does_not_keep_the_authored_value() {
    let mut t = blank();
    t.stat[0][0] = 30_000; // 30000 * 2 overflows
    t.stat[1][0] = -30_000; // and the negative side clamps too
    let (out, s) = transform(
        &t,
        &PatchPlan::default(),
        &RatioVector {
            hp: 1.0,
            stat: [2.0; 12],
            crit_delta: 0.0,
            para_delta: 0.0,
        },
    );
    assert_eq!(out.stat[0][0], 32767);
    assert_eq!(out.stat[1][0], -32768);
    assert_eq!(s.attack_pinned, 1);
}

#[test]
fn caps_apply_to_hp_and_to_crit_and_paralysis() {
    let mut t = blank();
    t.hp[0] = 300_000;
    t.crit[0] = 55;
    t.para[0] = 55;
    let (out, s) = transform(
        &t,
        &PatchPlan::default(),
        &RatioVector {
            hp: 13.0,
            stat: [1.0; 12],
            crit_delta: 19.5,
            para_delta: 19.5,
        },
    );
    assert_eq!(out.hp[0], 400_000);
    assert_eq!((out.crit[0], out.para[0]), (60, 60));
    assert_eq!((s.hp_capped, s.crit_pinned), (1, 1));
}

#[test]
fn rounding_is_ties_to_even() {
    let mut t = blank();
    t.crit[0] = 1; // 1 + 19.5 = 20.5 -> 20 ties-to-even, not 21
    t.crit[1] = 2; // 2 + 19.5 = 21.5 -> 22
    let (out, _) = transform(
        &t,
        &PatchPlan::default(),
        &RatioVector {
            hp: 1.0,
            stat: [1.0; 12],
            crit_delta: 19.5,
            para_delta: 19.5,
        },
    );
    assert_eq!(out.crit[0], 20);
    assert_eq!(out.crit[1], 22);
}

#[test]
fn the_practice_rows_become_an_ordinary_row_of_their_type() {
    let mut t = blank();
    t.hp[0] = 650;
    t.stat[0][0] = 190;
    t.crit[0] = 22;
    let (out, s) = transform(
        &t,
        &PatchPlan::default(),
        &RatioVector {
            hp: 1.0,
            stat: [1.0; 12],
            crit_delta: 0.0,
            para_delta: 0.0,
        },
    );
    assert_eq!(out.hp[638], out.hp[0]);
    assert_eq!(out.stat[638], out.stat[0]);
    assert_eq!(out.crit[638], out.crit[0]);
    assert_eq!(s.practice_rows, PRACTICE_MAP.len());
    assert_eq!((out.hp[644], out.rarity[644]), (100, 0)); // the tripwire survives the buff
}

#[test]
fn multipliers_scale_the_derived_factors() {
    let t = blank();
    let ratio = RatioVector {
        hp: 13.0,
        stat: [2.0; 12],
        crit_delta: 10.0,
        para_delta: 10.0,
    };
    let mut p = PatchPlan {
        hp_mult: 0.5,
        crit_mult: 0.0,
        ..Default::default()
    };
    p.stat_mult[0] = 3.0;
    let (out, _) = transform(&t, &p, &ratio);
    assert_eq!(out.hp[0], 650); // 100 * 6.5
    assert_eq!(out.stat[0][0], 60); // 10 * 6.0
    assert_eq!(out.crit[0], 2); // 2 + 0
}
