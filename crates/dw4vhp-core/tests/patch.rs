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
    let plan = PatchPlan {
        force_very_hard: true,
        ..Default::default()
    };
    let out = dir.path().join("extreme.iso");
    run(&layout, &input, &out, &plan);
    let elf = testkit::elf_region_offset(&input).unwrap();
    assert_eq!(testkit::changed_offsets(&input, &out), vec![elf]);
    assert_eq!(
        std::fs::read(&out).unwrap()[elf as usize..elf as usize + 4],
        0x2410_0001u32.to_le_bytes()
    );
}

#[test]
fn the_serial_step_changes_only_the_two_intended_ranges() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let plan = PatchPlan {
        serial: Some("SLUS_000.00".into()),
        ..Default::default()
    };
    let out = dir.path().join("serial.iso");
    run(&layout, &input, &out, &plan);
    let (a, b) = testkit::serial_offsets(&input).unwrap();
    let offsets = testkit::changed_offsets(&input, &out);
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
