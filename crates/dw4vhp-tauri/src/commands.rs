//! The IPC surface: every command the frontend can invoke, and the three
//! patch events it listens to. Engine calls never run on the main thread:
//! Tauri runs synchronous commands on the main thread, so [`analyze`],
//! [`plan_summary`] and [`start_patch`] are `async` and hand their work to
//! `spawn_blocking`. Progress and results come back through events.
//!
//! The only logic here beyond calling the engine is the status-kind mapping
//! and the output-name rule, both transcribed from the old egui app's
//! `form.rs` so the refusals and default output keep their exact wording.

use crate::state::AppState;
use dw4vhp_core::disc::{inspect, DiscReport};
use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;
use dw4vhp_core::patch::{patch_file_with_layout, PatchOptions};
use dw4vhp_core::plan::{transform, PatchPlan, PlanSummary};
use dw4vhp_core::ratio::RatioVector;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// The marker a patched image's name carries (old `form::OUTPUT_SUFFIX`).
const OUTPUT_SUFFIX: &str = " [VeryHardPlus].iso";

/// The wire form of a patch plan: camelCase over the engine's `PatchPlan`.
/// A dedicated payload type because the engine type carries no `Deserialize`
/// (and the engine crate stays serde-free by design).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanPayload {
    hp_mult: f64,
    stat_mult: Vec<f64>,
    crit_mult: f64,
    para_mult: f64,
    crown_rank: u8,
    exclude_destructibles: bool,
    exclude_tripwire: bool,
    exclude_practice: bool,
    practice_buff: bool,
    collapse_to_top: bool,
    force_very_hard: bool,
    serial: Option<String>,
}

impl PlanPayload {
    fn into_plan(self) -> Result<PatchPlan, String> {
        Ok(PatchPlan {
            hp_mult: self.hp_mult,
            stat_mult: {
                let mut stat_mult = [0.0; 12];
                let values: [f64; 12] = self
                    .stat_mult
                    .try_into()
                    .map_err(|_| "stat_mult must carry exactly 12 values".to_string())?;
                stat_mult.copy_from_slice(&values);
                stat_mult
            },
            crit_mult: self.crit_mult,
            para_mult: self.para_mult,
            crown_rank: self.crown_rank,
            exclude_destructibles: self.exclude_destructibles,
            exclude_tripwire: self.exclude_tripwire,
            exclude_practice: self.exclude_practice,
            practice_buff: self.practice_buff,
            collapse_to_top: self.collapse_to_top,
            force_very_hard: self.force_very_hard,
            serial: self.serial,
        })
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiscStatusPayload {
    kind: &'static str,
    text: String,
    size: u64,
    copies: Option<[usize; 3]>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummaryPayload {
    rows_changed: usize,
    live_rows: usize,
    attack_pinned: usize,
    hp_capped: usize,
    crit_pinned: usize,
    practice_rows: usize,
    collapsed_rows: usize,
    elf_step: bool,
    serial_step: bool,
}

impl PlanSummaryPayload {
    fn of(summary: &PlanSummary) -> Self {
        Self {
            rows_changed: summary.rows_changed,
            live_rows: summary.live_rows,
            attack_pinned: summary.attack_pinned,
            hp_capped: summary.hp_capped,
            crit_pinned: summary.crit_pinned,
            practice_rows: summary.practice_rows,
            collapsed_rows: summary.collapsed_rows,
            elf_step: summary.elf_step,
            serial_step: summary.serial_step,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PatchDonePayload {
    md5: String,
    bytes: u64,
    summary: PlanSummaryPayload,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    phase: &'static str,
    done: u64,
    total: u64,
}

/// The native open dialog for the player's ISO. `None` when cancelled.
#[tauri::command]
pub fn choose_iso(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .add_filter("PS2 disc image", &["iso"])
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
}

/// The native save dialog for the patched output. `None` when cancelled.
#[tauri::command]
pub fn choose_output(app: AppHandle, default_name: String) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .set_file_name(&default_name)
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
}

/// The output name an input suggests: same directory, stem plus the marker
/// (old `form::default_output_path`), so the default never targets the input.
#[tauri::command]
pub fn default_output(input: String) -> String {
    let input = std::path::Path::new(&input);
    let mut name = input
        .file_stem()
        .map(|s| s.to_os_string())
        .unwrap_or_default();
    name.push(OUTPUT_SUFFIX);
    input.with_file_name(name).display().to_string()
}

/// The verdict for one inspection result: a clean disc and its authored copy
/// counts, or a refusal that reuses the engine's own fix-naming message. The
/// kind mapping is the old `form::status_for` transcription.
fn verdict(result: &Result<DiscReport, Error>) -> DiscStatusPayload {
    match result {
        Ok(report) => DiscStatusPayload {
            kind: "ok",
            text: format!(
                "clean Digimon World 4 (USA) SLUS_208.36: {} HPMAX, {} stats and {} chargen \
                 copies",
                report.copies[0], report.copies[1], report.copies[2]
            ),
            size: report.size,
            copies: Some(report.copies),
        },
        Err(error) => DiscStatusPayload {
            kind: match error {
                Error::AlreadyModded { .. } | Error::WrongRevision { .. }
                | Error::NotThisDisc => "refused",
                _ => "unknown",
            },
            text: error.to_string(),
            size: 0,
            copies: None,
        },
    }
}

/// Inspects the disc off the main thread, stores a clean report for later
/// [`plan_summary`] and [`start_patch`] calls, and returns the verdict.
#[tauri::command]
pub async fn analyze(
    path: String,
    app: AppHandle,
) -> Result<DiscStatusPayload, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        inspect(&std::path::PathBuf::from(&path), &Layout::retail())
    })
    .await
    .map_err(|e| format!("analyze worker failed: {e}"))?;

    let payload = verdict(&result);
    if let Ok(report) = result {
        let state = app.state::<AppState>();
        *state.input.lock().expect("input lock") = Some(std::path::PathBuf::from(path));
        *state.report.lock().expect("report lock") = Some(report);
    }
    Ok(payload)
}

/// The pure `transform` summary for the live plan panel. The frontend calls
/// this on every plan edit; it reads the disc table stored by [`analyze`].
#[tauri::command]
pub async fn plan_summary(
    plan: PlanPayload,
    app: AppHandle,
) -> Result<PlanSummaryPayload, String> {
    let plan = plan.into_plan()?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let report = state.report.lock().expect("report lock");
        let report = report.as_ref().ok_or_else(|| {
            "choose a disc first — the summary needs the analysed enemy table".to_string()
        })?;
        let ratio = RatioVector::derive(&report.authored);
        let (_, summary) = transform(&report.authored, &plan, &ratio);
        Ok(PlanSummaryPayload::of(&summary))
    })
    .await
    .map_err(|e| format!("summary worker failed: {e}"))?
}

/// Runs the whole patch on a blocking thread: copy, write regions, verify,
/// atomic rename. Progress streams as `patch-progress` events; exactly one
/// terminal event follows — `patch-done` with the verified outcome, or
/// `patch-failed` with the engine's fix-naming message.
///
/// Returns `Err` synchronously only when no disc has been analysed yet.
#[tauri::command]
pub async fn start_patch(
    plan: PlanPayload,
    output: String,
    overwrite: bool,
    app: AppHandle,
) -> Result<(), String> {
    let plan = plan.into_plan()?;
    let input = {
        let state = app.state::<AppState>();
        let stored = state.input.lock().expect("input lock");
        stored.clone().ok_or_else(|| {
            "choose a disc first — nothing to patch until a disc is analysed".to_string()
        })?
    };
    let output = std::path::PathBuf::from(output);
    let opts = PatchOptions { overwrite };

    tauri::async_runtime::spawn_blocking(move || {
        let mut progress = |p: dw4vhp_core::writer::Progress| {
            // The window may be closed mid-patch; a dropped listener is not
            // worth stopping the write for.
            let _ = app.emit(
                "patch-progress",
                ProgressPayload {
                    phase: phase_name(p.phase),
                    done: p.done,
                    total: p.total,
                },
            );
        };
        let outcome =
            patch_file_with_layout(&input, &output, &plan, &opts, &Layout::retail(), &mut progress);
        match outcome {
            Ok(outcome) => {
                let _ = app.emit(
                    "patch-done",
                    PatchDonePayload {
                        md5: outcome.md5,
                        bytes: outcome.bytes,
                        summary: PlanSummaryPayload::of(&outcome.summary),
                    },
                );
            }
            Err(error) => {
                let _ = app.emit("patch-failed", error.to_string());
            }
        }
    })
    .await
    .map_err(|e| format!("patch worker failed: {e}"))?;
    Ok(())
}

fn phase_name(phase: dw4vhp_core::writer::Phase) -> &'static str {
    match phase {
        dw4vhp_core::writer::Phase::Inspecting => "inspecting",
        dw4vhp_core::writer::Phase::Copying => "copying",
        dw4vhp_core::writer::Phase::Writing => "writing",
        dw4vhp_core::writer::Phase::Verifying => "verifying",
    }
}
