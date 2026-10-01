use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;
use dw4vhp_core::patch::{patch_file_with_layout, PatchOptions};
use dw4vhp_core::plan::PatchPlan;
use dw4vhp_core::testkit;
use std::path::Path;

fn run(layout: &Layout, input: &Path, out: &Path, plan: &PatchPlan) {
    patch_file_with_layout(
        input,
        out,
        plan,
        &PatchOptions { overwrite: false },
        layout,
        &mut |_| {},
    )
    .unwrap();
}

#[test]
fn patches_a_clean_disc_and_reports_the_work_it_did() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let before = std::fs::read(&input).unwrap();
    let out = dir.path().join("out.iso");
    let o = patch_file_with_layout(
        &input,
        &out,
        &PatchPlan::default(),
        &PatchOptions { overwrite: false },
        &layout,
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(o.summary.rows_changed, 649 - 11);
    assert!(o.summary.practice_rows > 0);
    assert!(!o.summary.elf_step && !o.summary.serial_step);
    assert_eq!(
        std::fs::read(&input).unwrap(),
        before,
        "the input must be untouched"
    );
    assert!(testkit::changed_byte_count(&input, &out) > 0);
}

#[test]
fn the_elf_step_changes_exactly_four_bytes_and_only_when_enabled() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let default_out = dir.path().join("default.iso");
    run(&layout, &input, &default_out, &PatchPlan::default());
    let extreme = PatchPlan {
        force_very_hard: true,
        ..Default::default()
    };
    let extreme_out = dir.path().join("extreme.iso");
    run(&layout, &input, &extreme_out, &extreme);

    let elf = testkit::elf_region_offset(&input).unwrap();
    // Both outputs carry the table rewrite, so it cancels and the only changed
    // run between them is the ELF difficulty word.
    assert_eq!(
        testkit::changed_offsets(&default_out, &extreme_out),
        vec![elf]
    );
    let replacement = 0x2410_0001u32.to_le_bytes();
    assert_eq!(
        std::fs::read(&extreme_out).unwrap()[elf as usize..elf as usize + 4],
        replacement
    );
    assert_ne!(
        std::fs::read(&default_out).unwrap()[elf as usize..elf as usize + 4],
        replacement,
        "the default output must not carry the Extreme difficulty word"
    );
}

#[test]
fn the_serial_step_changes_only_the_two_intended_ranges() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let default_out = dir.path().join("default.iso");
    run(&layout, &input, &default_out, &PatchPlan::default());
    let serial_plan = PatchPlan {
        serial: Some("SLUS_000.00".into()),
        ..Default::default()
    };
    let serial_out = dir.path().join("serial.iso");
    run(&layout, &input, &serial_out, &serial_plan);

    let (a, b) = testkit::serial_offsets(&input).unwrap();
    // The two outputs differ by the serial step alone, so the table rewrite
    // cancels and every changed run must be one of the two serial ranges.
    let offsets = testkit::changed_offsets(&default_out, &serial_out);
    assert!(!offsets.is_empty(), "the serial step changed nothing");
    assert!(
        offsets
            .iter()
            .all(|o| (a..a + 13).contains(o) || (b..b + 11).contains(o)),
        "{offsets:?}"
    );
}

#[test]
fn paths_with_spaces_and_non_ascii_characters_work() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let sub = dir.path().join("Dígimon Wörld 4 — saves");
    std::fs::create_dir_all(&sub).unwrap();
    let out = sub.join("Very Hard Plus!.iso");
    run(&layout, &input, &out, &PatchPlan::default());
    assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn refuses_an_already_modded_disc_and_writes_nothing() {
    let (dir, input, layout) = testkit::modded_disc_tempfile();
    let out = dir.path().join("out.iso");
    let err = patch_file_with_layout(
        &input,
        &out,
        &PatchPlan::default(),
        &PatchOptions { overwrite: false },
        &layout,
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(err, Error::AlreadyModded { .. }));
    assert!(!out.exists());
}
