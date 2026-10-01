//! The committed row groups must partition the live rows exactly once, cover
//! the documented anchors, and leave the never-filled slots alone.
//!
//! These are the checks the ISO itself can answer. That the groups are the
//! game's real type partition is not one of them: it came from the runtime
//! `MODEL` column (spec §12.3) and is proved separately by
//! `rowgroup_snapshot.rs`.
use dw4vhp_core::rowgroup::{RowGroup, COLLAPSE_SKIP_ROWS, ROW_GROUPS};

fn group(model: &str) -> &'static RowGroup {
    ROW_GROUPS
        .iter()
        .find(|g| g.model == model)
        .unwrap_or_else(|| panic!("no group {model}"))
}

#[test]
fn the_groups_partition_the_live_rows_and_leave_the_empty_slots() {
    let mut owner = vec![None::<&str>; 649];
    for g in ROW_GROUPS {
        assert!(!g.rows.is_empty(), "{} has no rows", g.model);
        assert!(
            g.rows.contains(&g.top),
            "{}: top {} not in rows",
            g.model,
            g.top
        );
        assert!(
            g.rows.windows(2).all(|w| w[0] < w[1]),
            "{} rows are not ascending",
            g.model
        );
        for &row in g.rows {
            assert!(row < 649, "{}: row {row} out of range", g.model);
            if let Some(previous) = owner[row] {
                panic!("row {row} is claimed by both {previous} and {}", g.model);
            }
            owner[row] = Some(g.model);
        }
    }
    assert_eq!(owner.iter().filter(|o| o.is_some()).count(), 580);
    assert_eq!(owner.iter().filter(|o| o.is_none()).count(), 69);
}

#[test]
fn the_groups_are_sorted_by_first_row_with_unique_models() {
    assert!(ROW_GROUPS.windows(2).all(|w| w[0].rows[0] < w[1].rows[0]));
    let mut models: Vec<&str> = ROW_GROUPS.iter().map(|g| g.model).collect();
    let count = models.len();
    models.sort_unstable();
    models.dedup();
    assert_eq!(models.len(), count, "a model appears twice");
}

#[test]
fn the_documented_anchors_land_in_the_expected_groups() {
    for (model, start, end) in [
        ("e_goburi", 379, 393),
        ("e_mummy", 394, 408),
        ("e_otama", 409, 423),
        ("e_mecha2", 610, 624),
    ] {
        let g = group(model);
        assert!(
            (start..=end).all(|r| g.rows.contains(&r)),
            "{model} must own {start}-{end}"
        );
    }
    // The practice rows and the two special high-stat tops (spec §12.3).
    for (model, row) in [
        ("e_goburi", 0),
        ("e_goburi", 638),
        ("e_goburi", 639),
        ("e_goburi", 393),
        ("e_ogre", 33),
        ("e_ogre", 640),
        ("e_ogre", 641),
        ("e_ogre", 41),
        ("e_nume", 12),
        ("e_nume", 642),
        ("e_nume", 643),
        ("e_nume", 637),
        ("e_cockatri", 91),
        ("e_cockatri", 644),
        ("e_cockatri", 645),
        ("e_cockatri", 95),
        ("e_ldknight", 609),
        ("e_mecha4bs", 68),
        ("e_mecha4bs", 74),
    ] {
        assert!(
            group(model).rows.contains(&row),
            "{model} must own row {row}"
        );
    }
    assert_eq!(group("e_ldknight").top, 609);
    assert_eq!(group("e_nume").top, 637);
    assert_eq!(group("e_mecha4").top, 73);
}

#[test]
fn the_nine_destructibles_are_single_row_groups_at_370_to_378() {
    let props: Vec<&RowGroup> = ROW_GROUPS
        .iter()
        .filter(|g| g.model.starts_with("g_"))
        .collect();
    assert!(!props.is_empty());
    assert!(props.iter().all(|g| g.rows.len() == 1));
    let mut rows: Vec<usize> = props.iter().flat_map(|g| g.rows.iter().copied()).collect();
    rows.sort_unstable();
    assert_eq!(rows, (370..=378).collect::<Vec<_>>());
}

#[test]
fn the_collapse_skip_set_is_the_practice_range() {
    assert_eq!(COLLAPSE_SKIP_ROWS, 638..=645);
}
