// crates/dw4vhp-gui/tests/worker.rs
use dw4vhp_core::plan::PatchPlan;
use dw4vhp_core::testkit;
use dw4vhp_gui::worker::{spawn_patch_with_layout, WorkerMsg};

#[test]
fn the_worker_reports_progress_then_a_terminal_message() {
    let (dir, input, layout) = testkit::clean_disc_tempfile();
    let out = dir.path().join("out.iso");
    let w = spawn_patch_with_layout(input, out, PatchPlan::default(), false, layout);
    let mut saw_terminal = false;
    for msg in w.rx.iter() {
        match msg {
            WorkerMsg::Progress(_) => {}
            WorkerMsg::Done(_) => saw_terminal = true,
            WorkerMsg::Failed(e) => panic!("unexpected failure: {e}"),
        }
    }
    assert!(saw_terminal);
    w.handle.join().unwrap();
}

#[test]
fn the_worker_reports_a_refusal_as_failed_not_as_a_panic() {
    let (dir, input, layout) = testkit::modded_disc_tempfile();
    let out = dir.path().join("out.iso");
    let w = spawn_patch_with_layout(input, out, PatchPlan::default(), false, layout);
    let mut failed = None;
    for msg in w.rx.iter() {
        if let WorkerMsg::Failed(e) = msg {
            failed = Some(e);
        }
    }
    assert!(failed.unwrap().contains("original"));
    w.handle.join().unwrap();
}
