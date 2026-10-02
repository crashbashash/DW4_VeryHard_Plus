// crates/dw4vhp-gui/tests/form.rs
use dw4vhp_core::error::Error;
use dw4vhp_core::plan::{PlanSummary, Preset};
use dw4vhp_core::serial::DEFAULT_SERIAL;
use dw4vhp_gui::form::{
    attack_pin_note, collapse_note, default_output_path, status_for, StatusKind, UiState,
};
use std::path::{Path, PathBuf};

#[test]
fn the_default_output_name_keeps_the_directory_and_marks_the_suffix() {
    let p = default_output_path(Path::new("/games/Digimon World 4 (USA).iso"));
    assert_eq!(
        p,
        PathBuf::from("/games/Digimon World 4 (USA) [VeryHardPlus].iso")
    );
}

#[test]
fn editing_any_knob_switches_the_preset_to_custom() {
    let mut s = UiState {
        preset: Preset::VeryHardPlus,
        ..Default::default()
    };
    s.plan.crown_rank = 3;
    s.on_plan_edited();
    assert_eq!(s.preset, Preset::Custom);
}

#[test]
fn selecting_a_preset_loads_its_plan_into_the_advanced_panel() {
    let mut s = UiState::default();
    // Start somewhere far from any preset.
    s.plan.crown_rank = 0;
    s.plan.collapse_to_top = true;
    s.on_plan_edited();
    assert_eq!(s.preset, Preset::Custom);

    let mut serial_enabled = true;
    let mut serial_text = "SLUS_999.99".to_string();
    s.apply_preset(Preset::Brutal, &mut serial_enabled, &mut serial_text);
    assert_eq!(s.plan, Preset::Brutal.plan());
    assert_eq!(s.preset, Preset::Brutal);
    assert!(!serial_enabled);
    assert_eq!(serial_text, DEFAULT_SERIAL.to_string());

    // Custom keeps whatever plan is on screen — it has none of its own.
    let kept = s.plan.clone();
    s.apply_preset(Preset::Custom, &mut serial_enabled, &mut serial_text);
    assert_eq!(s.plan, kept);
}

#[test]
fn an_already_modded_disc_is_refused_with_the_fix() {
    let l = status_for(&Err(Error::AlreadyModded {
        rarity_nonzero: 638,
    }));
    assert_eq!(l.kind, StatusKind::Refused);
    assert!(l.text.contains("original"), "{}", l.text);
}

#[test]
fn a_wrong_revision_is_refused_and_names_the_supported_release() {
    let l = status_for(&Err(Error::WrongRevision {
        block: "HPMAX",
        found: 1,
        expected: 665,
    }));
    assert_eq!(l.kind, StatusKind::Refused);
    assert!(l.text.contains("SLUS_208.36"), "{}", l.text);
}

#[test]
fn the_attack_note_names_the_pinned_count_and_the_live_base() {
    let s = PlanSummary {
        attack_pinned: 337,
        live_rows: 580,
        ..Default::default()
    };
    assert_eq!(
        attack_pin_note(&s).unwrap(),
        "attack: 337 of 580 live rows pinned at 32767"
    );
}

#[test]
fn the_attack_note_is_absent_when_nothing_is_pinned() {
    let s = PlanSummary {
        attack_pinned: 0,
        live_rows: 580,
        ..Default::default()
    };
    assert!(attack_pin_note(&s).is_none());
}

#[test]
fn the_collapse_note_counts_the_rows_and_says_what_they_carry() {
    let s = PlanSummary {
        collapsed_rows: 524,
        ..Default::default()
    };
    assert_eq!(
        collapse_note(&s).unwrap(),
        "collapse: 524 rows now carry their type's strongest row — every enemy of a type is \
         identical, EXP included"
    );
}

#[test]
fn the_collapse_note_is_absent_when_nothing_is_collapsed() {
    let s = PlanSummary {
        collapsed_rows: 0,
        ..Default::default()
    };
    assert!(collapse_note(&s).is_none());
}
