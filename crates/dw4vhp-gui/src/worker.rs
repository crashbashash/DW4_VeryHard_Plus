//! The background patch worker.
//!
//! The window never calls the engine on its own thread: it hands the input,
//! output and plan to [`spawn_patch`], and the worker owns the patch call, the
//! progress callback and the single terminal message that ends the run. A
//! refusal is a [`WorkerMsg::Failed`] carrying the engine's own fix-naming
//! message — never a panic, never silence.

use dw4vhp_core::layout::Layout;
use dw4vhp_core::patch::{patch_file_with_layout, PatchOptions, PatchOutcome};
use dw4vhp_core::plan::PatchPlan;
use dw4vhp_core::writer::Progress;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;

/// One message from the worker to the window: a progress report, the verified
/// outcome, or a refusal's message. Exactly one terminal message — [`Done`] or
/// [`Failed`] — is sent, after every [`Progress`].
///
/// [`Done`]: WorkerMsg::Done
/// [`Failed`]: WorkerMsg::Failed
/// [`Progress`]: WorkerMsg::Progress
pub enum WorkerMsg {
    Progress(Progress),
    Done(Box<PatchOutcome>),
    Failed(String),
}

/// A running patch worker: its message channel and its thread handle.
pub struct WorkerHandle {
    pub rx: Receiver<WorkerMsg>,
    pub handle: JoinHandle<()>,
}

/// The app's entry point: patch `input` into `output` against the retail
/// [`Layout::retail`] layout on a background thread.
pub fn spawn_patch(
    input: PathBuf,
    output: PathBuf,
    plan: PatchPlan,
    overwrite: bool,
) -> WorkerHandle {
    spawn_patch_with_layout(input, output, plan, overwrite, Layout::retail())
}

/// The layout-parameterised seam the worker tests use. `spawn_patch` delegates
/// here so every caller takes the same inspect → plan → write → verify path.
pub fn spawn_patch_with_layout(
    input: PathBuf,
    output: PathBuf,
    plan: PatchPlan,
    overwrite: bool,
    layout: Layout,
) -> WorkerHandle {
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let opts = PatchOptions { overwrite };
        let mut progress = |p: Progress| {
            // The window may have gone away; a dropped receiver is not a
            // failure worth stopping the patch for.
            let _ = tx.send(WorkerMsg::Progress(p));
        };
        let outcome = patch_file_with_layout(&input, &output, &plan, &opts, &layout, &mut progress);
        let terminal = match outcome {
            Ok(outcome) => WorkerMsg::Done(Box::new(outcome)),
            Err(error) => WorkerMsg::Failed(error.to_string()),
        };
        let _ = tx.send(terminal);
    });
    WorkerHandle { rx, handle }
}
