use dw4vhp_core::plan::{
    transform, PatchPlan, Preset, DESTRUCTIBLE_ROWS, PRACTICE_MAP, PRACTICE_ROWS, TRIPWIRE_ROWS,
};
use dw4vhp_core::ratio::RatioVector;
use dw4vhp_core::rowgroup::{COLLAPSE_SKIP_ROWS, ROW_GROUPS};
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
    assert!(!p.exclude_practice && !p.force_very_hard && p.serial.is_none() && p.crown_rank == 5);
    let extreme = Preset::Extreme.plan();
    assert!(extreme.force_very_hard && extreme.exclude_practice);
    assert_eq!(Preset::detect(&extreme), Preset::Extreme);
}

#[test]
fn the_extreme_and_brutal_presets_leave_the_training_stage_vanilla() {
    let t = blank();
    for preset in [Preset::Extreme, Preset::Brutal] {
        let (out, s) = transform(
            &t,
            &preset.plan(),
            &RatioVector {
                hp: 13.0,
                stat: [2.0; 12],
                crit_delta: 19.5,
                para_delta: 19.5,
            },
        );
        for r in PRACTICE_ROWS {
            assert_eq!(
                (
                    out.hp[r],
                    out.stat[r],
                    out.crit[r],
                    out.para[r],
                    out.rarity[r]
                ),
                (100, [10; 12], 2, 2, 0),
                "{preset:?} row {r} must stay exactly as authored"
            );
        }
        assert_eq!(s.practice_rows, 0);
    }
}

#[test]
fn exclude_practice_beats_the_practice_buff() {
    let t = blank();
    let plan = PatchPlan {
        exclude_practice: true,
        practice_buff: true,
        ..PatchPlan::default()
    };
    let (out, s) = transform(
        &t,
        &plan,
        &RatioVector {
            hp: 1.0,
            stat: [1.0; 12],
            crit_delta: 0.0,
            para_delta: 0.0,
        },
    );
    assert_eq!(out.hp[638], 100); // untouched, not row 0's buffed value
    assert_eq!(s.practice_rows, 0);
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
fn the_summary_counts_live_rows_by_non_zero_authored_hp() {
    let mut t = blank();
    let ratio = RatioVector {
        hp: 1.0,
        stat: [1.0; 12],
        crit_delta: 0.0,
        para_delta: 0.0,
    };

    let (_, all) = transform(&t, &PatchPlan::default(), &ratio);
    assert_eq!(all.live_rows, 649); // every blank row has HP, so every row is live

    t.hp[5] = 0; // a slot the disc never filled
    t.hp[644] = 0; // an *excluded* row: the live base is the authored table, not the handled rows
    let (_, s) = transform(&t, &PatchPlan::default(), &ratio);
    assert_eq!(s.live_rows, 649 - 2);
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

/// A table where every row is distinguishable, so a copy is visible.
fn indexed() -> EnemyTable {
    EnemyTable {
        hp: (0..649).collect(),
        stat: (0..649).map(|r| [r; 12]).collect(),
        crit: vec![2; 649],
        para: vec![2; 649],
        rarity: vec![0; 649],
    }
}

fn identity_ratio() -> RatioVector {
    RatioVector {
        hp: 1.0,
        stat: [1.0; 12],
        crit_delta: 0.0,
        para_delta: 0.0,
    }
}

#[test]
fn collapse_copies_each_types_top_row_over_its_other_rows() {
    let t = indexed();
    let plan = PatchPlan {
        collapse_to_top: true,
        ..PatchPlan::default()
    };
    let (out, s) = transform(&t, &plan, &identity_ratio());

    for group in ROW_GROUPS {
        if group.rows.len() == 1 {
            continue;
        }
        for &row in group.rows {
            if row == group.top || COLLAPSE_SKIP_ROWS.contains(&row) {
                continue;
            }
            assert_eq!(
                out.hp[row], out.hp[group.top],
                "{} row {row} HP",
                group.model
            );
            assert_eq!(
                out.stat[row], out.stat[group.top],
                "{} row {row} stats",
                group.model
            );
            assert_eq!(
                out.rarity[row], out.rarity[group.top],
                "{} row {row} rarity",
                group.model
            );
        }
    }
    assert_eq!(s.collapsed_rows, 524);
}

#[test]
fn collapse_is_off_by_default() {
    let t = indexed();
    // The practice buff is switched off too: it is a separate stage that
    // rewrites rows 638-643, and this test isolates the collapse.
    let plan = PatchPlan {
        practice_buff: false,
        ..PatchPlan::default()
    };
    let (out, s) = transform(&t, &plan, &identity_ratio());
    assert_eq!(s.collapsed_rows, 0);
    assert!(ROW_GROUPS
        .iter()
        .all(|g| g.rows.iter().all(|&r| out.hp[r] == t.hp[r])));
}

#[test]
fn collapse_leaves_the_practice_rows_and_the_tripwire_alone() {
    let t = indexed();
    // The exclusions are switched off so this pins the collapse's own skip rule,
    // not the scaling exclusions.
    let plan = PatchPlan {
        collapse_to_top: true,
        practice_buff: false,
        exclude_destructibles: false,
        exclude_tripwire: false,
        ..PatchPlan::default()
    };
    let (out, _) = transform(&t, &plan, &identity_ratio());

    for row in 638..=645 {
        assert_eq!(out.hp[row], t.hp[row], "row {row} must keep its own value");
        assert_eq!(
            out.stat[row], t.stat[row],
            "row {row} must keep its own stats"
        );
    }
    for row in 370..=378 {
        assert_eq!(out.hp[row], t.hp[row], "destructible {row}");
    }
    assert_eq!(out.hp[644], 644); // the tripwire is authored, not collapsed onto 95
}

#[test]
fn collapse_then_practice_buff_puts_the_lesson_rows_on_their_type_top() {
    let t = indexed();
    let plan = PatchPlan {
        collapse_to_top: true,
        practice_buff: true,
        exclude_tripwire: true,
        ..PatchPlan::default()
    };
    let (out, s) = transform(&t, &plan, &identity_ratio());

    // The buff copies from rows 0/1/33/34/12/13 *after* the collapse, so the
    // lesson rows end up carrying their type's top row — intended for a preset
    // called Brutal, and pinned here so it stays a decision.
    assert_eq!(out.hp[638], out.hp[393]);
    assert_eq!(out.hp[640], out.hp[41]);
    assert_eq!(out.hp[642], out.hp[637]);
    assert_eq!(out.hp[644], 644); // still the authored tripwire row
    assert_eq!(s.practice_rows, 6);
    assert_eq!(s.collapsed_rows, 524);
}

#[test]
fn brutal_is_extreme_plus_the_collapse_and_round_trips() {
    let plan = Preset::Brutal.plan();
    assert!(plan.force_very_hard && plan.collapse_to_top && plan.practice_buff);
    assert!(
        plan.exclude_destructibles && plan.exclude_tripwire && plan.exclude_practice,
        "Brutal leaves the training stage vanilla like Extreme"
    );
    assert_eq!(Preset::detect(&plan), Preset::Brutal);
    assert_eq!(Preset::Brutal.label(), "Very Hard Plus — Brutal");

    let edited = PatchPlan {
        collapse_to_top: false,
        ..plan
    };
    assert_eq!(Preset::detect(&edited), Preset::Extreme);
    assert_eq!(Preset::detect(&PatchPlan::default()), Preset::VeryHardPlus);
}
